//! Gate de Fase 6: el índice coincide con BFS exhaustivo.
//!
//! - 120 grafos aleatorios dirigidos (2..=23 nodos): todas las parejas.
//! - Un grafo fijo contrastado contra `ReasoningEngine` (motor ya en producción).
//! - Una cadena de 500 nodos, justo por debajo del límite de 512 componentes.

use relic_core::graph_index::{ReachHierarchy, ReachVerdict};
use relic_core::{EpisodicMemory, ReasoningEngine};

fn next(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn bfs(mem: &EpisodicMemory, n: usize, start: u32) -> Vec<bool> {
    let mut seen = vec![false; n];
    let mut queue = vec![start];
    seen[start as usize] = true;
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        for &v in mem.get_neighbors_u32(u) {
            let vi = v as usize;
            if vi < n && !seen[vi] {
                seen[vi] = true;
                queue.push(v);
            }
        }
    }
    seen
}

fn verdict(index: &ReachHierarchy, u: u32, v: u32) -> bool {
    match index.reaches(u, v) {
        ReachVerdict::Reachable => true,
        ReachVerdict::Unreachable => false,
        ReachVerdict::OutOfRange => panic!("{u} -> {v} fuera del índice"),
    }
}

#[test]
fn matches_exhaustive_bfs_on_random_digraphs() {
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    for graph in 0..120 {
        let n = 2 + (next(&mut seed) % 22) as usize;
        let names: Vec<String> = (0..n).map(|i| format!("v{i}")).collect();
        let mut mem = EpisodicMemory::new();
        for name in &names {
            mem.interner.intern(name);
        }
        for u in 0..n {
            for v in 0..n {
                if next(&mut seed) % 8 == 0 {
                    mem.insert_edge(names[u].as_str(), names[v].as_str());
                }
            }
        }
        let index = ReachHierarchy::build(&mem).expect("grafo dentro de capacidad");
        for u in 0..n {
            let truth = bfs(&mem, n, u as u32);
            for v in 0..n {
                let expected = truth[v];
                let actual = verdict(&index, u as u32, v as u32);
                assert_eq!(
                    actual, expected,
                    "grafo {graph}: {u} -> {v} (esperado {expected}, índice {actual})"
                );
            }
        }
    }
}

#[test]
fn agrees_with_existing_engine_on_fixed_graph() {
    let edges = [
        ("a", "b"), ("b", "c"), ("c", "a"), ("c", "d"), ("d", "e"),
        ("e", "f"), ("f", "e"), ("b", "g"), ("g", "h"), ("h", "i"),
    ];
    let mut mem = EpisodicMemory::new();
    for (u, v) in edges {
        mem.insert_edge(u, v);
    }
    let index = ReachHierarchy::build(&mem).unwrap();
    let names = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
    for (u, from) in names.iter().enumerate() {
        for (v, to) in names.iter().enumerate() {
            let engine = ReasoningEngine::solve_reachability_bidirectional(
                &mut mem, from, to, None, &[], 100_000,
            )
            .is_reachable;
            assert_eq!(
                verdict(&index, u as u32, v as u32),
                engine,
                "{from} -> {to}: índice y motor discrepan"
            );
        }
    }
}

#[test]
fn long_chain_below_component_limit_is_exact() {
    let n = 500usize;
    let mut mem = EpisodicMemory::new();
    for i in 0..n - 1 {
        mem.insert_edge(format!("n{i}"), format!("n{}", i + 1));
    }
    let index = ReachHierarchy::build(&mem).unwrap();
    assert_eq!(index.node_count(), n);
    assert_eq!(index.component_count(), n);
    for u in [0u32, 1, 249, 498, 499] {
        for v in [0u32, 1, 250, 499] {
            assert_eq!(
                verdict(&index, u, v),
                u <= v,
                "cadena: {u} -> {v}"
            );
        }
    }
}
