//! Tests para MS-BFS (Multi-Source Breadth-First Search) y evaluación contrafactual.
//! Menos de 180 líneas.

use relic_core::episodic_memory::EpisodicMemory;
use relic_core::ms_bfs::{solve_ms_bfs_batch, solve_ms_bfs_counterfactual};
use relic_core::reasoning_engine::ReasoningEngine;

#[test]
fn test_ms_bfs_batch_matches_sequential() {
    let mut memory = EpisodicMemory::new();
    // Build a graph with 30 nodes and various paths
    for i in 0..25 {
        memory.insert_edge(format!("N{}", i), format!("N{}", i + 1));
        if i % 3 == 0 && i + 4 < 30 {
            memory.insert_edge(format!("N{}", i), format!("N{}", i + 4));
        }
        if i % 5 == 0 && i + 7 < 30 {
            memory.insert_edge(format!("N{}", i), format!("N{}", i + 7));
        }
    }
    // Add some disconnected nodes
    for i in 26..30 {
        memory.insert_edge(format!("N{}", i), format!("N{}", (i + 1) % 30));
    }

    let mut queries = Vec::new();
    let mut expected_reachable = Vec::new();

    for i in 0..64 {
        let s_idx = (i * 7) % 30;
        let t_idx = (i * 13 + 3) % 30;
        let s_name = format!("N{}", s_idx);
        let t_name = format!("N{}", t_idx);

        let s_id = memory.interner.intern(&s_name);
        let t_id = memory.interner.intern(&t_name);
        queries.push((s_id, t_id));

        let res = ReasoningEngine::solve_reachability_bfs(&mut memory, &s_name, &t_name, 50);
        expected_reachable.push(res.is_reachable);
    }

    let batch_res = solve_ms_bfs_batch(&memory, &queries);

    for (i, &expected) in expected_reachable.iter().enumerate() {
        assert_eq!(
            batch_res.is_reachable(i),
            expected,
            "Query {} (N{} -> N{}) mismatch: expected {}, got {}",
            i,
            queries[i].0,
            queries[i].1,
            expected,
            batch_res.is_reachable(i)
        );
    }
}

#[test]
fn test_ms_bfs_counterfactual_worlds_evaluation() {
    let mut memory = EpisodicMemory::new();
    memory.insert_edge("A", "B");
    memory.insert_edge("B", "C");
    memory.insert_edge("C", "D");
    memory.insert_edge("A", "X");
    memory.insert_edge("X", "D");

    // In world 0..31: edge B->C exists, edge X->D is disabled
    // In world 32..63: edge B->C is disabled, edge X->D exists
    let mask_bc: u64 = 0x0000_0000_FFFF_FFFF; // bits 0..31 set
    let mask_xd: u64 = 0xFFFF_FFFF_0000_0000; // bits 32..63 set

    memory.set_edge_world_mask("B", "C", mask_bc);
    memory.set_edge_world_mask("X", "D", mask_xd);

    let a_id = memory.interner.get_id("A").unwrap();
    let d_id = memory.interner.get_id("D").unwrap();

    let res = solve_ms_bfs_counterfactual(&memory, a_id, d_id, 64);

    // In all 64 worlds, D should be reachable:
    // in 0..31 via A->B->C->D, in 32..63 via A->X->D
    assert_eq!(res.reachable_mask, !0u64);

    // Now disable A->X completely (mask = 0)
    memory.set_edge_world_mask("A", "X", 0);
    let res2 = solve_ms_bfs_counterfactual(&memory, a_id, d_id, 64);
    // Now only worlds 0..31 can reach D via B->C!
    assert_eq!(res2.reachable_mask, mask_bc);
    for w in 0..64 {
        if w < 32 {
            assert!(res2.is_reachable(w));
            assert_eq!(res2.distances[w], Some(3));
        } else {
            assert!(!res2.is_reachable(w));
            assert_eq!(res2.distances[w], None);
        }
    }
}

#[test]
fn test_ms_bfs_edge_cases() {
    let mut memory = EpisodicMemory::new();
    memory.insert_edge("Alpha", "Beta");
    let alpha = memory.interner.get_id("Alpha").unwrap();
    let beta = memory.interner.get_id("Beta").unwrap();

    // 1. Empty batch
    let empty_res = solve_ms_bfs_batch(&memory, &[]);
    assert_eq!(empty_res.reachable_mask, 0);

    // 2. Self loop (start == target)
    let self_res = solve_ms_bfs_batch(&memory, &[(alpha, alpha)]);
    assert!(self_res.is_reachable(0));
    assert_eq!(self_res.distances[0], Some(0));

    // 3. Batch smaller than 64
    let small_res = solve_ms_bfs_batch(&memory, &[(alpha, beta), (beta, alpha)]);
    assert!(small_res.is_reachable(0));
    assert_eq!(small_res.distances[0], Some(1));
    assert!(!small_res.is_reachable(1));
    assert_eq!(small_res.distances[1], None);
    assert_eq!(small_res.reachable_mask & !3u64, 0);
}

#[test]
fn test_ms_bfs_256_worlds_simd() {
    let mut memory = EpisodicMemory::new();
    memory.insert_edge("S", "MidA");
    memory.insert_edge("MidA", "T");
    memory.insert_edge("S", "MidB");
    memory.insert_edge("MidB", "T");

    let even_mask = [0x5555_5555_5555_5555u64; 4];
    let odd_mask = [0xAAAA_AAAA_AAAA_AAAAu64; 4];
    memory.set_edge_world_mask_256("MidA", "T", even_mask);
    memory.set_edge_world_mask_256("MidB", "T", odd_mask);

    let s = memory.interner.get_id("S").unwrap();
    let t = memory.interner.get_id("T").unwrap();
    let res = solve_ms_bfs_counterfactual(&memory, s, t, 256);
    for w in 0..256 {
        assert!(res.is_reachable(w));
        assert_eq!(res.distances[w], Some(2));
    }

    memory.set_edge_world_mask_256("MidB", "T", [0; 4]);
    let res2 = solve_ms_bfs_counterfactual(&memory, s, t, 256);
    for w in 0..256 {
        if w % 2 == 0 { assert!(res2.is_reachable(w)); } else { assert!(!res2.is_reachable(w)); }
    }
}


