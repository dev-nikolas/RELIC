//! # relic_blackwall
//!
//! Frontera transaccional Sandbox / Núcleo de RELIC.
//! Implementa el ciclo estricto:
//! `Proponer (Sandbox) -> Verificar (Oráculo + Regresión) -> Adoptar o Revertir (Commit/Rollback en JSONL con Hash-Chain)`.
//! Menos de 180 líneas.

pub mod assimilation;
pub mod catapult;
pub mod causal_index;
pub mod oracle;
pub mod regression;
pub mod telemetry_arena;
pub mod transaction;

pub use assimilation::AssimilationReport;
pub use catapult::{CausalCatapult, RegressionSuiteMask};
pub use causal_index::CausalDependencyIndex;
pub use oracle::{validate_proposal_oracle, OracleViolation, RESERVED_CORE_TOKENS};
pub use regression::{
    canonical_regression_suite, compute_consolidation_fidelity, run_regression_suite,
    RegressionCase, RegressionFailure,
};
pub use telemetry_arena::{
    AnomalyGateMode, BindingTarget, BlackwallError, TelemetryArena, TelemetryAuditRecord,
    DEFAULT_MAX_CHANNELS,
};
pub use transaction::{BlackwallGate, TransactionOutcome};
