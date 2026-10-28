//! # relic_blackwall
//!
//! Frontera transaccional Sandbox / Núcleo de RELIC.
//! Implementa el ciclo estricto:
//! `Proponer (Sandbox) -> Verificar (Oráculo + Regresión) -> Adoptar o Revertir (Commit/Rollback en JSONL con Hash-Chain)`.
//! Menos de 180 líneas.

pub mod assimilation;
pub mod oracle;
pub mod regression;
pub mod transaction;

pub use assimilation::AssimilationReport;
pub use oracle::{validate_proposal_oracle, OracleViolation, RESERVED_CORE_TOKENS};
pub use regression::{
    canonical_regression_suite, compute_consolidation_fidelity, run_regression_suite,
    RegressionCase, RegressionFailure,
};
pub use transaction::{BlackwallGate, TransactionOutcome};

