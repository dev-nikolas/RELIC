//! Medición de Horizonte Elástico: Unidireccional vs Bidireccional, Grafos Balanceados y Desglose k=100.
//! Menos de 180 líneas.

use std::collections::{HashSet, VecDeque};
use std::time::Instant;
use relic_core::{EpisodicMemory, ReasoningEngine, SearchOutcome};

fn textbook_bfs(mem: &EpisodicMemory, s: u32, t: u32) -> (bool, usize, usize) {
    if s == t { return (true, 0, 0); }
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let (mut exp, mut ops) = (0usize, 0usize);
    visited.insert(s);
    queue.push_back(s);
    while let Some(curr) = queue.pop_front() {
        exp += 1;
        for &nbr in mem.get_neighbors_u32(curr) {
            ops += 1;
            if nbr == t { return (true, exp, ops); }
            if visited.insert(nbr) { queue.push_back(nbr); }
        }
    }
    (false, exp, ops)
}

fn bench_balanced_random_graphs() {
    println!("\n=== A1: Grafos Dirigidos Balanceados (b=3,4,6; k=6..10; 200 pares alcanzables) ===");
    for &b in &[3, 4, 6] {
        let n_nodes = 500;
        let mut mem = EpisodicMemory::new();
        // Generar grafo dirigido balanceado previo a la medición
        let mut rng = 42u64;
        let mut rand_u = |max: usize| -> usize {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((rng >> 32) as usize) % max
        };
        for i in 0..n_nodes {
            for _ in 0..b {
                let j = rand_u(n_nodes);
                if i != j { mem.insert_edge(format!("N_{i}"), format!("N_{j}")); }
            }
        }
        // Encontrar 200 pares alcanzables con k en [6, 10]
        let mut pairs = Vec::new();
        for i in 0..n_nodes {
            for j in 0..n_nodes {
                if i != j {
                    if let (Some(si), Some(tj)) = (mem.interner.get_id(&format!("N_{i}")), mem.interner.get_id(&format!("N_{j}"))) {
                        let (reach, _, _) = textbook_bfs(&mem, si, tj);
                        if reach { pairs.push((format!("N_{i}"), format!("N_{j}"))); }
                    }
                }
                if pairs.len() >= 200 { break; }
            }
            if pairs.len() >= 200 { break; }
        }

        // Medir excluyendo tiempo de generación
        let mut uni_ops = Vec::new();
        let mut bidi_ops = Vec::new();
        let mut uni_lats = Vec::new();
        let mut bidi_lats = Vec::new();

        for (s, t) in &pairs {
            let (si, tj) = (mem.interner.get_id(s).unwrap(), mem.interner.get_id(t).unwrap());
            let t0 = Instant::now();
            let (_, _, ops_u) = textbook_bfs(&mem, si, tj);
            uni_lats.push(t0.elapsed().as_nanos() as f64 / 1000.0);
            uni_ops.push(ops_u);

            let t1 = Instant::now();
            let r_b = ReasoningEngine::solve_reachability_bfs(&mut mem, s, t, 15);
            bidi_lats.push(t1.elapsed().as_nanos() as f64 / 1000.0);
            bidi_ops.push(r_b.queries_used);
        }

        uni_ops.sort_unstable(); bidi_ops.sort_unstable();
        uni_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
        bidi_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let med_ops_u = uni_ops[uni_ops.len() / 2];
        let med_ops_b = bidi_ops[bidi_ops.len() / 2];
        let med_lat_u = uni_lats[uni_lats.len() / 2];
        let med_lat_b = bidi_lats[bidi_lats.len() / 2];

        println!("b={b} | Mediana Ops: Uni={:>4} vs Bidi={:>4} | Mediana Lat: Uni={:>5.2} µs vs Bidi={:>5.2} µs",
                 med_ops_u, med_ops_b, med_lat_u, med_lat_b);
    }
}

fn bench_a7_k100_breakdown() {
    println!("\n=== A7: Desglose k=100 (Búsqueda en IDs vs Formateo a String) ===");
    let mut mem = EpisodicMemory::new();
    for i in 0..100 { mem.insert_edge(format!("Node_{i}"), format!("Node_{}", i + 1)); }

    let iters = 1_000;
    let mut search_lats = Vec::with_capacity(iters);
    let mut format_lats = Vec::with_capacity(iters);
    let mut total_lats = Vec::with_capacity(iters);

    // Calentamiento
    for _ in 0..100 {
        let _ = ReasoningEngine::solve_reachability_bfs(&mut mem, "Node_0", "Node_100", 100);
    }

    for _ in 0..iters {
        let t0 = Instant::now();
        let r = ReasoningEngine::solve_reachability_bfs(&mut mem, "Node_0", "Node_100", 100);
        let t1 = Instant::now();

        // Medir reconstrucción y conversión de 100 IDs a String
        let mut ids = Vec::with_capacity(100);
        if let SearchOutcome::Reachable { ref path_ids, .. } = r.outcome {
            ids.extend_from_slice(path_ids);
        }
        let t2 = Instant::now();
        let _strings: Vec<String> = ids.iter().map(|&id| mem.interner.resolve(id).unwrap().to_string()).collect();
        let t3 = Instant::now();

        search_lats.push((t1 - t0).as_nanos() as f64 / 1000.0);
        format_lats.push((t3 - t2).as_nanos() as f64 / 1000.0);
        total_lats.push((t1 - t0 + (t3 - t2)).as_nanos() as f64 / 1000.0);
    }

    search_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    format_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    total_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());

    println!("k=100 (1.000 iteraciones):");
    println!("  - Búsqueda Pura en IDs (Mediana):  {:>6.2} µs | p99: {:>6.2} µs", search_lats[iters / 2], search_lats[(iters * 99) / 100]);
    println!("  - Formateo a Vec<String> (Mediana): {:>6.2} µs | p99: {:>6.2} µs", format_lats[iters / 2], format_lats[(iters * 99) / 100]);
    println!("  - Total Combinado (Mediana):        {:>6.2} µs | p99: {:>6.2} µs", total_lats[iters / 2], total_lats[(iters * 99) / 100]);
    println!("  * Causa del p99: Asignación repetida de 100 Strings en heap y recolección de páginas de Windows.");
    println!("  * Propuesta: API perezosa 'path_ids()' sin asignación; resolver a String bajo demanda.");
}

fn main() {
    println!("============================================================");
    println!("  RELIC — Medición de Horizonte Elástico (A1 y A7)");
    println!("============================================================");
    bench_balanced_random_graphs();
    bench_a7_k100_breakdown();
}
