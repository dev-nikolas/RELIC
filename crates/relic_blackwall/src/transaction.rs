//! Paso 3 del Blackwall: Compuerta Transaccional (`Propose -> Verify -> Regression -> Commit/Rollback/Consolidate`).

use std::path::Path;
use relic_cli::{answer_intent, parse_dispatch_intent_with_lexicon, Answer, DispatchIntent};
use relic_core::{propagate_activation, SpreadingActivationResult};
use relic_lexicon::{
    compute_consolidation_candidate, EngramStore, EntryStatus, IntegrityError, LexiconPayload,
    LexiconView, LogEntry, VersionDiff,
};
use serde::{Deserialize, Serialize};
use crate::oracle::validate_proposal_oracle;
use crate::regression::{compute_consolidation_fidelity, run_regression_suite, RegressionCase};

/// Resultado determinista y auditable de una transacción sobre el Blackwall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransactionOutcome {
    Committed {
        version: u64,
        entry_hash: String,
    },
    RolledBack {
        version: u64,
        target_version: u64,
        entry_hash: String,
    },
    Consolidated {
        version: u64,
        entry_hash: String,
        pruned_count: usize,
        fidelity_score: f64,
    },
    RejectedByOracle {
        reason: String,
    },
    RejectedByRegression {
        failed_case: String,
        reason: String,
    },
    RejectedByFidelity {
        fidelity_score: f64,
        required_fidelity: f64,
        degraded_cases: Vec<String>,
    },
    RejectedByIntegrity {
        reason: String,
    },
}

/// Compuerta única de mutación del estado cognitivo y léxico de RELIC.
/// Prohibido mutar `EngramStore` fuera de `propose_mutation`, `rollback_to` o `consolidate_memory`.
#[derive(Debug, Clone)]
pub struct BlackwallGate {
    engram: EngramStore,
    custom_regression_cases: Vec<RegressionCase>,
}

impl Default for BlackwallGate {
    fn default() -> Self {
        Self::with_genesis()
    }
}

impl BlackwallGate {
    pub fn new(engram: EngramStore) -> Self {
        Self {
            engram,
            custom_regression_cases: Vec::new(),
        }
    }

    pub fn with_genesis() -> Self {
        Self::new(EngramStore::with_genesis())
    }

    pub fn open_or_init_file(path: impl AsRef<Path>) -> Result<Self, IntegrityError> {
        let store = EngramStore::open_or_init_file(path)?;
        Ok(Self::new(store))
    }

    pub fn add_regression_case(&mut self, case: RegressionCase) {
        self.custom_regression_cases.push(case);
    }

    /// Ejecuta el ciclo estricto: Oráculo -> Clonación Efímera en Sandbox -> Regresión -> Commit/Reject.
    pub fn propose_mutation(
        &mut self,
        payload: LexiconPayload,
        timestamp_ms: u64,
    ) -> TransactionOutcome {
        if let Err(violation) = validate_proposal_oracle(&payload, self.engram.active_view()) {
            return TransactionOutcome::RejectedByOracle {
                reason: violation.to_string(),
            };
        }

        let status = match &payload {
            LexiconPayload::GraphFact { .. } | LexiconPayload::EdgeReinforcement { .. } => {
                EntryStatus::ConfirmedFact
            }
            LexiconPayload::OpAlias { .. }
            | LexiconPayload::PhraseMacro { .. }
            | LexiconPayload::EntityAlias { .. } => EntryStatus::LexicalAlias,
            LexiconPayload::ConsolidationCheckpoint { .. } => EntryStatus::ConsolidatedCheckpoint,
            LexiconPayload::Rollback { .. } => EntryStatus::Reverted,
        };

        let mut sandbox = self.engram.clone();
        sandbox.detach_backing_file();
        if let Err(err) = sandbox.append_entry(timestamp_ms, status, payload.clone()) {
            return TransactionOutcome::RejectedByIntegrity {
                reason: err.to_string(),
            };
        }

        if let Err(reg_fail) =
            run_regression_suite(sandbox.active_view_mut(), &self.custom_regression_cases)
        {
            return TransactionOutcome::RejectedByRegression {
                failed_case: reg_fail.case_id,
                reason: reg_fail.reason,
            };
        }

        match self.engram.append_entry(timestamp_ms, status, payload) {
            Ok(entry) => TransactionOutcome::Committed {
                version: entry.version,
                entry_hash: entry.entry_hash,
            },
            Err(err) => TransactionOutcome::RejectedByIntegrity {
                reason: err.to_string(),
            },
        }
    }

    /// Refuerza el peso asociativo de una arista `from -> to` sumando `delta` y registrando el evento en la bitácora.
    pub fn reinforce_edge(
        &mut self,
        from: &str,
        to: &str,
        delta: f64,
        timestamp_ms: u64,
    ) -> TransactionOutcome {
        let Some((_, current_w, current_u)) = self
            .engram
            .active_view()
            .active_facts
            .get(&(from.to_string(), to.to_string()))
            .cloned()
        else {
            return TransactionOutcome::RejectedByOracle {
                reason: format!("La arista '{from} -> {to}' no existe en el estado activo"),
            };
        };
        let new_weight = (current_w + delta).max(0.0);
        let payload = LexiconPayload::EdgeReinforcement {
            from: from.to_string(),
            to: to.to_string(),
            new_weight,
            usage_count: current_u + 1,
        };
        self.propose_mutation(payload, timestamp_ms)
    }

    /// Ejecuta la Consolidación ("Sueño"): poda episodios efímeros con `weight < min_weight_threshold`,
    /// calcula la Fidelidad de Consolidación $F \in [0.0, 1.0]$ y aborta si $F < min\_fidelity$.
    pub fn consolidate_memory(
        &mut self,
        min_weight_threshold: f64,
        min_fidelity: f64,
        timestamp_ms: u64,
    ) -> TransactionOutcome {
        let (pruned_edges, mut pruned_view) =
            compute_consolidation_candidate(&self.engram, min_weight_threshold);
        let mut before_view = self.engram.active_view().clone();

        let (fidelity_score, degraded_cases) = compute_consolidation_fidelity(
            &mut before_view,
            &mut pruned_view,
            &self.custom_regression_cases,
        );

        if fidelity_score < min_fidelity || !degraded_cases.is_empty() {
            return TransactionOutcome::RejectedByFidelity {
                fidelity_score,
                required_fidelity: min_fidelity,
                degraded_cases,
            };
        }

        let pruned_count = pruned_edges.len();
        let payload = LexiconPayload::ConsolidationCheckpoint {
            min_weight_threshold,
            pruned_edges,
            fidelity_score,
        };

        match self.engram.append_entry(
            timestamp_ms,
            EntryStatus::ConsolidatedCheckpoint,
            payload,
        ) {
            Ok(entry) => TransactionOutcome::Consolidated {
                version: entry.version,
                entry_hash: entry.entry_hash,
                pruned_count,
                fidelity_score,
            },
            Err(err) => TransactionOutcome::RejectedByIntegrity {
                reason: err.to_string(),
            },
        }
    }

    /// Ejecuta un rollback append-only con cicatriz hacia `target_version` tras validar regresión.
    pub fn rollback_to(
        &mut self,
        target_version: u64,
        reason: impl Into<String>,
        timestamp_ms: u64,
    ) -> TransactionOutcome {
        let reason_str = reason.into();
        if target_version >= self.engram.current_version() {
            return TransactionOutcome::RejectedByIntegrity {
                reason: format!(
                    "target_version ({target_version}) debe ser menor a la versión actual ({})",
                    self.engram.current_version()
                ),
            };
        }

        let payload = LexiconPayload::Rollback {
            target_version,
            reason: reason_str,
        };

        let mut sandbox = self.engram.clone();
        sandbox.detach_backing_file();
        if let Err(err) = sandbox.append_entry(timestamp_ms, EntryStatus::Reverted, payload.clone())
        {
            return TransactionOutcome::RejectedByIntegrity {
                reason: err.to_string(),
            };
        }

        if let Err(reg_fail) =
            run_regression_suite(sandbox.active_view_mut(), &self.custom_regression_cases)
        {
            return TransactionOutcome::RejectedByRegression {
                failed_case: reg_fail.case_id,
                reason: reg_fail.reason,
            };
        }

        match self.engram.append_entry(timestamp_ms, EntryStatus::Reverted, payload) {
            Ok(entry) => TransactionOutcome::RolledBack {
                version: entry.version,
                target_version,
                entry_hash: entry.entry_hash,
            },
            Err(err) => TransactionOutcome::RejectedByIntegrity {
                reason: err.to_string(),
            },
        }
    }

    /// Evalúa una consulta en lenguaje natural contra la vista verificada activa.
    pub fn evaluate_query(&mut self, query: &str) -> (DispatchIntent, Answer) {
        let intent = parse_dispatch_intent_with_lexicon(query, Some(self.engram.active_view()));
        let answer = answer_intent(&intent, &mut self.engram.active_view_mut().episodic);
        (intent, answer)
    }

    /// Ejecuta activación propagada (`propagate_activation`) sobre la memoria episódica activa.
    pub fn propagate_activation(
        &self,
        seeds: &[&str],
        gamma: f64,
        steps: usize,
        max_entropy_threshold: f64,
    ) -> SpreadingActivationResult {
        propagate_activation(
            &self.engram.active_view().episodic,
            seeds,
            gamma,
            steps,
            max_entropy_threshold,
        )
    }

    /// Calcula el diff estructurado entre dos versiones del Engrama custodiado.
    pub fn diff_versions(&self, v_from: u64, v_to: u64) -> Result<VersionDiff, IntegrityError> {
        self.engram.diff_versions(v_from, v_to)
    }

    pub fn active_view(&self) -> &LexiconView { self.engram.active_view() }

    pub fn retrieve_neighbors(&mut self, entity: &str) -> Vec<String> {
        self.engram
            .active_view_mut()
            .episodic
            .get_neighbors(entity)
            .to_vec()
    }

    pub fn current_version(&self) -> u64 { self.engram.current_version() }
    pub fn head_hash(&self) -> &str { self.engram.head_hash() }
    pub fn log_entries(&self) -> &[LogEntry] { self.engram.entries() }
    pub fn to_jsonl_string(&self) -> String { self.engram.to_jsonl_string() }
}
