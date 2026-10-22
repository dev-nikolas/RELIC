//! Motor de razonamiento algorítmico determinista (BFS sobre grafos y reglas). Menos de 180 líneas.

use std::collections::{HashSet, VecDeque};
use thiserror::Error;
use crate::episodic_memory::EpisodicMemory;

/// Errores derivados de la ejecución y composición de reglas.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuleError {
    #[error("La cadena de entrada no puede estar vacía")]
    EmptyInput,
    #[error("La longitud de la cadena ({0}) supera el límite permitido ({1})")]
    InputTooLong(usize, usize),
    #[error("La profundidad de composición ({0}) supera el límite máximo permitido ({1})")]
    MaxDepthExceeded(usize, usize),
}

/// Resultado de una consulta de alcanzabilidad en grafos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachabilityResult {
    pub is_reachable: bool,
    pub path_length: usize,
    pub path: Vec<String>,
    pub queries_used: usize,
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
            return ReachabilityResult {
                is_reachable: false,
                path_length: 0,
                path: Vec::new(),
                queries_used: 0,
            };
        }

        if start == target {
            return ReachabilityResult {
                is_reachable: true,
                path_length: 0,
                path: vec![start.to_string()],
                queries_used: 0,
            };
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();

        visited.insert(start.to_string());
        queue.push_back((start.to_string(), vec![start.to_string()]));

        while let Some((curr, path)) = queue.pop_front() {
            if path.len() > max_depth {
                continue;
            }

            let neighbors = memory.get_neighbors(&curr).to_vec();

            for neighbor in neighbors {
                if avoid_set.contains(neighbor.as_str()) {
                    continue;
                }

                if neighbor == target {
                    let mut full_path = path;
                    full_path.push(neighbor);
                    let len = full_path.len() - 1;
                    return ReachabilityResult {
                        is_reachable: true,
                        path_length: len,
                        path: full_path,
                        queries_used: memory.total_queries() - initial_queries,
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
        }
    }
}

/// Operadores atómicos discretos para transformación de cadenas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtomicOperator { Reverse, ShiftLeft, SwapCase, DuplicateVowels, RemoveFirstLast }

/// Catálogo de reglas y ejecutor composicional seguro.
pub struct RuleCatalog { max_input_length: usize, max_composition_depth: usize }

impl Default for RuleCatalog {
    fn default() -> Self { Self { max_input_length: 1024, max_composition_depth: 8 } }
}

impl RuleCatalog {
    pub fn new(max_input_length: usize, max_composition_depth: usize) -> Self {
        Self { max_input_length, max_composition_depth }
    }

    /// Aplica un único operador atómico con validación estricta de entrada.
    pub fn apply_operator(&self, input: &str, op: AtomicOperator) -> Result<String, RuleError> {
        if input.is_empty() { return Err(RuleError::EmptyInput); }
        if input.len() > self.max_input_length { return Err(RuleError::InputTooLong(input.len(), self.max_input_length)); }

        let result = match op {
            AtomicOperator::Reverse => input.chars().rev().collect(),
            AtomicOperator::ShiftLeft => {
                let mut chars = input.chars();
                if let Some(first) = chars.next() {
                    let mut s: String = chars.collect();
                    s.push(first);
                    s
                } else { input.to_string() }
            }
            AtomicOperator::SwapCase => input.chars().map(|c| {
                if c.is_lowercase() { c.to_uppercase().collect::<String>() } else { c.to_lowercase().collect::<String>() }
            }).collect(),
            AtomicOperator::DuplicateVowels => {
                let mut s = String::with_capacity(input.len() * 2);
                for c in input.chars() {
                    s.push(c);
                    if matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u') { s.push(c); }
                }
                s
            }
            AtomicOperator::RemoveFirstLast => {
                let chars: Vec<char> = input.chars().collect();
                if chars.len() <= 2 { String::new() } else { chars[1..chars.len() - 1].iter().collect() }
            }
        };
        Ok(result)
    }

    /// Compone una secuencia de operadores atómicos con límite de profundidad.
    pub fn compose(&self, input: &str, ops: &[AtomicOperator]) -> Result<String, RuleError> {
        if ops.len() > self.max_composition_depth {
            return Err(RuleError::MaxDepthExceeded(ops.len(), self.max_composition_depth));
        }
        let mut current = input.to_string();
        for &op in ops { current = self.apply_operator(&current, op)?; }
        Ok(current)
    }
}
