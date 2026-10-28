//! # relic_blackwall
//!
//! Frontera transaccional Sandbox / Núcleo de RELIC.
//! Implementa el ciclo estricto:
//! `Proponer (Sandbox) -> Verificar (Oráculo + Regresión) -> Adoptar o Revertir (Commit/Rollback en JSONL con Hash-Chain)`.
//! Menos de 180 líneas.

pub mod benchmarks;
pub mod oracle;
pub mod regression;
pub mod transaction;

pub use benchmarks::{
    run_all_validated_benchmarks, run_bench1_polysemy_ablation, run_bench2_reasoning_and_epistemic,
    run_bench3_network_telemetry, NetworkTelemetryMetrics, PolysemyAblationMetrics,
    ReasoningEpistemicMetrics, ValidatedBenchmarkReport,
};
pub use oracle::{validate_proposal_oracle, OracleViolation, RESERVED_CORE_TOKENS};
pub use regression::{
    canonical_regression_suite, compute_consolidation_fidelity, run_regression_suite,
    RegressionCase, RegressionFailure,
};
pub use transaction::{BlackwallGate, TransactionOutcome};

