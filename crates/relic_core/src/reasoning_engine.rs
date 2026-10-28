//! Motor de razonamiento algorítmico determinista (BFS sobre grafos). Menos de 180 líneas.

use std::collections::{HashSet, VecDeque};
use crate::episodic_memory::EpisodicMemory;

/// Resultado de una consulta de alcanzabilidad en grafos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachabilityResult {
    pub is_reachable: bool,
    pub path_length: usize,
    pub path: Vec<String>,
    pub queries_used: usize,
    pub depth_exhausted: bool,
}

/// Motor de razonamiento determinista para consultas estructuradas.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReasoningEngine;

impl ReasoningEngine {
    /// Resuelve alcanzabilidad en grafos mediante BFS sobre la memoria episódica.
    pub fn solve_reachability_bfs(
        memory: &mut EpisodicMemory, start: &str, target: &str, max_depth: usize,
    ) -> ReachabilityResult {
        Self::solve_reachability_bfs_constrained(memory, start, target, max_depth, &[])
    }

    /// Resuelve alcanzabilidad BFS respetando un conjunto de nodos estrictamente prohibidos (avoid).
    pub fn solve_reachability_bfs_constrained(
        memory: &mut EpisodicMemory,
        start: &str,
        target: &str,
        max_depth: usize,
        avoid_nodes: &[&str],
    ) -> ReachabilityResult {
        let initial_queries = memory.total_queries();
        let avoid_set: HashSet<&str> = avoid_nodes.iter().copied().collect();

        if avoid_set.contains(start) || avoid_set.contains(target) {
            return ReachabilityResult { is_reachable: false, path_length: 0, path: Vec::new(), queries_used: 0, depth_exhausted: false };
        }

        if start == target {
            return ReachabilityResult { is_reachable: true, path_length: 0, path: vec![start.to_string()], queries_used: 0, depth_exhausted: false };
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();
        let mut depth_hit = false;

        visited.insert(start.to_string());
        queue.push_back((start.to_string(), vec![start.to_string()]));

        while let Some((curr, path)) = queue.pop_front() {
            if path.len() > max_depth {
                depth_hit = true;
                continue;
            }

            let neighbors = memory.get_neighbors(&curr).to_vec();
            for neighbor in neighbors {
                if avoid_set.contains(neighbor.as_str()) { continue; }
                if neighbor == target {
                    let mut full_path = path;
                    full_path.push(neighbor);
                    let len = full_path.len() - 1;
                    return ReachabilityResult {
                        is_reachable: true,
                        path_length: len,
                        path: full_path,
                        queries_used: memory.total_queries() - initial_queries,
                        depth_exhausted: false,
                    };
                }

                if !visited.contains(&neighbor) {
                    visited.insert(neighbor.clone());
                    let mut next_path = path.clone();
                    next_path.push(neighbor.clone());
                    queue.push_back((neighbor, next_path));
                }
            }
        }

        ReachabilityResult {
            is_reachable: false,
            path_length: 0,
            path: Vec::new(),
            queries_used: memory.total_queries() - initial_queries,
            depth_exhausted: depth_hit,
        }
    }
}

