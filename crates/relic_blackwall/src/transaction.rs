//! Paso 3 del Blackwall: Compuerta Transaccional (`Propose -> Verify -> Regression -> Commit/Rollback/Consolidate`).
//! Menos de 180 líneas.

use std::path::Path;
use relic_cli::{answer_intent, parse_dispatch_intent_with_lexicon, Answer, DispatchIntent};
use relic_core::{
    analyze_spectrum_donoho_stark, propagate_activation, verify_network_alert, ArithmeticOp,
    CollapseDecision, NetworkAlertSpec, SpectralOutcome, SpreadingActivationResult, VerifiedAlertReport,
};
use relic_lexicon::{
    compute_consolidation_candidate, ClassicalPolysemyResult, DurabilityMode, EngramStore, EntryStatus,
    IntegrityError, InverseOp, LexiconPayload, LexiconView, LogEntry, VersionDiff,
};
use serde::{Deserialize, Serialize};
use crate::causal_index::CausalDependencyIndex;
use crate::oracle::validate_proposal_oracle;
use crate::regression::{compute_consolidation_fidelity, run_regression_suite, RegressionCase};

struct UndoGuard<'a> { view: &'a mut LexiconView, ops: Vec<InverseOp>, committed: bool }
impl<'a> UndoGuard<'a> { fn commit(mut self) { self.committed = true; } }
impl<'a> Drop for UndoGuard<'a> {
    fn drop(&mut self) {
        if !self.committed { for op in self.ops.drain(..).rev() { self.view.apply_inverse_op(op); } }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransactionOutcome {
    Committed { version: u64, entry_hash: String },
    RolledBack { version: u64, target_version: u64, entry_hash: String },
    Consolidated { version: u64, entry_hash: String, pruned_count: usize, fidelity_score: f64 },
    RejectedByOracle { reason: String },
    RejectedByRegression { failed_case: String, reason: String },
    RejectedByFidelity { fidelity_score: f64, required_fidelity: f64, degraded_cases: Vec<String> },
    RejectedByIntegrity { reason: String },
    RejectedByAnomaly { channel_id: u32, reason: String },
}

#[derive(Debug, Clone)]
pub struct BlackwallGate {
    engram: EngramStore,
    custom_regression_cases: Vec<RegressionCase>,
    causal_index: Option<CausalDependencyIndex>,
    pub enable_catapult: bool,
    pub shadow_mode: bool,
    pub durability: DurabilityMode,
    pub telemetry_arena: crate::telemetry_arena::TelemetryArena,
}

impl Default for BlackwallGate { fn default() -> Self { Self::with_genesis() } }

impl BlackwallGate {
    pub fn new(engram: EngramStore) -> Self {
        Self {
            engram,
            custom_regression_cases: Vec::new(),
            causal_index: None,
            enable_catapult: false, // Por defecto desactivada (contención de riesgo)
            shadow_mode: false,
            durability: DurabilityMode::Group,
            telemetry_arena: crate::telemetry_arena::TelemetryArena::new(
                crate::telemetry_arena::AnomalyGateMode::PassiveMonitor,
            )
            .expect("Error al inicializar TelemetryArena con pesos Seis Ojos"),
        }
    }
    pub fn with_genesis() -> Self { Self::new(EngramStore::with_genesis()) }
    pub fn open_or_init_file(p: impl AsRef<Path>) -> Result<Self, IntegrityError> { Ok(Self::new(EngramStore::open_or_init_file(p)?)) }
    pub fn add_regression_case(&mut self, case: RegressionCase) { self.custom_regression_cases.push(case); self.causal_index = None; }
    pub fn invalidate_causal_index(&mut self) { self.causal_index = None; }
    pub fn validate_causal_regression(&mut self, payload: &LexiconPayload) -> Result<usize, crate::regression::RegressionFailure> {
        if self.causal_index.is_none() {
            self.causal_index = Some(CausalDependencyIndex::build(self.engram.active_view_mut(), &self.custom_regression_cases));
        }
        let affected = self.causal_index.as_ref().unwrap().find_affected_tests_mode(payload, self.engram.active_view(), self.enable_catapult);
        let c_idx = self.causal_index.as_ref().unwrap();
        let reg_cases = &self.custom_regression_cases;
        c_idx.run_selective(self.engram.active_view_mut(), reg_cases, &affected)
    }

    pub fn propose_mutation(&mut self, payload: LexiconPayload, ts: u64) -> TransactionOutcome {
        self.commit_batch_payloads(std::slice::from_ref(&payload), ts)
    }

    pub fn commit_batch_payloads(&mut self, payloads: &[LexiconPayload], ts: u64) -> TransactionOutcome {
        match self.commit_transaction_with_telemetry(payloads, &[], ts) {
            Ok(outcome) => outcome,
            Err(err) => TransactionOutcome::RejectedByIntegrity { reason: err.to_string() },
        }
    }

    /// Ejecuta una transacción completa que incluye mutaciones simbólicas del léxico y/o telemetría continua.
    ///
    /// Semántica de commit y rollback:
    /// 1. Si la arena está en modo Enforcing y algún canal reporta anomalía (range_violation, drift_alarm o score_relic >= 0.990),
    ///    la transacción es rechazada SIN mutar ni el estado del detector ni el grafo episódico.
    /// 2. Si las mutaciones simbólicas son rechazadas por el oráculo o por regresión, `UndoGuard` revierte la memoria
    ///    y la telemetría NO se compromete.
    /// 3. Solo cuando el WAL (`engram.jsonl`) se persiste con éxito, se llama a `telemetry_arena.commit_batch`.
    /// 4. En modo `PassiveMonitor`, los veredictos de telemetría van a un log de auditoría fuera de banda y NUNCA
    ///    ensucian `engram.jsonl`.
    pub fn commit_transaction_with_telemetry(
        &mut self,
        payloads: &[LexiconPayload],
        telemetry: &[(u32, f64)],
        ts: u64,
    ) -> Result<TransactionOutcome, crate::telemetry_arena::BlackwallError> {
        if payloads.is_empty() && telemetry.is_empty() {
            return Ok(TransactionOutcome::RejectedByIntegrity {
                reason: "Lote vacío (ni mutaciones ni telemetría).".to_string(),
            });
        }

        // 1. Evaluación pura de la telemetría (falla si hay contención de canales o arena agotada)
        let assessments = self.telemetry_arena.evaluate_batch_pure(telemetry)?;

        // 2. Si está en Enforcing, auditar si hay alguna violación
        if self.telemetry_arena.mode == crate::telemetry_arena::AnomalyGateMode::Enforcing {
            for &(ch_id, _, ref assess) in &assessments {
                if assess.range_violation || assess.drift_alarm || assess.score_relic >= 0.990 {
                    return Ok(TransactionOutcome::RejectedByAnomaly {
                        channel_id: ch_id,
                        reason: format!(
                            "Violación en canal {ch_id}: score_relic={:.4}, range_violation={}, drift_alarm={}",
                            assess.score_relic, assess.range_violation, assess.drift_alarm
                        ),
                    });
                }
            }
        }

        // 3. Si no hay mutaciones simbólicas (solo telemetría continua), comprometer telemetría directamente
        if payloads.is_empty() {
            self.telemetry_arena.commit_batch(&assessments, ts);
            return Ok(TransactionOutcome::Committed {
                version: self.engram.current_version(),
                entry_hash: self.engram.head_hash().to_string(),
            });
        }

        // 4. Si hay mutaciones simbólicas, aplicar en UndoGuard y verificar
        let mut guard = UndoGuard {
            view: self.engram.active_view_mut(),
            ops: Vec::with_capacity(payloads.len()),
            committed: false,
        };
        let reg_cases = &self.custom_regression_cases;
        for payload in payloads {
            guard.ops.push(guard.view.apply_payload_recording_undo(payload));
            if let Err(v) = validate_proposal_oracle(payload, guard.view) {
                // guard se descarta y revierte la vista. La telemetría NO se compromete.
                return Ok(TransactionOutcome::RejectedByOracle { reason: v.to_string() });
            }
        }

        if self.causal_index.is_none() {
            self.causal_index = Some(CausalDependencyIndex::build(guard.view, reg_cases));
        }
        let c_idx = self.causal_index.as_ref().unwrap();
        let mut affected = Vec::new();
        let mut touched_nodes = std::collections::HashSet::new();
        for payload in payloads {
            match payload {
                LexiconPayload::GraphFact { from, to, replaces, .. } => {
                    let mut intra_batch_hazard = replaces.is_some();
                    if let (Some(u), Some(v)) = (
                        guard.view.episodic.interner.get_id(from),
                        guard.view.episodic.interner.get_id(to),
                    ) {
                        if touched_nodes.contains(&u) || touched_nodes.contains(&v) {
                            intra_batch_hazard = true;
                        }
                        touched_nodes.insert(u);
                        touched_nodes.insert(v);
                    }
                    if intra_batch_hazard {
                        for i in 0..c_idx.total_indexed {
                            if !c_idx.independent_tests.contains(&i) {
                                affected.push(i);
                            }
                        }
                    } else {
                        affected.extend(c_idx.find_affected_tests_mode(
                            payload,
                            guard.view,
                            self.enable_catapult,
                        ));
                    }
                }
                _ => {
                    affected.extend(c_idx.find_affected_tests_mode(
                        payload,
                        guard.view,
                        self.enable_catapult,
                    ));
                }
            }
        }
        affected.sort_unstable();
        affected.dedup();

        if let Err(reg) = c_idx.run_selective(guard.view, reg_cases, &affected) {
            return Ok(TransactionOutcome::RejectedByRegression {
                failed_case: reg.case_id,
                reason: reg.reason,
            });
        }

        // Modo sombra: si está activo, valida en paralelo contra la suite completa
        if self.shadow_mode {
            if let Err(full_fail) = run_regression_suite(guard.view, reg_cases) {
                panic!(
                    "FATAL SHADOW MODE VIOLATION: Catapult/Index filtered test but full suite failed! Case: '{}', reason: '{}'",
                    full_fail.case_id, full_fail.reason
                );
            }
        }

        guard.commit();

        let mut items = Vec::with_capacity(payloads.len());
        for (i, payload) in payloads.iter().enumerate() {
            let status = match payload {
                LexiconPayload::GraphFact { .. } | LexiconPayload::EdgeReinforcement { .. } => {
                    EntryStatus::ConfirmedFact
                }
                LexiconPayload::OpAlias { .. }
                | LexiconPayload::QuantumPolysemousAlias { .. }
                | LexiconPayload::PhraseMacro { .. }
                | LexiconPayload::EntityAlias { .. } => EntryStatus::LexicalAlias,
                LexiconPayload::ConsolidationCheckpoint { .. } => {
                    EntryStatus::ConsolidatedCheckpoint
                }
                LexiconPayload::Rollback { .. } => EntryStatus::Reverted,
            };
            items.push((ts + i as u64, status, payload.clone()));
        }

        match self.engram.append_batch(&items, self.durability) {
            Ok(entries) => {
                // Commit atómico del lote de telemetría tras confirmación de WAL
                self.telemetry_arena.commit_batch(&assessments, ts);
                let last = entries.last().unwrap();
                Ok(TransactionOutcome::Committed {
                    version: last.version,
                    entry_hash: last.entry_hash.clone(),
                })
            }
            Err(err) => Ok(TransactionOutcome::RejectedByIntegrity { reason: err.to_string() }),
        }
    }

    pub fn reinforce_edge(&mut self, from: &str, to: &str, delta: f64, ts: u64) -> TransactionOutcome {
        let Some((_, current_w, current_u)) = self.engram.active_view().active_facts.get(&(from.to_string(), to.to_string())).cloned() else {
            return TransactionOutcome::RejectedByOracle { reason: format!("Arista '{from} -> {to}' inexistente") };
        };
        let payload = LexiconPayload::EdgeReinforcement { from: from.to_string(), to: to.to_string(), new_weight: (current_w + delta).max(0.0), usage_count: current_u + 1 };
        self.propose_mutation(payload, ts)
    }

    pub fn consolidate_memory(&mut self, min_w: f64, min_fid: f64, ts: u64) -> TransactionOutcome {
        let (pruned_edges, mut pruned_view) = compute_consolidation_candidate(&self.engram, min_w);
        let mut before_view = self.engram.active_view().clone();
        let (fidelity_score, degraded_cases) = compute_consolidation_fidelity(&mut before_view, &mut pruned_view, &self.custom_regression_cases);
        if fidelity_score < min_fid || !degraded_cases.is_empty() {
            return TransactionOutcome::RejectedByFidelity { fidelity_score, required_fidelity: min_fid, degraded_cases };
        }
        let pruned_count = pruned_edges.len();
        let payload = LexiconPayload::ConsolidationCheckpoint { min_weight_threshold: min_w, pruned_edges, fidelity_score };
        match self.engram.append_entry(ts, EntryStatus::ConsolidatedCheckpoint, payload) {
            Ok(e) => { self.causal_index = None; TransactionOutcome::Consolidated { version: e.version, entry_hash: e.entry_hash, pruned_count, fidelity_score } }
            Err(err) => TransactionOutcome::RejectedByIntegrity { reason: err.to_string() },
        }
    }

    pub fn rollback_to(&mut self, target_v: u64, reason: impl Into<String>, ts: u64) -> TransactionOutcome {
        if target_v >= self.engram.current_version() {
            return TransactionOutcome::RejectedByIntegrity { reason: format!("target_v ({target_v}) >= actual ({})", self.engram.current_version()) };
        }
        let payload = LexiconPayload::Rollback { target_version: target_v, reason: reason.into() };
        match EngramStore::rebuild_view_at_version(self.engram.entries(), Some(target_v)) {
            Ok(mut candidate_view) => {
                if let Err(reg) = run_regression_suite(&mut candidate_view, &self.custom_regression_cases) {
                    return TransactionOutcome::RejectedByRegression { failed_case: reg.case_id, reason: reg.reason };
                }
            }
            Err(err) => return TransactionOutcome::RejectedByIntegrity { reason: err.to_string() },
        }
        match self.engram.append_entry(ts, EntryStatus::Reverted, payload) {
            Ok(e) => { self.causal_index = None; TransactionOutcome::RolledBack { version: e.version, target_version: target_v, entry_hash: e.entry_hash } }
            Err(err) => TransactionOutcome::RejectedByIntegrity { reason: err.to_string() },
        }
    }

    pub fn evaluate_query(&mut self, q: &str) -> (DispatchIntent, Answer) {
        let intent = parse_dispatch_intent_with_lexicon(q, Some(self.engram.active_view()));
        let answer = answer_intent(&intent, &mut self.engram.active_view_mut().episodic);
        (intent, answer)
    }

    pub fn propagate_activation(&self, s: &[&str], g: f64, st: usize, m: f64) -> SpreadingActivationResult { propagate_activation(&self.engram.active_view().episodic, s, g, st, m) }
    pub fn diff_versions(&self, vf: u64, vt: u64) -> Result<VersionDiff, IntegrityError> { self.engram.diff_versions(vf, vt) }
    pub fn resolve_polysemous_quantum(&self, w: &str, ctx: &[&str], l: f64, c: f64) -> Option<CollapseDecision<ArithmeticOp>> { self.engram.active_view().resolve_polysemous_quantum(w, ctx, l, c) }
    pub fn resolve_polysemous_classical(&self, w: &str, ctx: &[&str]) -> ClassicalPolysemyResult { self.engram.active_view().resolve_polysemous_classical(w, ctx) }
    pub fn analyze_spectrum(&self, sig: &[f64], eps: f64, min_c: f64, p: Option<f64>) -> SpectralOutcome { analyze_spectrum_donoho_stark(sig, eps, min_c, p) }
    pub fn verify_network_alert(&self, spec: &NetworkAlertSpec) -> VerifiedAlertReport { verify_network_alert(spec) }

    pub fn active_view(&self) -> &LexiconView { self.engram.active_view() }
    pub fn engram_store(&self) -> &EngramStore { &self.engram }
    pub fn engram_store_mut(&mut self) -> &mut EngramStore { &mut self.engram }
    pub fn custom_regression_cases(&self) -> &[RegressionCase] { &self.custom_regression_cases }
    pub fn retrieve_neighbors(&mut self, entity: &str) -> Vec<String> { self.engram.active_view_mut().episodic.get_neighbors(entity).to_vec() }
    pub fn current_version(&self) -> u64 { self.engram.current_version() }
    pub fn head_hash(&self) -> &str { self.engram.head_hash() }
    pub fn log_entries(&self) -> &[LogEntry] { self.engram.entries() }
    pub fn to_jsonl_string(&self) -> String { self.engram.to_jsonl_string() }
}
