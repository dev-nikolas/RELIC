//! # relic_lexicon
//!
//! Sistema de Engrama Versionado (JSONL append-only con Hash-Chain SHA-256)
//! y proyección en RAM O(N) para aprendizaje léxico y relacional sin olvido catastrófico.
//! Menos de 180 líneas.

pub mod calibrator;
pub mod capsule;
pub mod consolidation;
pub mod diff;
pub mod genesis;
pub mod log;
pub mod memory;
pub mod polysemy;
pub mod view;

pub use calibrator::{auto_calibrate_phases, ContextObservation, PhaseCalibrationReport};
pub use capsule::{CapsuleEntry, EngramCapsule, CAPSULE_GENESIS_HASH, CAPSULE_SCHEMA_VERSION};
pub use consolidation::{compute_consolidation_candidate, ConsolidationCandidate};
pub use diff::{diff_versions, to_dot_diff, to_mermaid_diff, RevertedEntrySummary, VersionDiff, WeightDelta};
pub use genesis::{build_genesis_entries, genesis_jsonl_seed, GENESIS_FACTS};
pub use log::{
    compute_entry_hash, EntryStatus, IntegrityError, LexiconPayload, LogEntry, GENESIS_PREV_HASH,
};
pub use memory::{ClassicalPolysemyResult, EngramStore, PolysemousEntry};
pub use view::{InverseOp, LexiconView};
