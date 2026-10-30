//! # certificate
//!
//! Certificados formales de alcanzabilidad y verificadores independientes O(k) y O(deg).
use std::collections::HashSet;
use crate::episodic_memory::EpisodicMemory;

/// Certificado formal del resultado de una búsqueda en grafos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchOutcome {
    Reachable { path: Vec<String>, path_ids: Vec<u32> },
    Unreachable { closed_set: Vec<String>, closed_set_ids: Vec<u32> },
    BudgetExceeded {
        levels_forward: usize, levels_backward: usize, frontier_size: usize,
        estimated_next_cost: usize, operations_spent: usize,
    },
}

/// Resultado de una consulta de alcanzabilidad en grafos con certificado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachabilityResult {
    pub is_reachable: bool,
    pub path_length: usize,
    pub path: Vec<String>,
    pub queries_used: usize,
    pub depth_exhausted: bool,
    pub outcome: SearchOutcome,
}

impl ReachabilityResult {
    #[inline]
    pub fn path_ids(&self) -> &[u32] {
        match &self.outcome {
            SearchOutcome::Reachable { path_ids, .. } => path_ids,
            _ => &[],
        }
    }

    #[inline]
    pub fn closed_set_ids(&self) -> &[u32] {
        match &self.outcome {
            SearchOutcome::Unreachable { closed_set_ids, .. } => closed_set_ids,
            _ => &[],
        }
    }
}

/// Verifica independientemente un camino positivo en O(k).
pub fn verify_positive(memory: &mut EpisodicMemory, path: &[String], avoid_nodes: &[&str]) -> Result<(), String> {
    if path.is_empty() { return Err("Camino vacío".to_string()); }
    let avoid_set: HashSet<&str> = avoid_nodes.iter().copied().collect();
    for node in path {
        if avoid_set.contains(node.as_str()) { return Err(format!("Camino contiene nodo prohibido '{node}'")); }
    }
    if path.len() == 1 { return Ok(()); }
    for window in path.windows(2) {
        let (u, v) = (&window[0], &window[1]);
        if !memory.has_edge(u, v) { return Err(format!("Arista inexistente: '{u}' -> '{v}'")); }
    }
    Ok(())
}

/// Verifica independientemente un certificado negativo forward en O(Σ out-grado(u)).
pub fn verify_negative(
    memory: &EpisodicMemory, start: &str, target: &str, closed_set: &[String], avoid_nodes: &[&str],
) -> Result<(), String> {
    let start_id = memory.interner.get_id(start).ok_or_else(|| "Nodo origen no existe".to_string())?;
    let target_id = memory.interner.get_id(target).ok_or_else(|| "Nodo destino no existe".to_string())?;
    let avoid_ids: HashSet<u32> = avoid_nodes.iter().filter_map(|&a| memory.interner.get_id(a)).collect();
    if avoid_ids.contains(&start_id) || avoid_ids.contains(&target_id) { return Ok(()); }

    let s_ids: HashSet<u32> = closed_set.iter().filter_map(|n| memory.interner.get_id(n)).collect();
    if !s_ids.contains(&start_id) { return Err("S no contiene el nodo origen s".to_string()); }
    if s_ids.contains(&target_id) { return Err("S contiene el nodo destino t".to_string()); }

    for &u in &s_ids {
        for &v in memory.get_neighbors_u32(u) {
            if !s_ids.contains(&v) && !avoid_ids.contains(&v) {
                let un = memory.interner.resolve(u).unwrap_or("?");
                let vn = memory.interner.resolve(v).unwrap_or("?");
                return Err(format!("Fuga de clausura: {un} -> {vn}, pero {vn} no está en S ni en avoid"));
            }
        }
    }
    Ok(())
}

/// Verifica independientemente un certificado negativo backward en O(Σ in-grado(v)).
pub fn verify_negative_backward(
    memory: &EpisodicMemory, start: &str, target: &str, closed_set_t: &[String], avoid_nodes: &[&str],
) -> Result<(), String> {
    let start_id = memory.interner.get_id(start).ok_or_else(|| "Nodo origen no existe".to_string())?;
    let target_id = memory.interner.get_id(target).ok_or_else(|| "Nodo destino no existe".to_string())?;
    let avoid_ids: HashSet<u32> = avoid_nodes.iter().filter_map(|&a| memory.interner.get_id(a)).collect();
    if avoid_ids.contains(&start_id) || avoid_ids.contains(&target_id) { return Ok(()); }

    let t_ids: HashSet<u32> = closed_set_t.iter().filter_map(|n| memory.interner.get_id(n)).collect();
    if !t_ids.contains(&target_id) { return Err("T no contiene el nodo destino t".to_string()); }
    if t_ids.contains(&start_id) { return Err("T contiene el nodo origen s".to_string()); }

    for &v in &t_ids {
        for &u in memory.get_predecessors_u32(v) {
            if !t_ids.contains(&u) && !avoid_ids.contains(&u) {
                let un = memory.interner.resolve(u).unwrap_or("?");
                let vn = memory.interner.resolve(v).unwrap_or("?");
                return Err(format!("Fuga backward: {un} es predecesor de {vn}, no está en T ni avoid"));
            }
        }
    }
    Ok(())
}

/// Verifica automáticamente el certificado negativo sin importar de qué lado se agotó la búsqueda.
pub fn verify_unreachable(
    memory: &EpisodicMemory, start: &str, target: &str, closed_set: &[String], avoid_nodes: &[&str],
) -> Result<(), String> {
    if closed_set.iter().any(|s| s == start) {
        verify_negative(memory, start, target, closed_set, avoid_nodes)
    } else if closed_set.iter().any(|s| s == target) {
        verify_negative_backward(memory, start, target, closed_set, avoid_nodes)
    } else {
        Err("El conjunto cerrado no contiene ni al origen s ni al destino t".to_string())
    }
}
