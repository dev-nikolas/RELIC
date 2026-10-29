//! Bitácora append-only con encadenamiento criptográfico SHA-256 (Hash-Chain). Menos de 180 líneas.

use relic_core::{sha256_hex, ArithmeticOp};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryStatus {
    ConfirmedFact,
    LexicalAlias,
    Reverted,
    ConsolidatedCheckpoint,
}

fn default_weight() -> f64 { 1.0 }
fn default_zero() -> u64 { 0 }
fn default_zero_f64() -> f64 { 0.0 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LexiconPayload {
    GraphFact {
        from: String,
        relation: String,
        to: String,
        #[serde(default = "default_weight")]
        weight: f64,
        #[serde(default = "default_zero")]
        usage_count: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        replaces: Option<(String, String, String)>,
    },
    OpAlias {
        alias_word: String,
        target_op: ArithmeticOp,
        #[serde(default = "default_weight")]
        weight: f64,
        #[serde(default = "default_zero")]
        usage_count: u64,
    },
    QuantumPolysemousAlias {
        alias_word: String,
        target_op: ArithmeticOp,
        #[serde(default = "default_weight")]
        amplitude_r: f64,
        #[serde(default = "default_zero_f64")]
        phase_theta: f64,
        context_word: Option<String>,
    },
    PhraseMacro {
        phrase: String,
        expansion: String,
        #[serde(default = "default_weight")]
        weight: f64,
        #[serde(default = "default_zero")]
        usage_count: u64,
    },
    EntityAlias {
        alias: String,
        canonical: String,
        #[serde(default = "default_weight")]
        weight: f64,
        #[serde(default = "default_zero")]
        usage_count: u64,
    },
    EdgeReinforcement {
        from: String,
        to: String,
        new_weight: f64,
        usage_count: u64,
    },
    ConsolidationCheckpoint {
        min_weight_threshold: f64,
        pruned_edges: Vec<(String, String)>,
        fidelity_score: f64,
    },
    Rollback {
        target_version: u64,
        reason: String,
    },
}

impl LexiconPayload {
    pub fn graph_fact(from: impl Into<String>, relation: impl Into<String>, to: impl Into<String>) -> Self {
        Self::GraphFact { from: from.into(), relation: relation.into(), to: to.into(), weight: 1.0, usage_count: 0, replaces: None }
    }
    pub fn graph_fact_weighted(from: impl Into<String>, relation: impl Into<String>, to: impl Into<String>, weight: f64) -> Self {
        Self::GraphFact { from: from.into(), relation: relation.into(), to: to.into(), weight, usage_count: 0, replaces: None }
    }
    pub fn graph_fact_replacing(from: impl Into<String>, relation: impl Into<String>, to: impl Into<String>, replaces: (String, String, String)) -> Self {
        Self::GraphFact { from: from.into(), relation: relation.into(), to: to.into(), weight: 1.0, usage_count: 0, replaces: Some(replaces) }
    }
    pub fn op_alias(alias_word: impl Into<String>, target_op: ArithmeticOp) -> Self {
        Self::OpAlias { alias_word: alias_word.into(), target_op, weight: 1.0, usage_count: 0 }
    }
    pub fn quantum_polysemous_alias(
        alias_word: impl Into<String>,
        target_op: ArithmeticOp,
        amplitude_r: f64,
        phase_theta: f64,
        context_word: Option<String>,
    ) -> Self {
        Self::QuantumPolysemousAlias { alias_word: alias_word.into(), target_op, amplitude_r, phase_theta, context_word }
    }
    pub fn phrase_macro(phrase: impl Into<String>, expansion: impl Into<String>) -> Self {
        Self::PhraseMacro { phrase: phrase.into(), expansion: expansion.into(), weight: 1.0, usage_count: 0 }
    }
    pub fn entity_alias(alias: impl Into<String>, canonical: impl Into<String>) -> Self {
        Self::EntityAlias { alias: alias.into(), canonical: canonical.into(), weight: 1.0, usage_count: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    pub version: u64,
    pub timestamp_ms: u64,
    pub prev_hash: String,
    pub entry_hash: String,
    pub status: EntryStatus,
    pub payload: LexiconPayload,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    #[error("Corrupción de hash en versión {version}: esperado '{expected}', encontrado '{actual}'")]
    HashMismatch { version: u64, expected: String, actual: String },
    #[error("Ruptura de cadena en versión {version}: prev_hash esperado '{expected_prev}', encontrado '{actual_prev}'")]
    BrokenChain { version: u64, expected_prev: String, actual_prev: String },
    #[error("Secuencia de versión inválida: esperada {expected}, encontrada {actual}")]
    InvalidVersionSequence { expected: u64, actual: u64 },
    #[error("Objetivo de rollback inválido en versión {version}: target_version {target_version} fuera de rango")]
    InvalidRollbackTarget { version: u64, target_version: u64 },
    #[error("Error de sintaxis JSONL en línea {line}: {details}")]
    ParseError { line: usize, details: String },
    #[error("Error de E/S en bitácora JSONL: {0}")]
    IoError(String),
}

impl LogEntry {
    pub fn new_sealed(version: u64, timestamp_ms: u64, prev_hash: impl Into<String>, status: EntryStatus, payload: LexiconPayload) -> Self {
        let prev = prev_hash.into();
        let entry_hash = compute_entry_hash(version, timestamp_ms, &prev, status, &payload);
        Self { version, timestamp_ms, prev_hash: prev, entry_hash, status, payload }
    }
    pub fn verify(&self, expected_version: u64, expected_prev_hash: &str) -> Result<(), IntegrityError> {
        if self.version != expected_version {
            return Err(IntegrityError::InvalidVersionSequence { expected: expected_version, actual: self.version });
        }
        if self.prev_hash != expected_prev_hash {
            return Err(IntegrityError::BrokenChain { version: self.version, expected_prev: expected_prev_hash.to_string(), actual_prev: self.prev_hash.clone() });
        }
        let computed = compute_entry_hash(self.version, self.timestamp_ms, &self.prev_hash, self.status, &self.payload);
        if self.entry_hash != computed {
            return Err(IntegrityError::HashMismatch { version: self.version, expected: computed, actual: self.entry_hash.clone() });
        }
        Ok(())
    }
}

pub fn compute_entry_hash(version: u64, timestamp_ms: u64, prev_hash: &str, status: EntryStatus, payload: &LexiconPayload) -> String {
    let payload_json = serde_json::to_string(payload).unwrap_or_default();
    let canonical = format!("v={version}|ts={timestamp_ms}|prev={prev_hash}|status={status:?}|payload={payload_json}");
    sha256_hex(canonical.as_bytes())
}
