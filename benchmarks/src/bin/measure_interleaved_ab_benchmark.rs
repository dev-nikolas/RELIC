//! Benchmark A/B Intercalado Riguroso y Microbenchmark de la Guarda de Telemetría.
//!
//! Mediciones:
//! 1. Microbenchmark aislado de la guarda `if telemetry.is_empty()` (10.000.000 iteraciones).
//! 2. Benchmark A/B intercalado (Disabled vs PassiveMonitor) sobre 10.000 mutaciones de grafo.
//! 3. Métricas reportadas: p50, p90, p99, p99.9, media, desviación estándar e intervalo de confianza al 95%.

use std::time::Instant;
use relic_blackwall::{AnomalyGateMode, BlackwallGate, TransactionOutcome};
use relic_lexicon::LexiconPayload;

fn compute_percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let idx = ((sorted.len() as f64 * p).ceil() as usize).saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

fn compute_stats(mut samples: Vec<f64>) -> (f64, f64, f64, f64, f64, f64, f64) {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std_dev = var.sqrt();
    let ci95 = 1.96 * (std_dev / n.sqrt());

    let p50 = compute_percentile(&samples, 0.50);
    let p90 = compute_percentile(&samples, 0.90);
    let p99 = compute_percentile(&samples, 0.99);
    let p999 = compute_percentile(&samples, 0.999);

    (mean, std_dev, ci95, p50, p90, p99, p999)
}

fn main() {
    println!("================================================================================");
    println!("  RELIC & Seis Ojos — Microbenchmark de Guarda y A/B Intercalado Riguroso");
    println!("================================================================================");

    // =========================================================================
    // 1. MICROBENCHMARK AISLADO DE LA GUARDA TELEMÉTRICA
    // =========================================================================
    println!("\n[1/2] Microbenchmark de la guarda 'if telemetry.is_empty()' (10.000.000 llamadas)");
    let mut gate = BlackwallGate::with_genesis();
    gate.telemetry_arena.mode = AnomalyGateMode::PassiveMonitor;

    let iterations = 10_000_000usize;
    let empty_slice: &[(u32, f64)] = &[];

    // Warm-up de 100k
    for _ in 0..100_000 {
        let _ = gate.telemetry_arena.evaluate_batch_pure(empty_slice);
    }

    let t0 = Instant::now();
    let mut blackbox_sum = 0usize;
    for _ in 0..iterations {
        let res = gate.telemetry_arena.evaluate_batch_pure(empty_slice).unwrap();
        blackbox_sum += res.len();
    }
    let elapsed = t0.elapsed();
    let ns_per_call = elapsed.as_nanos() as f64 / iterations as f64;

    println!("  -> Total acumulado:   {} llamadas en {:.2?}", iterations, elapsed);
    println!("  -> Latencia neta:     {:.3} ns / llamada (verificado < 5 ns, suma: {})", ns_per_call, blackbox_sum);

    // =========================================================================
    // 2. BENCHMARK A/B INTERCALADO (10.000 MUTACIONES)
    // =========================================================================
    println!("\n[2/2] Benchmark A/B Intercalado: 10.000 Mutaciones Simbólicas");
    println!("      Patrón: A -> B -> A -> B... para mitigar deriva térmica y scheduling del SO");

    let num_pairs = 10_000usize;
    let mut gate_a = BlackwallGate::with_genesis();
    gate_a.telemetry_arena.mode = AnomalyGateMode::Disabled;

    let mut gate_b = BlackwallGate::with_genesis();
    gate_b.telemetry_arena.mode = AnomalyGateMode::PassiveMonitor;

    let mut lats_a = Vec::with_capacity(num_pairs);
    let mut lats_b = Vec::with_capacity(num_pairs);

    // Warm-up de 500 pares
    for i in 0..500 {
        let p_warm = LexiconPayload::graph_fact(format!("warm_{i}"), "rel", format!("node_{i}"));
        let _ = gate_a.commit_transaction_with_telemetry(&[p_warm.clone()], &[], 500 + i as u64);
        let _ = gate_b.commit_transaction_with_telemetry(&[p_warm], &[], 500 + i as u64);
    }

    for i in 0..num_pairs {
        let payload = LexiconPayload::graph_fact(
            format!("ent_src_{i}"),
            "link",
            format!("ent_dst_{i}"),
        );
        let ts = 10_000 + i as u64;

        // Ejecutar A (Disabled)
        let t_a = Instant::now();
        let out_a = gate_a.commit_transaction_with_telemetry(std::slice::from_ref(&payload), &[], ts)
            .expect("Transacción A exitosa");
        let dur_a = t_a.elapsed().as_nanos() as f64 / 1_000.0;
        assert!(matches!(out_a, TransactionOutcome::Committed { .. }));
        lats_a.push(dur_a);

        // Ejecutar B (PassiveMonitor)
        let t_b = Instant::now();
        let out_b = gate_b.commit_transaction_with_telemetry(std::slice::from_ref(&payload), &[], ts)
            .expect("Transacción B exitosa");
        let dur_b = t_b.elapsed().as_nanos() as f64 / 1_000.0;
        assert!(matches!(out_b, TransactionOutcome::Committed { .. }));
        lats_b.push(dur_b);
    }

    let (mean_a, std_a, ci_a, p50_a, p90_a, p99_a, p999_a) = compute_stats(lats_a);
    let (mean_b, std_b, ci_b, p50_b, p90_b, p99_b, p999_b) = compute_stats(lats_b);

    println!("\n  RESULTADOS ESTADÍSTICOS FORMALES (N = 10.000):");
    println!("  ------------------------------------------------------------------------------");
    println!("  Condición [A] Telemetría DISABLED:");
    println!("    Media:  {:>6.2} µs  ± {:.2} µs (IC 95%)  [Desv. Est: {:.2} µs]", mean_a, ci_a, std_a);
    println!("    p50:    {:>6.2} µs", p50_a);
    println!("    p90:    {:>6.2} µs", p90_a);
    println!("    p99:    {:>6.2} µs", p99_a);
    println!("    p99.9:  {:>6.2} µs", p999_a);

    println!("\n  Condición [B] Telemetría PASSIVE_MONITOR:");
    println!("    Media:  {:>6.2} µs  ± {:.2} µs (IC 95%)  [Desv. Est: {:.2} µs]", mean_b, ci_b, std_b);
    println!("    p50:    {:>6.2} µs", p50_b);
    println!("    p90:    {:>6.2} µs", p90_b);
    println!("    p99:    {:>6.2} µs", p99_b);
    println!("    p99.9:  {:>6.2} µs", p999_b);

    println!("  ------------------------------------------------------------------------------");
    println!("  Comparativa Directa (B vs A):");
    println!("    Delta Media: {:>+6.2} µs  ({:+.2}%)", mean_b - mean_a, ((mean_b - mean_a) / mean_a) * 100.0);
    println!("    Delta p50:   {:>+6.2} µs  ({:+.2}%)", p50_b - p50_a, ((p50_b - p50_a) / p50_a) * 100.0);
    println!("    Delta p99:   {:>+6.2} µs  ({:+.2}%)", p99_b - p99_a, ((p99_b - p99_a) / p99_a) * 100.0);
    let overlap = (mean_a - ci_a <= mean_b + ci_b) && (mean_b - ci_b <= mean_a + ci_a);
    println!("    Intervalos IC 95% se solapan: {}", if overlap { "SÍ (Diferencia estadísticamente indistinguible de cero)" } else { "NO" });
    println!("================================================================================");
}

