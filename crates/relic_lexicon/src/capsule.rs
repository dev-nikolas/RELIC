//! # relic_lexicon::capsule
//!
//! Exportación, verificación criptográfica SHA-256 e importación portátil de cápsulas `.engram`.
//! Permite compartir conocimiento léxico y relacional entre dispositivos Edge/offline
//! garantizando integridad de cadena de bloques ligera antes de pasar por el Sandbox del Blackwall.
use relic_core::sha256_hex;
use serde::{Deserialize, Serialize};
use crate::log::{compute_entry_hash, EntryStatus, IntegrityError, LexiconPayload};
use crate::memory::EngramStore;

pub const CAPSULE_SCHEMA_VERSION: u32 = 1;

/// Entrada portátil dentro de una cápsula `.engram`, encadenada criptográficamente dentro de la cápsula.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapsuleEntry {
    pub seq_index: u64,
    pub timestamp_ms: u64,
    pub status: EntryStatus,
    pub payload: LexiconPayload,
    pub prev_capsule_hash: String,
    pub entry_capsule_hash: String,
}

/// Cápsula portátil `.engram` con metadatos de dominio, cadena interna SHA-256 y sello criptográfico global.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngramCapsule {
    pub schema_version: u32,
    pub domain: String,
    pub source_version_from: u64,
    pub source_version_to: u64,
    pub source_head_hash: String,
    pub entries: Vec<CapsuleEntry>,
    pub merkle_seal_sha256: String,
}

pub const CAPSULE_GENESIS_HASH: &str =
    "c000000000000000000000000000000000000000000000000000000000000000";

impl EngramCapsule {
    /// Calcula el sello SHA-256 canónico sobre los metadatos y la cadena de entradas de la cápsula.
    pub fn compute_seal(
        schema_version: u32,
        domain: &str,
        v_from: u64,
        v_to: u64,
        source_head_hash: &str,
        entries: &[CapsuleEntry],
    ) -> String {
        let mut buf = format!(
            "capsule_v={schema_version}|domain={domain}|from={v_from}|to={v_to}|src_head={source_head_hash}|n={}",
            entries.len()
        );
        for e in entries {
            buf.push('|');
            buf.push_str(&e.entry_capsule_hash);
        }
        sha256_hex(buf.as_bytes())
    }

    /// Verifica la integridad criptográfica completa de la cápsula `.engram`.
    pub fn verify_cryptographic_integrity(&self) -> Result<(), IntegrityError> {
        let mut expected_prev = CAPSULE_GENESIS_HASH.to_string();
        for (idx, item) in self.entries.iter().enumerate() {
            let expected_seq = (idx + 1) as u64;
            if item.seq_index != expected_seq {
                return Err(IntegrityError::InvalidVersionSequence { expected: expected_seq, actual: item.seq_index });
            }
            if item.prev_capsule_hash != expected_prev {
                return Err(IntegrityError::BrokenChain {
                    version: item.seq_index, expected_prev, actual_prev: item.prev_capsule_hash.clone(),
                });
            }
            let computed = compute_entry_hash(item.seq_index, item.timestamp_ms, &item.prev_capsule_hash, item.status, &item.payload);
            if item.entry_capsule_hash != computed {
                return Err(IntegrityError::HashMismatch {
                    version: item.seq_index, expected: computed, actual: item.entry_capsule_hash.clone(),
                });
            }
            expected_prev = item.entry_capsule_hash.clone();
        }

        let expected_seal = Self::compute_seal(
            self.schema_version, &self.domain, self.source_version_from, self.source_version_to,
            &self.source_head_hash, &self.entries,
        );
        if self.merkle_seal_sha256 != expected_seal {
            return Err(IntegrityError::HashMismatch {
                version: self.source_version_to, expected: expected_seal, actual: self.merkle_seal_sha256.clone(),
            });
        }
        Ok(())
    }

    pub fn to_json_string(&self) -> Result<String, IntegrityError> {
        serde_json::to_string_pretty(self).map_err(|e| IntegrityError::IoError(e.to_string()))
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, IntegrityError> {
        serde_json::from_str(json_str).map_err(|e| IntegrityError::ParseError { line: 1, details: e.to_string() })
    }
}

impl EngramStore {
    /// Exporta el rango `[v_from, v_to]` de entradas activas del `EngramStore` como una cápsula `.engram` sellada.
    pub fn export_capsule(&self, domain: impl Into<String>, v_from: u64, v_to: u64) -> Result<EngramCapsule, IntegrityError> {
        let _ = Self::rebuild_active_view(self.entries())?;
        let domain_str = domain.into();
        let (mut prev_hash, mut capsule_entries) = (CAPSULE_GENESIS_HASH.to_string(), Vec::new());

        for entry in self.entries() {
            if entry.version >= v_from && entry.version <= v_to && entry.status != EntryStatus::Reverted {
                if matches!(entry.payload, LexiconPayload::Rollback { .. } | LexiconPayload::ConsolidationCheckpoint { .. }) {
                    continue;
                }
                let seq = (capsule_entries.len() + 1) as u64;
                let entry_hash = compute_entry_hash(seq, entry.timestamp_ms, &prev_hash, entry.status, &entry.payload);
                capsule_entries.push(CapsuleEntry {
                    seq_index: seq, timestamp_ms: entry.timestamp_ms, status: entry.status,
                    payload: entry.payload.clone(), prev_capsule_hash: prev_hash, entry_capsule_hash: entry_hash.clone(),
                });
                prev_hash = entry_hash;
            }
        }

        let seal = EngramCapsule::compute_seal(
            CAPSULE_SCHEMA_VERSION, &domain_str, v_from, v_to, self.head_hash(), &capsule_entries,
        );
        Ok(EngramCapsule {
            schema_version: CAPSULE_SCHEMA_VERSION, domain: domain_str, source_version_from: v_from,
            source_version_to: v_to, source_head_hash: self.head_hash().to_string(),
            entries: capsule_entries, merkle_seal_sha256: seal,
        })
    }
}
