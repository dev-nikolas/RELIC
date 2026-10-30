//! # relic_blackwall::assimilation
//!
//! Asimilación Autónoma ("Shadow Distillation"), Auto-Calibración de Fase
use relic_core::{
    simulate_kuramoto_synchronization, ArithmeticOp, KuramotoNetworkSpec, SynchronizationOutcome,
};
use relic_lexicon::{
    auto_calibrate_phases, ContextObservation, EngramCapsule, IntegrityError, LexiconPayload,
};
use serde::{Deserialize, Serialize};
use crate::oracle::RESERVED_CORE_TOKENS;
use crate::regression::RegressionCase;
use crate::transaction::{BlackwallGate, TransactionOutcome};

/// Resultado del ciclo de Destilación en la Sombra (`assimilate_verified_example`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssimilationReport {
    pub extracted_token: String,
    pub context_token: Option<String>,
    pub target_op: ArithmeticOp,
    pub outcome: TransactionOutcome,
    pub registered_regression_id: Option<String>,
}

const INVENTORY_CONTEXT_STOPWORDS: &[&str] = &[
    "me", "te", "se", "le", "les", "nos", "lo", "la", "los", "las", "un", "una", "unos", "unas",
    "mi", "mis", "tu", "tus", "su", "sus", "ese", "esa", "esos", "esas", "este", "esta", "estos",
    "estas", "hoy", "ayer", "ahora", "entonces", "cuánto", "cuanto", "cuántos", "cuantos",
    "cuántas", "cuantas", "queda", "quedan", "quedó", "quedo", "total", "final",
];

fn extract_unknown_candidate_from_query(query: &str, initial_unit: Option<&str>) -> Option<(String, Option<String>)> {
    let normalized = query.to_lowercase()
        .replace(',', " | ").replace(" luego ", " | ").replace(" después ", " | ")
        .replace(" despues ", " | ").replace(" pero ", " | ").replace(" y ", " | ");
    let clauses: Vec<&str> = normalized.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let target_clauses = if clauses.len() > 1 { &clauses[1..] } else { &clauses[..] };

    for clause in target_clauses {
        let has_number = clause.split_whitespace().any(|w| {
            let c = w.trim_matches(|ch: char| !ch.is_alphanumeric());
            !c.is_empty() && c.chars().all(|ch| ch.is_ascii_digit())
        });
        if !has_number { continue; }
        let mut candidates = Vec::new();
        for raw_w in clause.split_whitespace() {
            let clean = raw_w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            if clean.len() < 3 || clean.chars().all(|c| c.is_ascii_digit()) { continue; }
            if RESERVED_CORE_TOKENS.contains(&clean.as_str()) || INVENTORY_CONTEXT_STOPWORDS.contains(&clean.as_str()) { continue; }
            if initial_unit.is_some_and(|u| u.eq_ignore_ascii_case(&clean)) { continue; }
            candidates.push(clean);
        }
        if let Some(verb_token) = candidates.first().cloned() {
            return Some((verb_token, candidates.get(1).cloned()));
        }
    }
    None
}

impl BlackwallGate {
    /// Ejecuta dinámica de sincronización de Kuramoto sobre una especificación de osciladores acoplados.
    pub fn synchronize_kuramoto(&self, spec: &KuramotoNetworkSpec) -> SynchronizationOutcome {
        simulate_kuramoto_synchronization(spec)
    }

    /// Auto-calibra amplitudes y fases ($\theta = 0$ constructiva, $\theta = \pi$ destructiva)
    /// a partir de observaciones crudas y las comitea transaccionalmente en el Engrama.
    pub fn auto_calibrate_and_commit(&mut self, observations: &[ContextObservation], timestamp_ms: u64) -> TransactionOutcome {
        let reports = auto_calibrate_phases(observations);
        if reports.is_empty() {
            return TransactionOutcome::RejectedByOracle { reason: "No se suministraron observaciones válidas para calibrar.".to_string() };
        }
        let all_payloads: Vec<LexiconPayload> = reports.into_iter().flat_map(|r| r.synthesized_payloads).collect();
        self.commit_batch_payloads(&all_payloads, timestamp_ms)
    }

    /// Ciclo de Destilación en la Sombra ("Shadow Distillation"):
    /// Extrae el término desconocido de una consulta `INCOMPLETE`, sintetiza su alias o auto-calibra
    /// su fase si colisiona, valida en Sandbox, hace commit y registra su test de regresión protector.
    pub fn assimilate_verified_example(&mut self, raw_query: &str, verified_op: ArithmeticOp, timestamp_ms: u64) -> AssimilationReport {
        let first_unit = raw_query.split_whitespace().collect::<Vec<_>>().windows(2).find_map(|w| {
            let n = w[0].trim_matches(|c: char| !c.is_alphanumeric());
            (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                .then(|| w[1].trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        });

        let Some((token, ctx_opt)) = extract_unknown_candidate_from_query(raw_query, first_unit.as_deref()) else {
            return AssimilationReport {
                extracted_token: String::new(), context_token: None, target_op: verified_op,
                outcome: TransactionOutcome::RejectedByOracle { reason: "No se pudo extraer un token candidato no reservado de la consulta.".to_string() },
                registered_regression_id: None,
            };
        };

        let existing_op = self.active_view().resolve_op_alias(&token);
        let outcome = if let Some(prev_op) = existing_op.filter(|&op| op != verified_op) {
            let ctx_new = ctx_opt.clone().unwrap_or_else(|| "contexto_nuevo".to_string());
            let obs = vec![
                ContextObservation::new(token.clone(), "general", prev_op),
                ContextObservation::new(token.clone(), ctx_new, verified_op),
            ];
            self.auto_calibrate_and_commit(&obs, timestamp_ms)
        } else {
            self.propose_mutation(LexiconPayload::op_alias(token.clone(), verified_op), timestamp_ms)
        };

        let mut registered_regression_id = None;
        if let TransactionOutcome::Committed { version, .. } = &outcome {
            let (_, ans) = self.evaluate_query(raw_query);
            if ans.status == "VALID" {
                let reg_id = format!("auto_reg_v{version}_{token}");
                self.add_regression_case(RegressionCase {
                    id: reg_id.clone(), query: raw_query.to_string(),
                    expected_substring: ans.text, expected_route: ans.route, expected_status: ans.status,
                });
                registered_regression_id = Some(reg_id);
            }
        }

        AssimilationReport { extracted_token: token, context_token: ctx_opt, target_op: verified_op, outcome, registered_regression_id }
    }

    /// Exporta el rango `[v_from, v_to]` del Engrama como una cápsula portátil `.engram`.
    pub fn export_engram_capsule(&self, domain: impl Into<String>, v_from: u64, v_to: u64) -> Result<EngramCapsule, IntegrityError> {
        self.engram_store().export_capsule(domain, v_from, v_to)
    }

    /// Importa una cápsula `.engram` externa verificando integridad SHA-256, Oráculo y 100% de Regresión en Sandbox.
    pub fn import_engram_capsule(&mut self, capsule: &EngramCapsule, timestamp_ms: u64) -> TransactionOutcome {
        if let Err(err) = capsule.verify_cryptographic_integrity() {
            return TransactionOutcome::RejectedByIntegrity { reason: err.to_string() };
        }
        if capsule.entries.is_empty() {
            return TransactionOutcome::RejectedByIntegrity { reason: "La cápsula .engram no contiene entradas para fusionar.".to_string() };
        }
        let payloads: Vec<LexiconPayload> = capsule.entries.iter().map(|e| e.payload.clone()).collect();
        self.commit_batch_payloads(&payloads, timestamp_ms)
    }
}

