//! # relic_blackwall::benchmarks
//!
//! Re-exporta el arnés científico de benchmarking y ablación multi-dominio desde `relic_cli::bench`.
//! Menos de 180 líneas.

pub use relic_cli::bench::{
    run_all_validated_benchmarks, run_bench1_polysemy_ablation, run_bench2_reasoning_and_epistemic,
    run_bench3_network_telemetry, NetworkTelemetryMetrics, PolysemyAblationMetrics,
    ReasoningEpistemicMetrics, ValidatedBenchmarkReport,
};
