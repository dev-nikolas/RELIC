//! Descomposición empírica de latencia de commit in-place y benchmark de suite de regresión causal.
use std::time::Instant;
use relic_blackwall::causal_index::CausalDependencyIndex;
use relic_blackwall::oracle::validate_proposal_oracle;
use relic_blackwall::regression::{run_regression_suite, RegressionCase};
use relic_blackwall::BlackwallGate;
use relic_lexicon::{EntryStatus, LexiconPayload};

fn run_breakdown_for_scale(scale: usize) {
    println!("\n=== DESCOMPOSICIÓN DE COMMIT IN-PLACE CON {scale} HECHOS ===");
    let mut gate = BlackwallGate::with_genesis();
    for i in 0..scale {
        let p = LexiconPayload::graph_fact(format!("ent_{i}"), "rel", format!("ent_{}", i + 1));
        gate.engram_store_mut().active_view_mut().apply_payload(&p);
    }

    let iterations = 30;
    let _custom_cases = gate.custom_regression_cases().to_vec();
    let mut dur_inplace_ns = Vec::with_capacity(iterations);
    let mut dur_oracle_ns = Vec::with_capacity(iterations);
    let mut dur_regr_ns = Vec::with_capacity(iterations);
    let mut dur_append_ns = Vec::with_capacity(iterations);
    let mut dur_total_ns = Vec::with_capacity(iterations);

    for it in 0..iterations {
        let payload = LexiconPayload::graph_fact(format!("src_{scale}_{it}"), "rel", format!("dst_{scale}_{it}"));

        let t0 = Instant::now();
        let inv = gate.engram_store_mut().active_view_mut().apply_payload_recording_undo(&payload);
        let t1 = Instant::now();

        validate_proposal_oracle(&payload, gate.active_view()).expect("oracle ok");
        let t2 = Instant::now();

        gate.validate_causal_regression(&payload).expect("regr ok");
        let t3 = Instant::now();

        gate.engram_store_mut().append_entry(1_700_000_000 + it as u64, EntryStatus::ConfirmedFact, payload).expect("append ok");
        let t4 = Instant::now();

        let _ = inv;
        dur_inplace_ns.push((t1 - t0).as_nanos());
        dur_oracle_ns.push((t2 - t1).as_nanos());
        dur_regr_ns.push((t3 - t2).as_nanos());
        dur_append_ns.push((t4 - t3).as_nanos());
        dur_total_ns.push((t4 - t0).as_nanos());
    }

    let mean_inplace_us = (dur_inplace_ns.iter().sum::<u128>() as f64 / iterations as f64) / 1_000.0;
    let mean_oracle_us = (dur_oracle_ns.iter().sum::<u128>() as f64 / iterations as f64) / 1_000.0;
    let mean_regr_us = (dur_regr_ns.iter().sum::<u128>() as f64 / iterations as f64) / 1_000.0;
    let mean_append_us = (dur_append_ns.iter().sum::<u128>() as f64 / iterations as f64) / 1_000.0;
    let mean_total_us = (dur_total_ns.iter().sum::<u128>() as f64 / iterations as f64) / 1_000.0;

    println!("Total Commit In-Place Medio:  {:.2} µs ({:.3} ms)", mean_total_us, mean_total_us / 1000.0);
    println!("  1. In-place + Undo Setup:   {:>7.2} µs ({:.1}%)", mean_inplace_us, (mean_inplace_us / mean_total_us) * 100.0);
    println!("  2. Oráculo de Invariantes:  {:>7.2} µs ({:.1}%)", mean_oracle_us, (mean_oracle_us / mean_total_us) * 100.0);
    println!("  3. Regresión Causal Invert: {:>7.2} µs ({:.1}%)", mean_regr_us, (mean_regr_us / mean_total_us) * 100.0);
    println!("  4. Append + SHA256:         {:>7.2} µs ({:.1}%)", mean_append_us, (mean_append_us / mean_total_us) * 100.0);
}

fn bench_regression_suite_scaling() {
    println!("\n=== B5: BENCHMARK DE REGRESIÓN: CIEGA O(R) vs CAUSAL O(|Cone|) ===");
    let mut gate = BlackwallGate::with_genesis();

    for &r_count in &[200, 1_000, 10_000] {
        let mut extra_cases = Vec::with_capacity(r_count);
        for i in 0..r_count {
            extra_cases.push(RegressionCase {
                id: format!("syn_reg_{i}"),
                query: format!("{} + {}", i % 100, (i * 7) % 50),
                expected_substring: format!("{}", (i % 100) + ((i * 7) % 50)),
                expected_route: "deterministic_arithmetic".to_string(),
                expected_status: "VALID".to_string(),
            });
        }

        // 1. Regresión Ciega O(R)
        let iters = 3;
        let t0 = Instant::now();
        for _ in 0..iters {
            let res = run_regression_suite(gate.engram_store_mut().active_view_mut(), &extra_cases);
            assert!(res.is_ok());
        }
        let dur_ms_blind = (t0.elapsed().as_nanos() as f64 / iters as f64) / 1_000_000.0;

        // 2. Regresión Causal con Índice Invertido
        let c_idx = CausalDependencyIndex::build(gate.engram_store_mut().active_view_mut(), &extra_cases);
        let test_payload = LexiconPayload::graph_fact("nuevo_nodo_a", "rel", "nuevo_nodo_b");
        let t1 = Instant::now();
        for _ in 0..iters * 100 {
            let affected = c_idx.find_affected_tests(&test_payload, gate.active_view());
            let res = c_idx.run_selective(gate.engram_store_mut().active_view_mut(), &extra_cases, &affected);
            assert!(res.is_ok());
        }
        let dur_us_causal = (t1.elapsed().as_nanos() as f64 / (iters * 100) as f64) / 1_000.0;
        let speedup = (dur_ms_blind * 1_000.0) / dur_us_causal;

        println!("R = {:<5} | Ciega: {:>7.2} ms ({:>5.2} µs/c) | Causal: {:>6.2} µs | Aceleración: {:>7.1}x",
                 r_count, dur_ms_blind, (dur_ms_blind * 1000.0) / (r_count as f64 + 10.0), dur_us_causal, speedup);
    }
}

fn main() {
    println!("============================================================");
    println!("  RELIC — Descomposición de Commit In-Place y Regresión (B4 y B5)");
    println!("============================================================");
    run_breakdown_for_scale(5_000);
    run_breakdown_for_scale(50_000);
    bench_regression_suite_scaling();
}
