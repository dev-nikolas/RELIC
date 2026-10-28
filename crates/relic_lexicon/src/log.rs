//! Bitácora append-only con encadenamiento criptográfico SHA-256 (Hash-Chain).
//! Menos de 180 líneas.

use relic_core::{sha256_hex, ArithmeticOp};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Hash semilla del bloque génesis (64 ceros hexadecimales).
pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Estado epistemológico de cada entrada en la bitácora del engrama.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryStatus {
    ConfirmedFact,
    LexicalAlias,
    Reverted,
    ConsolidatedCheckpoint,
}

fn default_weight() -> f64 {
    1.0
}

fn default_zero() -> u64 {
    0
}

/// Tipos de carga útil (payload) soportados por el engrama versionado.
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
    },
    OpAlias {
        alias_word: String,
        target_op: ArithmeticOp,
        #[serde(default = "default_weight")]
        weight: f64,
        #[serde(default = "default_zero")]
        usage_count: u64,
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
        Self::GraphFact {
            from: from.into(),
            relation: relation.into(),
            to: to.into(),
            weight: 1.0,
            usage_count: 0,
        }
    }

    pub fn graph_fact_weighted(
        from: impl Into<String>,
        relation: impl Into<String>,
        to: impl Into<String>,
        weight: f64,
    ) -> Self {
        Self::GraphFact {
            from: from.into(),
            relation: relation.into(),
            to: to.into(),
            weight,
            usage_count: 0,
        }
    }

    pub fn op_alias(alias_word: impl Into<String>, target_op: ArithmeticOp) -> Self {
        Self::OpAlias {
            alias_word: alias_word.into(),
            target_op,
            weight: 1.0,
            usage_count: 0,
        }
    }

    pub fn phrase_macro(phrase: impl Into<String>, expansion: impl Into<String>) -> Self {
        Self::PhraseMacro {
            phrase: phrase.into(),
            expansion: expansion.into(),
            weight: 1.0,
            usage_count: 0,
        }
    }

    pub fn entity_alias(alias: impl Into<String>, canonical: impl Into<String>) -> Self {
        Self::EntityAlias {
            alias: alias.into(),
            canonical: canonical.into(),
            weight: 1.0,
            usage_count: 0,
        }
    }
}

/// Entrada individual e inmutable dentro del registro JSONL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    pub version: u64,
    pub timestamp_ms: u64,
    pub prev_hash: String,
    pub entry_hash: String,
    pub status: EntryStatus,
    pub payload: LexiconPayload,
}

/// Errores deterministas de integridad criptográfica y de formato JSONL.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    #[error("Corrupción de hash en versión {version}: esperado '{expected}', encontrado '{actual}'")]
    HashMismatch {
        version: u64,
        expected: String,
        actual: String,
    },

    #[error("Ruptura de cadena en versión {version}: prev_hash esperado '{expected_prev}', encontrado '{actual_prev}'")]
    BrokenChain {
        version: u64,
        expected_prev: String,
        actual_prev: String,
    },

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
    /// Construye y sella criptográficamente una nueva entrada encadenada al hash previo.
    pub fn new_sealed(
        version: u64,
        timestamp_ms: u64,
        prev_hash: impl Into<String>,
        status: EntryStatus,
        payload: LexiconPayload,
    ) -> Self {
        let prev = prev_hash.into();
        let entry_hash = compute_entry_hash(version, timestamp_ms, &prev, status, &payload);
        Self {
            version,
            timestamp_ms,
            prev_hash: prev,
            entry_hash,
            status,
            payload,
        }
    }

    /// Verifica que el `prev_hash` coincida con el eslabón anterior y que `entry_hash` sea auténtico.
    pub fn verify(&self, expected_version: u64, expected_prev_hash: &str) -> Result<(), IntegrityError> {
        if self.version != expected_version {
            return Err(IntegrityError::InvalidVersionSequence {
                expected: expected_version,
                actual: self.version,
            });
        }
        if self.prev_hash != expected_prev_hash {
            return Err(IntegrityError::BrokenChain {
                version: self.version,
                expected_prev: expected_prev_hash.to_string(),
                actual_prev: self.prev_hash.clone(),
            });
        }
        let computed = compute_entry_hash(
            self.version,
            self.timestamp_ms,
            &self.prev_hash,
            self.status,
            &self.payload,
        );
        if self.entry_hash != computed {
            return Err(IntegrityError::HashMismatch {
                version: self.version,
                expected: computed,
                actual: self.entry_hash.clone(),
            });
        }
        Ok(())
    }
}

/// Calcula el hash canónico SHA-256 sobre los campos deterministas de la entrada y `prev_hash`.
pub fn compute_entry_hash(
    version: u64,
    timestamp_ms: u64,
    prev_hash: &str,
    status: EntryStatus,
    payload: &LexiconPayload,
) -> String {
    let payload_json = serde_json::to_string(payload).unwrap_or_default();
    let canonical = format!(
        "v={version}|ts={timestamp_ms}|prev={prev_hash}|status={status:?}|payload={payload_json}"
    );
    sha256_hex(canonical.as_bytes())
}
