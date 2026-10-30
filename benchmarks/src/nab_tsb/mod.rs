//! Módulo de Benchmarks de Detección de Anomalías (NAB, TSB y AUC-ROC).

pub mod metrics;
pub mod runner;

pub use metrics::{compute_auc_metrics, compute_nab_score, NabProfile, NabTsbMetrics};
pub use runner::run_nab_tsb_evaluation;

