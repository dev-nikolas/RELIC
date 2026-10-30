//! Benchmark empírico: MS-BFS (256 Mundos SIMD y Lotes) vs BFS Secuencial.
use std::time::Instant;
use relic_core::episodic_memory::EpisodicMemory;
use relic_core::ms_bfs::{solve_ms_bfs_batch, solve_ms_bfs_counterfactual};
use relic_core::reasoning_engine::ReasoningEngine;

fn build_graph(n: usize) -> EpisodicMemory {
    let mut mem = EpisodicMemory::new();
    for i in 0..n {
        let u = format!("N_{:04}", i);
        if i + 1 < n { mem.insert_edge(&u, format!("N_{:04}", i + 1)); }
        if i + 3 < n && i % 2 == 0 { mem.insert_edge(&u, format!("N_{:04}", i + 3)); }
        if i + 7 < n && i % 3 == 0 { mem.insert_edge(&u, format!("N_{:04}", i + 7)); }
        if i + 15 < n && i % 5 == 0 { mem.insert_edge(&u, format!("N_{:04}", i + 15)); }
    }
    for i in 0..n {
        if i % 3 == 0 && i + 7 < n {
            let u = format!("N_{:04}", i);
            let v = format!("N_{:04}", i + 7);
            mem.set_edge_world_mask(&u, &v, 0x5555_5555_5555_5555u64);
        }
        if i % 5 == 0 && i + 15 < n {
            let u = format!("N_{:04}", i);
            let v = format!("N_{:04}", i + 15);
            mem.set_edge_world_mask(&u, &v, 0xAAAA_AAAA_AAAA_AAAAu64);
        }
    }
    mem
}

fn main() {
    println!("=== BENCHMARK: MS-BFS SIMD (256 MUNDOS CONTRAFACTUALES Y LOTES) ===");
    let n = 1200;
    let mut mem = build_graph(n);
    println!("Grafo construido: {} nodos, {} aristas", mem.total_node_count(), mem.edge_count());

    // PARTE 1: 256 MUNDOS CONTRAFACTUALES
    let s_id = mem.interner.get_id("N_0000").unwrap();
    let t_id = mem.interner.get_id("N_0050").unwrap();
    let iters = 200;
    let worlds = 256;

    // Warm-up
    for _ in 0..10 {
        let _ = solve_ms_bfs_counterfactual(&mem, s_id, t_id, worlds);
        for _ in 0..32 {
            let _ = ReasoningEngine::solve_reachability_bfs(&mut mem, "N_0000", "N_0050", 100);
        }
    }

    let t0 = Instant::now();
    for _ in 0..iters { let _ = solve_ms_bfs_counterfactual(&mem, s_id, t_id, worlds); }
    let us_cf_batch = (t0.elapsed().as_micros() as f64) / (iters as f64);

    let t1 = Instant::now();
    for _ in 0..iters {
        for _ in 0..worlds {
            let _ = ReasoningEngine::solve_reachability_bfs(&mut mem, "N_0000", "N_0050", 100);
        }
    }
    let us_cf_seq_total = (t1.elapsed().as_micros() as f64) / (iters as f64);
    let us_cf_single = us_cf_seq_total / (worlds as f64);
    let speedup_cf = us_cf_seq_total / us_cf_batch;
    let ratio_cf = us_cf_batch / us_cf_single;

    println!("\n[1] EVALUACIÓN DE 256 MUNDOS CONTRAFACTUALES SIMULTÁNEOS (AVX2/SIMD):");
    println!("- Tiempo Lote MS-BFS (256 mundos en paralelo): {:.2} µs [MEDIDO]", us_cf_batch);
    println!("- Tiempo Acumulado Secuencial (256 mundos): {:.2} µs [MEDIDO]", us_cf_seq_total);
    println!("- Tiempo Medio por Mundo Individual: {:.2} µs [MEDIDO]", us_cf_single);
    println!("- Ratio Lote vs Mundo Individual: {:.2}x [MEDIDO]", ratio_cf);
    println!("- Aceleración (Speedup Lote vs Secuencial): {:.2}x [MEDIDO]", speedup_cf);
    println!("- Criterio Salida (Ratio <= 3.0x): {}", if ratio_cf <= 3.0 { "CUMPLIDO [MEDIDO]" } else { "NO CUMPLIDO [MEDIDO]" });
    println!("- Criterio Salida (Speedup >= 80.0x): {}", if speedup_cf >= 80.0 { "CUMPLIDO [MEDIDO]" } else { "NO CUMPLIDO [MEDIDO]" });

    // PARTE 2: LOTE DE 256 CONSULTAS
    let mut queries_u32 = Vec::new();
    let mut queries_str = Vec::new();
    for i in 0..256 {
        let s = format!("N_{:04}", (i * 4) % (n - 60));
        let t = format!("N_{:04}", (i * 4) % (n - 60) + 30);
        let sid = mem.interner.get_id(&s).unwrap();
        let tid = mem.interner.get_id(&t).unwrap();
        queries_u32.push((sid, tid));
        queries_str.push((s, t));
    }

    let t2 = Instant::now();
    for _ in 0..iters { let _ = solve_ms_bfs_batch(&mem, &queries_u32); }
    let us_batch_queries = (t2.elapsed().as_micros() as f64) / (iters as f64);

    let t3 = Instant::now();
    for _ in 0..iters {
        for (s, t) in &queries_str {
            let _ = ReasoningEngine::solve_reachability_bfs(&mut mem, s, t, 100);
        }
    }
    let us_seq_queries = (t3.elapsed().as_micros() as f64) / (iters as f64);
    let us_single_q = us_seq_queries / (queries_str.len() as f64);
    let speedup_q = us_seq_queries / us_batch_queries;
    let ratio_q = us_batch_queries / us_single_q;

    println!("\n[2] LOTE DE 256 CONSULTAS DISTRIBUIDAS (SIMD):");
    println!("- Tiempo Lote MS-BFS (256 consultas): {:.2} µs [MEDIDO]", us_batch_queries);
    println!("- Tiempo Acumulado Secuencial (256 consultas): {:.2} µs [MEDIDO]", us_seq_queries);
    println!("- Tiempo Medio por Consulta: {:.2} µs [MEDIDO]", us_single_q);
    println!("- Ratio Lote vs Consulta Individual: {:.2}x [MEDIDO]", ratio_q);
    println!("- Aceleración (Speedup Lote vs Secuencial): {:.2}x [MEDIDO]", speedup_q);
}
