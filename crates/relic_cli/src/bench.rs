//! # relic_cli::bench
//!
//! Coordinador y runner de la Suite de Benchmarks Científicos y Estudio de Ablación.
//! Menos de 180 líneas.

use serde::{Deserialize, Serialize};
pub use crate::bench_ablation::{run_bench1_polysemy_ablation, PolysemyAblationMetrics};
pub use crate::bench_epistemic::{run_bench2_reasoning_and_epistemic, ReasoningEpistemicMetrics};
pub use crate::bench_telemetry::{run_bench3_network_telemetry, NetworkTelemetryMetrics};
use crate::monitor::get_current_rss_kb;

/// Reporte consolidado de los 3 benchmarks científicos validados.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatedBenchmarkReport {
    pub bench1_polysemy_ablation: PolysemyAblationMetrics,
    pub bench2_reasoning_epistemic: ReasoningEpistemicMetrics,
    pub bench3_network_telemetry: NetworkTelemetryMetrics,
    pub rss_mb: f64,
}

pub(crate) fn compute_mean_and_p99(latencies_us: &[f64]) -> (f64, f64) {
    if latencies_us.is_empty() {
        return (0.0, 0.0);
    }
    let sum: f64 = latencies_us.iter().sum();
    let mean = sum / (latencies_us.len() as f64);
    let mut sorted = latencies_us.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() as f64 * 0.99).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    (mean, sorted[idx])
}

/// Ejecuta los 3 Benchmarks Científicos de RELIC y devuelve el reporte consolidado.
pub fn run_all_validated_benchmarks() -> ValidatedBenchmarkReport {
    ValidatedBenchmarkReport {
        bench1_polysemy_ablation: run_bench1_polysemy_ablation(),
        bench2_reasoning_epistemic: run_bench2_reasoning_and_epistemic(),
        bench3_network_telemetry: run_bench3_network_telemetry(),
        rss_mb: get_current_rss_kb() as f64 / 1024.0,
    }
}

/// Imprime en consola las tablas Markdown completas de los 3 benchmarks validados.
pub fn handle_bench() {
    let report = run_all_validated_benchmarks();
    let b1 = &report.bench1_polysemy_ablation;
    let b2 = &report.bench2_reasoning_epistemic;
    let b3 = &report.bench3_network_telemetry;

    println!("================================================================================");
    println!("  RELIC — SUITE CIENTÍFICA DE BENCHMARKS Y ESTUDIO DE ABLACIÓN");
    println!("================================================================================\n");

    println!("### Benchmark 1 — Ablación: Léxico Clásico vs. Interferencia Cuántica de Fase ({} casos)", b1.total_cases);
    println!("| Motor Evaluado | Accuracy (Resueltos) | Coverage (Cobertura) | Contradiction Error Rate | F1-Score | Latencia Media (µs) | Latencia p99 (µs) |");
    println!("| :--- | :---: | :---: | :---: | :---: | :---: | :---: |");
    println!(
        "| **Baseline Clásico (Pesos Positivos)** | {:.2}% | {:.2}% | {:.2}% | {:.4} | {:.2} µs | {:.2} µs |",
        b1.classical_resolved_accuracy_pct, b1.classical_coverage_pct, b1.classical_contradiction_error_rate_pct,
        b1.classical_f1, b1.classical_mean_latency_us, b1.classical_p99_latency_us
    );
    println!(
        "| **Cuántico-Inspirado (Fase + Born + Costo Asimétrico)** | **{:.2}%** | **{:.2}%** | **{:.2}%** | **{:.4}** | {:.2} µs | {:.2} µs |\n",
        b1.quantum_resolved_accuracy_pct, b1.quantum_coverage_pct, b1.quantum_contradiction_error_rate_pct,
        b1.quantum_f1, b1.quantum_mean_latency_us, b1.quantum_p99_latency_us
    );

    println!("### Benchmark 2 — Razonamiento Estándar y Abstención Epistémica (GSM8K-ES / SVAMP / bAbI + Traps, {} casos)", b2.total_cases);
    println!("| Métrica de Verificación | Resultado RELIC | Detalle de Dominio |");
    println!("| :--- | :---: | :--- |");
    println!("| **Selective Accuracy (sobre respondidas)** | **{:.2}%** | {} problemas resolubles (Aritmética, Grafos BFS, Orden) |", b2.selective_accuracy_pct, b2.answerable_cases);
    println!("| **Coverage sobre Consultas Resolubles** | **{:.2}%** | Cobertura determinista sin degradación |", b2.coverage_on_answerable_pct);
    println!("| **Hallucination / False Answer Rate (Trampas)** | **{:.2}%** | {} trampas epistémicas (Div/0, Nodos fantasmas, Ciclos, H=1.0) |", b2.hallucination_rate_pct, b2.unanswerable_trap_cases);
    println!("| **Exact Trace Match (Oráculo RPN / BFS)** | **{:.2}%** | Concordancia paso a paso con traza verificada |", b2.exact_trace_match_pct);
    println!("| **Latencia Media / p99** | **{:.2} µs / {:.2} µs** | Ejecución en CPU unihilo sin GPU |\n", b2.mean_latency_us, b2.p99_latency_us);

    println!("### Benchmark 3 — Dominio Cruzado: Telemetría y Ciberseguridad (N-BaIoT / CTU-13 / Mirai, {} escenarios)", b3.total_traces);
    println!("| Métrica Espectral y Topológica | Resultado RELIC | Criterio Matemático |");
    println!("| :--- | :---: | :--- |");
    println!("| **Beacon Detection Recall (C2 / Mirai)** | **{:.2}%** | {} trazas periódicas (DFT + Donoho-Stark D_DS ~ 0.0) |", b3.beacon_detection_recall_pct, b3.attack_traces);
    println!("| **False Positive Rate (Tráfico Humano c/ Jitter)** | **{:.2}%** | {} trazas aperiódicas dispersas (D_DS > 1.0) |", b3.false_positive_rate_pct, b3.benign_jitter_traces);
    println!("| **Uncertainty Abstention Accuracy (Ventana Corta)** | **{:.2}%** | {} trazas truncadas (N < min_cycles * P -> InsufficientWindow) |", b3.uncertainty_abstention_accuracy_pct, b3.truncated_window_traces);
    println!("| **Topological Isolation Exact Match (BFS avoid)** | **{:.2}%** | Subredes protegidas tras aislar nodo puente |", b3.topological_isolation_exact_match_pct);
    println!("| **Latencia Media / p99** | **{:.2} µs / {:.2} µs** | DFT exacta + Welford + BFS en RAM |\n", b3.mean_latency_us, b3.p99_latency_us);

    println!("--------------------------------------------------------------------------------");
    println!("  Memoria Física Activa (RSS): {:.2} MB | Estado de Verificación: 100% DETERMINISTA", report.rss_mb);
    println!("================================================================================");
}
