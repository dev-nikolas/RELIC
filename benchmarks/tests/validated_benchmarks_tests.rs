//! Suite de Pruebas de Regresión de Benchmarks Validados (`validated_benchmarks_tests.rs`):
//! Verifica con `assert!` estrictos las cotas de calidad de los 3 Benchmarks Científicos:
//! 1. Ablación Polisémica: `assert!(quantum_f1 > classical_f1)` y `quantum_contradiction_error_rate_pct == 0.0`.
//! 2. Razonamiento y Trampas Epistémicas: `assert_eq!(hallucination_rate, 0.0)` y `selective_accuracy_pct == 100.0`.
//! 3. Telemetría y Ciberseguridad: `assert_eq!(false_positive_rate_network, 0.0)` y `beacon_detection_recall_pct == 100.0`.

use relic_benchmarks::{
    run_all_validated_benchmarks, run_bench1_polysemy_ablation, run_bench2_reasoning_and_epistemic,
    run_bench3_network_telemetry,
};

#[test]
fn test_1_bench1_polysemy_ablation_quantum_strictly_outperforms_classical() {
    let m = run_bench1_polysemy_ablation();
    assert!(m.total_cases >= 20, "El dataset de ablación debe tener al menos 20 casos estructurados");
    let quantum_f1 = m.quantum_f1;
    let classical_f1 = m.classical_f1;
    assert!(
        quantum_f1 > classical_f1,
        "El F1 cuántico ({quantum_f1:.4}) debe superar estrictamente al F1 clásico ({classical_f1:.4})"
    );
    assert!((m.quantum_resolved_accuracy_pct - 100.0).abs() < 1e-9);
    assert!((m.quantum_coverage_pct - 100.0).abs() < 1e-9);
    assert_eq!(m.quantum_contradiction_error_rate_pct, 0.0);
    assert!(m.quantum_mean_latency_us >= 0.0 && m.quantum_p99_latency_us >= 0.0);
}

#[test]
fn test_2_bench2_reasoning_and_epistemic_zero_hallucination_and_exact_traces() {
    let m = run_bench2_reasoning_and_epistemic();
    assert!(m.total_cases >= 30);
    let hallucination_rate = m.hallucination_rate_pct;
    assert_eq!(hallucination_rate, 0.0, "Cota innegociable: cero alucinación en trampas epistémicas");
    assert!((m.selective_accuracy_pct - 100.0).abs() < 1e-9);
    assert!((m.exact_trace_match_pct - 100.0).abs() < 1e-9);
}

#[test]
fn test_3_bench3_network_telemetry_zero_false_positives_and_full_isolation() {
    let m = run_bench3_network_telemetry();
    assert!(m.total_traces >= 12);
    let false_positive_rate_network = m.false_positive_rate_pct;
    assert_eq!(false_positive_rate_network, 0.0, "Cota innegociable: cero falsos positivos sobre tráfico humano con jitter");
    assert!((m.beacon_detection_recall_pct - 100.0).abs() < 1e-9);
    assert!((m.uncertainty_abstention_accuracy_pct - 100.0).abs() < 1e-9);
    assert!((m.topological_isolation_exact_match_pct - 100.0).abs() < 1e-9);
}

#[test]
fn test_4_consolidated_benchmark_report_consistency() {
    let rep = run_all_validated_benchmarks();
    assert_eq!(rep.bench1_polysemy_ablation.total_cases, 24);
    assert_eq!(rep.bench2_reasoning_epistemic.total_cases, 32);
    assert_eq!(rep.bench3_network_telemetry.total_traces, 14);
    assert_eq!(rep.bench4_continual_learning.total_edits, 200);
    assert!(rep.bench4_continual_learning.locality_retention_pct >= 99.9);
    assert!(rep.bench5_kuramoto_transition.subcritical_abstention_pct >= 99.9);
    assert!(rep.bench6_stochastic_jitter_roc.max_resilient_jitter_pct >= 10.0);
    assert!(rep.rss_mb > 0.0);
}
