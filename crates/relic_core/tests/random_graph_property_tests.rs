//! # random_graph_property_tests
//!
//! Prueba formal de propiedades sobre 1.000 grafos aleatorios (Erdős-Rényi y dirigidos).
//! Comprueba optimalidad estricta contra BFS de libro y verificación de certificados.
//! Menos de 180 líneas.

use std::collections::{HashSet, VecDeque};
use relic_core::{
    verify_positive, verify_unreachable, EpisodicMemory, ReasoningEngine, SearchOutcome,
};

fn textbook_bfs(
    memory: &EpisodicMemory, start: &str, target: &str, avoid_nodes: &[&str],
) -> Option<usize> {
    let (start_id, target_id) = (memory.interner.get_id(start)?, memory.interner.get_id(target)?);
    let avoid_set: HashSet<u32> = avoid_nodes.iter().filter_map(|&a| memory.interner.get_id(a)).collect();
    if avoid_set.contains(&start_id) || avoid_set.contains(&target_id) { return None; }
    if start_id == target_id { return Some(0); }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start_id);
    queue.push_back((start_id, 0usize));

    while let Some((curr, dist)) = queue.pop_front() {
        for &nbr in memory.get_neighbors_u32(curr) {
            if avoid_set.contains(&nbr) { continue; }
            if nbr == target_id { return Some(dist + 1); }
            if visited.insert(nbr) {
                queue.push_back((nbr, dist + 1));
            }
        }
    }
    None
}

#[test]
fn test_property_optimality_and_certificates_over_1000_random_graphs() {
    let mut rng = 987654321u64;
    let mut next_rand = |max: usize| -> usize {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((rng >> 32) as usize) % max
    };

    let total_graphs = 1_000;
    let mut positive_cases = 0;
    let mut negative_cases = 0;

    for g_idx in 0..total_graphs {
        let mut mem = EpisodicMemory::new();
        let num_nodes = 12 + next_rand(20); // 12 a 31 nodos
        let num_edges = 15 + next_rand(35); // 15 a 49 aristas

        let node_names: Vec<String> = (0..num_nodes).map(|i| format!("G{g_idx}_N{i}")).collect();
        for name in &node_names {
            mem.interner.intern(name);
        }
        for _ in 0..num_edges {
            let u = next_rand(num_nodes);
            let v = next_rand(num_nodes);
            if u != v {
                mem.insert_edge(&node_names[u], &node_names[v]);
            }
        }

        let s_idx = next_rand(num_nodes);
        let t_idx = next_rand(num_nodes);
        let (s_node, t_node) = (&node_names[s_idx], &node_names[t_idx]);

        let has_avoid = next_rand(2) == 1;
        let mut avoid_strs = Vec::new();
        if has_avoid {
            let avoid_idx = next_rand(num_nodes);
            avoid_strs.push(node_names[avoid_idx].as_str());
        }

        let expected_dist = textbook_bfs(&mem, s_node, t_node, &avoid_strs);
        let res = ReasoningEngine::solve_reachability_bfs_constrained(
            &mut mem, s_node, t_node, 100, &avoid_strs,
        );

        match expected_dist {
            Some(d) => {
                positive_cases += 1;
                assert!(res.is_reachable, "Graph {g_idx}: Se esperaba alcanzable");
                assert_eq!(res.path_length, d, "Graph {g_idx}: Longitud de ruta óptima no coincide con BFS de libro");
                assert!(verify_positive(&mut mem, &res.path, &avoid_strs).is_ok(), "Graph {g_idx}: verify_positive falló");
            }
            None => {
                negative_cases += 1;
                assert!(!res.is_reachable, "Graph {g_idx}: Se esperaba inalcanzable");
                if let SearchOutcome::Unreachable { ref closed_set, .. } = res.outcome {
                    if let Err(e) = verify_unreachable(&mem, s_node, t_node, closed_set, &avoid_strs) {
                        panic!("Graph {g_idx}: verify_unreachable falló con err='{e}', s={s_node}, t={t_node}, avoid={avoid_strs:?}, S={closed_set:?}");
                    }
                } else {
                    panic!("Graph {g_idx}: Se esperaba certificado Unreachable");
                }
            }
        }
    }

    assert!(positive_cases > 200, "Debe haber casos positivos representativos");
    assert!(negative_cases > 200, "Debe haber casos negativos representativos");
    println!("  [OK] 1.000 grafos evaluados: {positive_cases} positivos (óptimos), {negative_cases} inalcanzables (certificados).");
}
