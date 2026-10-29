//! Benchmark Riguroso: Catapulta Causal A/B, Desglose por Fases y Modos de Durabilidad.
//!
//! Evalúa:
//! 1. Latencia ALU de la Catapulta sin contaminación de temporizador (lotes de 1.000 llamadas).
//! 2. A/B exacto: Catapulta ACTIVA vs INACTIVA sobre grafo Power-Law con hubs.
//! 3. Desglose analítico de latencia por fase (Oráculo, Undo, Catapulta, Regresión, SHA-256, I/O).
//! 4. Modos de durabilidad: Strict (por hecho) vs Group (por lote) vs Relaxed (page cache).

use std::fs;
use std::time::Instant;
use relic_blackwall::causal_index::CausalDependencyIndex;
use relic_blackwall::regression::RegressionCase;
use relic_blackwall::BlackwallGate;
use relic_lexicon::{DurabilityMode, LexiconPayload};

fn compute_percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let idx = ((sorted.len() as f64 * p).ceil() as usize).saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

fn build_power_law_graph(gate: &mut BlackwallGate, total_nodes: usize, m_edges_per_node: usize) {
    let mut rng_state = 0x1234_5678_9ABC_DEF0u64;
    let mut rand_u32 = || -> u32 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (rng_state >> 32) as u32
    };

    let mut repeated_nodes = Vec::new();
    // Semilla inicial K_m
    for i in 0..m_edges_per_node {
        for j in (i + 1)..m_edges_per_node {
            let p = LexiconPayload::graph_fact(format!("n_{i}"), "rel", format!("n_{j}"));
            gate.engram_store_mut().active_view_mut().apply_payload(&p);
            repeated_nodes.push(i as u32);
            repeated_nodes.push(j as u32);
        }
    }

    // Crecimiento por conexión preferencial (Barabási-Albert)
    for i in m_edges_per_node..total_nodes {
        let u_name = format!("n_{i}");
        let mut targets = std::collections::HashSet::new();
        while targets.len() < m_edges_per_node && !repeated_nodes.is_empty() {
            let idx = (rand_u32() as usize) % repeated_nodes.len();
            targets.insert(repeated_nodes[idx]);
        }
        for target in targets {
            let v_name = format!("n_{target}");
            let p = LexiconPayload::graph_fact(u_name.clone(), "rel", v_name);
            gate.engram_store_mut().active_view_mut().apply_payload(&p);
            repeated_nodes.push(i as u32);
            repeated_nodes.push(target);
        }
    }
    println!("Grafo Power-Law generado: {total_nodes} nodos, {} aristas, con hubs concentradores.", repeated_nodes.len() / 2);
}

fn microbench_catapult_latency_batched() {
    println!("\n=== [1/4] LATENCIA ALU MICRO-BENCHMARK (SIN CONTAMINACIÓN DE TIMER) ===");
    let mut gate = BlackwallGate::with_genesis();
    build_power_law_graph(&mut gate, 500, 3);

    let cases = vec![
        RegressionCase {
            id: "neg_cut_01".into(),
            query: "¿Es posible ir de n_400 a n_10?".into(),
            expected_substring: "inalcanzable".into(),
            expected_route: "deterministic_graph".into(),
            expected_status: "VALID".into(),
        },
        RegressionCase {
            id: "neg_cut_02".into(),
            query: "¿Es posible ir de n_450 a n_20?".into(),
            expected_substring: "inalcanzable".into(),
            expected_route: "deterministic_graph".into(),
            expected_status: "VALID".into(),
        },
    ];

    let c_idx = CausalDependencyIndex::build(gate.engram_store_mut().active_view_mut(), &cases);
    let view = gate.active_view();

    let u_id = view.episodic.interner.get_id("n_400").unwrap_or(0);
    let v_id = view.episodic.interner.get_id("n_10").unwrap_or(1);

    let samples = 1_000;
    let batch_size = 1_000;
    let mut latencies_per_call_ns = Vec::with_capacity(samples);

    for _ in 0..samples {
        let t0 = Instant::now();
        for _ in 0..batch_size {
            let res = c_idx.catapult.evaluate_edge_addition(u_id, v_id);
            std::hint::black_box(res);
        }
        let elapsed_ns = t0.elapsed().as_nanos() as f64;
        latencies_per_call_ns.push(elapsed_ns / (batch_size as f64));
    }

    latencies_per_call_ns.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = compute_percentile(&latencies_per_call_ns, 0.50);
    let p90 = compute_percentile(&latencies_per_call_ns, 0.90);
    let p99 = compute_percentile(&latencies_per_call_ns, 0.99);
    let mean = latencies_per_call_ns.iter().sum::<f64>() / samples as f64;

    println!("Evaluaciones por muestra:    {batch_size} (amortiza Instant::now)");
    println!("Muestras recolectadas:       {samples}");
    println!("Latencia ALU p50:            {:>6.2} ns/evaluación (Objetivo RFC: <= 10 ns)", p50);
    println!("Latencia ALU p90:            {:>6.2} ns/evaluación", p90);
    println!("Latencia ALU p99:            {:>6.2} ns/evaluación (Objetivo RFC: <= 35 ns)", p99);
    println!("Latencia ALU Media:          {:>6.2} ns/evaluación", mean);
    if p50 <= 10.0 && p99 <= 35.0 {
        println!("-> CUMPLE CRITERIO ESTRICTO DEL RFC: p50 <= 10 ns y p99 <= 35 ns.");
    } else {
        println!("-> OBSERVACIÓN: Latencia p50 ({:.2} ns) vs umbral teórico.", p50);
    }
}

fn measure_ab_catapult_on_vs_off_and_breakdown() {
    println!("\n=== [2/4] EXPERIMENTO A/B: CATAPULTA ACTIVA vs INACTIVA EN GRAFO POWER-LAW ===");
    let mut gate_on = BlackwallGate::with_genesis();
    build_power_law_graph(&mut gate_on, 1000, 3);
    let mut gate_off = gate_on.clone();

    gate_on.enable_catapult = true;
    gate_off.enable_catapult = false;

    let test_count = 500;
    let mut payloads = Vec::with_capacity(test_count);
    for i in 0..test_count {
        if i % 5 == 0 {
            // MQuAKE: reemplazo causal (replaces)
            payloads.push(LexiconPayload::GraphFact {
                from: format!("n_{}", i % 500),
                relation: "rel".into(),
                to: format!("n_{}", (i + 13) % 500),
                weight: 1.0,
                usage_count: 0,
                replaces: Some((format!("n_{}", i % 500), "rel".into(), format!("n_{}", (i + 1) % 500))),
            });
        } else {
            payloads.push(LexiconPayload::graph_fact(format!("n_{}", (i * 7) % 1000), "rel", format!("n_{}", (i * 11) % 1000)));
        }
    }

    // 1. Ejecución A (Catapulta ON)
    let t_on = Instant::now();
    for p in &payloads {
        let _ = gate_on.commit_batch_payloads(std::slice::from_ref(p), 1_700_000_000);
    }
    let dur_on_us = t_on.elapsed().as_micros() as f64 / test_count as f64;

    // 2. Ejecución B (Catapulta OFF)
    let t_off = Instant::now();
    for p in &payloads {
        let _ = gate_off.commit_batch_payloads(std::slice::from_ref(p), 1_700_000_000);
    }
    let dur_off_us = t_off.elapsed().as_micros() as f64 / test_count as f64;

    let speedup = dur_off_us / dur_on_us.max(0.1);
    println!("Catapulta INACTIVA (Base con cono invertido): {:>7.2} µs / commit en RAM", dur_off_us);
    println!("Catapulta ACTIVA   (Filtro de corte bitwise): {:>7.2} µs / commit en RAM", dur_on_us);
    println!("Aceleración A/B Real en RAM:                  {:>7.2}x", speedup);

    // 3. Desglose granular por fase (Catapulta ON)
    println!("\n=== [3/4] DESGLOSE ANALÍTICO POR FASE (COMMITS EN RAM) ===");
    let p = &payloads[1];
    let iters = 200;
    let mut dur_oracle = 0.0;
    let mut dur_undo = 0.0;
    let mut dur_find = 0.0;
    let mut dur_regr = 0.0;
    let mut dur_sha = 0.0;

    for _ in 0..iters {
        let t0 = Instant::now();
        let _ = relic_blackwall::validate_proposal_oracle(p, gate_on.active_view());
        dur_oracle += t0.elapsed().as_nanos() as f64;

        let t1 = Instant::now();
        let inv = gate_on.engram_store_mut().active_view_mut().apply_payload_recording_undo(p);
        dur_undo += t1.elapsed().as_nanos() as f64;

        let t2 = Instant::now();
        let c_idx = CausalDependencyIndex::build(gate_on.engram_store_mut().active_view_mut(), &[]);
        let aff = c_idx.find_affected_tests_mode(p, gate_on.active_view(), true);
        dur_find += t2.elapsed().as_nanos() as f64;

        let t3 = Instant::now();
        let _ = c_idx.run_selective(gate_on.engram_store_mut().active_view_mut(), &[], &aff);
        dur_regr += t3.elapsed().as_nanos() as f64;

        let t4 = Instant::now();
        let entry = relic_lexicon::LogEntry::new_sealed(1, 1_700_000_000, "prev", relic_lexicon::EntryStatus::ConfirmedFact, p.clone());
        let _ = serde_json::to_string(&entry);
        dur_sha += t4.elapsed().as_nanos() as f64;

        gate_on.engram_store_mut().active_view_mut().apply_inverse_op(inv);
    }

    let tot = (dur_oracle + dur_undo + dur_find + dur_regr + dur_sha) / (iters as f64 * 1000.0);
    println!("Total por commit en RAM:           {:>6.2} µs", tot);
    println!("  1. Oráculo Invariantes:          {:>6.2} µs ({:.1}%)", (dur_oracle / (iters as f64 * 1000.0)), ((dur_oracle / 1000.0) / (tot * iters as f64)) * 100.0);
    println!("  2. Apply in-place + Undo:        {:>6.2} µs ({:.1}%)", (dur_undo / (iters as f64 * 1000.0)), ((dur_undo / 1000.0) / (tot * iters as f64)) * 100.0);
    println!("  3. Catapulta + Búsqueda de Cono: {:>6.2} µs ({:.1}%)", (dur_find / (iters as f64 * 1000.0)), ((dur_find / 1000.0) / (tot * iters as f64)) * 100.0);
    println!("  4. Regresión Selectiva Sandbox:  {:>6.2} µs ({:.1}%)", (dur_regr / (iters as f64 * 1000.0)), ((dur_regr / 1000.0) / (tot * iters as f64)) * 100.0);
    println!("  5. Sello SHA-256 + Serializado:  {:>6.2} µs ({:.1}%)", (dur_sha / (iters as f64 * 1000.0)), ((dur_sha / 1000.0) / (tot * iters as f64)) * 100.0);
}

fn measure_durability_modes_comparison() {
    println!("\n=== [4/4] IMPACTO DEL I/O Y MODOS DE DURABILIDAD (GROUP COMMIT) ===");
    let temp_dir = std::env::temp_dir().join(format!("relic_bench_durability_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let batch_sizes = [1, 10, 50];
    for &b_size in &batch_sizes {
        println!("\n--- LOTE DE {b_size} HECHOS ---");
        let payloads: Vec<LexiconPayload> = (0..b_size)
            .map(|i| LexiconPayload::graph_fact(format!("src_{i}"), "rel", format!("dst_{i}")))
            .collect();

        // 1. Strict (fsync por operación)
        let log_strict = temp_dir.join(format!("log_strict_{b_size}.jsonl"));
        let mut gate_strict = BlackwallGate::open_or_init_file(&log_strict).unwrap();
        gate_strict.durability = DurabilityMode::Strict;
        let t0 = Instant::now();
        gate_strict.commit_batch_payloads(&payloads, 1_700_000_000);
        let dur_strict_ms = t0.elapsed().as_micros() as f64 / 1000.0;

        // 2. Group (1 fsync por lote agrupado con BufWriter)
        let log_group = temp_dir.join(format!("log_group_{b_size}.jsonl"));
        let mut gate_group = BlackwallGate::open_or_init_file(&log_group).unwrap();
        gate_group.durability = DurabilityMode::Group;
        let t1 = Instant::now();
        gate_group.commit_batch_payloads(&payloads, 1_700_000_000);
        let dur_group_ms = t1.elapsed().as_micros() as f64 / 1000.0;

        // 3. Relaxed (Page Cache, sin fsync bloqueante)
        let log_relaxed = temp_dir.join(format!("log_relaxed_{b_size}.jsonl"));
        let mut gate_relaxed = BlackwallGate::open_or_init_file(&log_relaxed).unwrap();
        gate_relaxed.durability = DurabilityMode::Relaxed;
        let t2 = Instant::now();
        gate_relaxed.commit_batch_payloads(&payloads, 1_700_000_000);
        let dur_relaxed_ms = t2.elapsed().as_micros() as f64 / 1000.0;

        println!("  Strict  (sync_data individual): {:>8.2} ms ({:>6.2} ms/hecho)", dur_strict_ms, dur_strict_ms / b_size as f64);
        println!("  Group   (sync_data agrupado):   {:>8.2} ms ({:>6.2} ms/hecho) [Aceleración: {:.1}x]", dur_group_ms, dur_group_ms / b_size as f64, dur_strict_ms / dur_group_ms.max(0.001));
        println!("  Relaxed (Page Cache / diferido):{:>8.2} ms ({:>6.2} µs/hecho) [Aceleración: {:.1}x]", dur_relaxed_ms, (dur_relaxed_ms * 1000.0) / b_size as f64, dur_strict_ms / dur_relaxed_ms.max(0.001));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

fn main() {
    println!("============================================================");
    println!("  RELIC — Auditoría Empírica: Catapulta Causal A/B e I/O");
    println!("============================================================");
    microbench_catapult_latency_batched();
    measure_ab_catapult_on_vs_off_and_breakdown();
    measure_durability_modes_comparison();
}
