//! Pruebas de propiedades de search_buffer: aislamiento, orden aleatorio y concurrencia.
use std::sync::Arc;
use std::thread;
use relic_core::{EpisodicMemory, ReasoningEngine};

#[test]
fn test_search_buffer_reused_vs_fresh_equivalence() {
    let mut mem = EpisodicMemory::new();
    for i in 0..50 {
        mem.insert_edge(format!("Node_{i}"), format!("Node_{}", i + 1));
        if i % 3 == 0 { mem.insert_edge(format!("Node_{i}"), format!("Branch_{i}")); }
    }

    let queries = [
        ("Node_0", "Node_10"), ("Node_5", "Node_25"), ("Node_0", "Node_40"),
        ("Node_10", "Branch_12"), ("Node_20", "Node_20"), ("Node_30", "Node_5"),
    ];

    // Consulta de referencia (orden secuencial)
    let ref_results: Vec<_> = queries.iter().map(|&(s, t)| {
        ReasoningEngine::solve_reachability_bfs(&mut mem, s, t, 30)
    }).collect();

    // 100 permutaciones aleatorias sobre el mismo buffer thread_local
    let mut rng = 123456789u64;
    for _ in 0..100 {
        let mut perm = queries.to_vec();
        for i in 0..perm.len() {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = ((rng >> 32) as usize) % perm.len();
            perm.swap(i, j);
        }
        for &(s, t) in &perm {
            let actual = ReasoningEngine::solve_reachability_bfs(&mut mem, s, t, 30);
            let expected_idx = queries.iter().position(|&q| q == (s, t)).unwrap();
            let expected = &ref_results[expected_idx];
            assert_eq!(actual.is_reachable, expected.is_reachable);
            assert_eq!(actual.path_length, expected.path_length);
            assert_eq!(actual.path, expected.path);
        }
    }
}

#[test]
fn test_concurrent_multithreaded_search_buffer_isolation() {
    let mut base_mem = EpisodicMemory::new();
    for i in 0..100 {
        base_mem.insert_edge(format!("Hub_{i}"), format!("Hub_{}", i + 1));
        for j in 0..5 {
            base_mem.insert_edge(format!("Hub_{i}"), format!("Leaf_{i}_{j}"));
        }
    }
    let shared_mem = Arc::new(base_mem);
    let mut handles = Vec::new();

    for th_id in 0..8 {
        let mem_clone = Arc::clone(&shared_mem);
        handles.push(thread::spawn(move || {
            let mut local_mem = (*mem_clone).clone();
            for it in 0..50 {
                let start = format!("Hub_{}", (th_id * 10 + it) % 90);
                let target = format!("Hub_{}", ((th_id * 10 + it) % 90) + 10);
                let res = ReasoningEngine::solve_reachability_bfs(&mut local_mem, &start, &target, 20);
                assert!(res.is_reachable);
                assert_eq!(res.path_length, 10);
            }
        }));
    }

    for h in handles {
        h.join().expect("Hilo terminó con pánico");
    }
}

