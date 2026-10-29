//! # ms_bfs
//!
//! Motor MS-BFS (Multi-Source Breadth-First Search) con evaluacion paralela
//! bitwise u64 para hasta 64 consultas simultaneas o 64 mundos contrafactuales.
//! Menos de 180 lineas.

use std::cell::RefCell;
use crate::episodic_memory::EpisodicMemory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsBfsResult {
    pub reachable_mask: u64,
    pub distances: [Option<u32>; 64],
    pub operations: usize,
}

impl MsBfsResult {
    pub fn is_reachable(&self, index: usize) -> bool {
        if index >= 64 { false } else { (self.reachable_mask & (1u64 << index)) != 0 }
    }
}

#[derive(Default)]
pub struct MsBfsBuffers {
    pub visited: Vec<u64>,
    pub frontier: Vec<u64>,
    pub next_frontier: Vec<u64>,
    pub touched_visited: Vec<u32>,
    pub active_nodes: Vec<u32>,
    pub next_active_nodes: Vec<u32>,
}

impl MsBfsBuffers {
    pub fn prepare(&mut self, n: usize) {
        if self.visited.len() < n { self.visited.resize(n, 0); }
        if self.frontier.len() < n { self.frontier.resize(n, 0); }
        if self.next_frontier.len() < n { self.next_frontier.resize(n, 0); }
        self.touched_visited.clear();
        self.active_nodes.clear();
        self.next_active_nodes.clear();
    }

    pub fn reset(&mut self) {
        for &id in &self.touched_visited {
            let idx = id as usize;
            if let Some(v) = self.visited.get_mut(idx) { *v = 0; }
            if let Some(f) = self.frontier.get_mut(idx) { *f = 0; }
            if let Some(nf) = self.next_frontier.get_mut(idx) { *nf = 0; }
        }
        self.touched_visited.clear();
        self.active_nodes.clear();
        self.next_active_nodes.clear();
    }
}

thread_local! {
    pub static MS_BUFFERS: RefCell<MsBfsBuffers> = RefCell::new(MsBfsBuffers::default());
}

pub fn solve_ms_bfs_batch(memory: &EpisodicMemory, queries: &[(u32, u32)]) -> MsBfsResult {
    let k = queries.len().min(64);
    if k == 0 { return MsBfsResult { reachable_mask: 0, distances: [None; 64], operations: 0 }; }
    let mut reachable_mask = 0u64;
    let mut unreached_mask = if k == 64 { !0u64 } else { (1u64 << k) - 1 };
    let mut distances = [None; 64];

    for (i, &(s, t)) in queries.iter().enumerate().take(k) {
        if s == t {
            let bit = 1u64 << i;
            reachable_mask |= bit; distances[i] = Some(0); unreached_mask &= !bit;
        }
    }
    if unreached_mask == 0 { return MsBfsResult { reachable_mask, distances, operations: 0 }; }

    let n = memory.interner.len();
    MS_BUFFERS.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.prepare(n);

        for (i, &(s_id, _)) in queries.iter().enumerate().take(k) {
            let bit = 1u64 << i;
            if (unreached_mask & bit) != 0 {
                let s = s_id as usize;
                if s < n {
                    if buf.frontier[s] == 0 { buf.active_nodes.push(s_id); }
                    if buf.visited[s] == 0 { buf.touched_visited.push(s_id); }
                    buf.frontier[s] |= bit;
                    buf.visited[s] |= bit;
                }
            }
        }

        let mut current_hop = 0u32;
        let mut operations = 0usize;

        while !buf.active_nodes.is_empty() && unreached_mask != 0 {
            current_hop += 1;
            for idx in 0..buf.active_nodes.len() {
                let u = buf.active_nodes[idx];
                let curr_mask = buf.frontier[u as usize];
                if curr_mask == 0 { continue; }
                let neighbors = memory.get_neighbors_u32(u);
                operations += neighbors.len();
                for &v in neighbors {
                    let v_idx = v as usize;
                    if v_idx >= n { continue; }
                    let edge_mask = memory.get_edge_world_mask_u32(u, v);
                    let passed = curr_mask & edge_mask;
                    let new_bits = passed & !buf.visited[v_idx];
                    if new_bits != 0 {
                        if buf.next_frontier[v_idx] == 0 { buf.next_active_nodes.push(v); }
                        if buf.visited[v_idx] == 0 { buf.touched_visited.push(v); }
                        buf.next_frontier[v_idx] |= new_bits;
                        buf.visited[v_idx] |= new_bits;
                    }
                }
                buf.frontier[u as usize] = 0;
            }

            let mut newly_reached = 0u64;
            for (i, &(_, t_id)) in queries.iter().enumerate().take(k) {
                let bit = 1u64 << i;
                if (unreached_mask & bit) != 0 {
                    let t = t_id as usize;
                    if t < n && (buf.visited[t] & bit) != 0 {
                        reachable_mask |= bit; distances[i] = Some(current_hop); newly_reached |= bit;
                    }
                }
            }
            unreached_mask &= !newly_reached;

            buf.active_nodes.clear();
            for idx in 0..buf.next_active_nodes.len() {
                let v = buf.next_active_nodes[idx];
                let v_idx = v as usize;
                let pruned = buf.next_frontier[v_idx] & unreached_mask;
                buf.next_frontier[v_idx] = 0;
                if pruned != 0 { buf.frontier[v_idx] = pruned; buf.active_nodes.push(v); }
            }
            buf.next_active_nodes.clear();
        }

        buf.reset();
        MsBfsResult { reachable_mask, distances, operations }
    })
}

pub fn solve_ms_bfs_counterfactual(
    memory: &EpisodicMemory, start: u32, target: u32, world_count: usize,
) -> MsBfsResult {
    let k = world_count.min(64);
    let queries: Vec<(u32, u32)> = (0..k).map(|_| (start, target)).collect();
    solve_ms_bfs_batch(memory, &queries)
}
