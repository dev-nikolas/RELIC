//! # ms_bfs
//!
//! Motor MS-BFS con evaluacion paralela bitwise SIMD para 256 mundos contrafactuales.
//! Menos de 180 lineas.

use std::cell::RefCell;
use crate::episodic_memory::EpisodicMemory;
pub use crate::mask256::Mask256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsBfsResult {
    pub reachable_mask: u64,
    pub mask256: Mask256,
    pub distances: [Option<u32>; 256],
    pub operations: usize,
}

impl MsBfsResult {
    pub fn is_reachable(&self, index: usize) -> bool { self.mask256.get_bit(index) }
}

#[derive(Default)]
pub struct MsBfsBuffers {
    pub visited: Vec<Mask256>,
    pub frontier: Vec<Mask256>,
    pub next_frontier: Vec<Mask256>,
    pub touched_visited: Vec<u32>,
    pub active_nodes: Vec<u32>,
    pub next_active_nodes: Vec<u32>,
}

impl MsBfsBuffers {
    pub fn prepare(&mut self, n: usize) {
        if self.visited.len() < n { self.visited.resize(n, Mask256::ZERO); }
        if self.frontier.len() < n { self.frontier.resize(n, Mask256::ZERO); }
        if self.next_frontier.len() < n { self.next_frontier.resize(n, Mask256::ZERO); }
        self.touched_visited.clear(); self.active_nodes.clear(); self.next_active_nodes.clear();
    }

    pub fn reset(&mut self) {
        for &id in &self.touched_visited {
            let idx = id as usize;
            if let Some(v) = self.visited.get_mut(idx) { *v = Mask256::ZERO; }
            if let Some(f) = self.frontier.get_mut(idx) { *f = Mask256::ZERO; }
            if let Some(nf) = self.next_frontier.get_mut(idx) { *nf = Mask256::ZERO; }
        }
        self.touched_visited.clear(); self.active_nodes.clear(); self.next_active_nodes.clear();
    }
}

thread_local! { pub static MS_BUFFERS: RefCell<MsBfsBuffers> = RefCell::new(MsBfsBuffers::default()); }

pub fn solve_ms_bfs_batch(memory: &EpisodicMemory, queries: &[(u32, u32)]) -> MsBfsResult {
    let k = queries.len().min(256);
    if k == 0 { return MsBfsResult { reachable_mask: 0, mask256: Mask256::ZERO, distances: [None; 256], operations: 0 }; }
    let mut mask256 = Mask256::ZERO;
    let mut unreached = Mask256::ones(k);
    let mut distances = [None; 256];

    for (i, &(s, t)) in queries.iter().enumerate().take(k) {
        if s == t {
            let bit = Mask256::from_bit(i);
            mask256 |= bit; distances[i] = Some(0); unreached &= !bit;
        }
    }
    if unreached.is_zero() {
        return MsBfsResult { reachable_mask: mask256.0[0], mask256, distances, operations: 0 };
    }

    let is_uniform = queries.iter().take(k).all(|&(s, t)| s == queries[0].0 && t == queries[0].1);
    let n = memory.interner.len();

    MS_BUFFERS.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.prepare(n);

        if is_uniform {
            let s = queries[0].0 as usize;
            if s < n {
                buf.active_nodes.push(queries[0].0); buf.touched_visited.push(queries[0].0);
                buf.frontier[s] = unreached; buf.visited[s] = unreached;
            }
        } else {
            for (i, &(s_id, _)) in queries.iter().enumerate().take(k) {
                let bit = Mask256::from_bit(i);
                if !(unreached & bit).is_zero() {
                    let s = s_id as usize;
                    if s < n {
                        if buf.frontier[s].is_zero() { buf.active_nodes.push(s_id); }
                        if buf.visited[s].is_zero() { buf.touched_visited.push(s_id); }
                        buf.frontier[s] |= bit; buf.visited[s] |= bit;
                    }
                }
            }
        }

        let mut current_hop = 0u32;
        let mut operations = 0usize;

        while !buf.active_nodes.is_empty() && !unreached.is_zero() {
            current_hop += 1;
            for idx in 0..buf.active_nodes.len() {
                let u = buf.active_nodes[idx];
                let curr_mask = buf.frontier[u as usize];
                if curr_mask.is_zero() { continue; }
                let neighbors = memory.get_neighbors_u32(u);
                operations += neighbors.len();
                for &v in neighbors {
                    let v_idx = v as usize;
                    if v_idx >= n { continue; }
                    let edge_mask = memory.get_edge_mask256(u, v);
                    let passed = curr_mask & edge_mask;
                    let new_bits = passed & !buf.visited[v_idx];
                    if !new_bits.is_zero() {
                        if buf.next_frontier[v_idx].is_zero() { buf.next_active_nodes.push(v); }
                        if buf.visited[v_idx].is_zero() { buf.touched_visited.push(v); }
                        buf.next_frontier[v_idx] |= new_bits; buf.visited[v_idx] |= new_bits;
                    }
                }
                buf.frontier[u as usize] = Mask256::ZERO;
            }

            let mut newly_reached = Mask256::ZERO;
            if is_uniform {
                let t = queries[0].1 as usize;
                if t < n {
                    let newly = buf.visited[t] & unreached;
                    if !newly.is_zero() {
                        mask256 |= newly;
                        for (w, &b) in newly.0.iter().enumerate() {
                            let mut bits = b;
                            while bits != 0 {
                                let bit_idx = w * 64 + bits.trailing_zeros() as usize;
                                if bit_idx < k { distances[bit_idx] = Some(current_hop); }
                                bits &= bits - 1;
                            }
                        }
                        newly_reached = newly;
                    }
                }
            } else {
                for (i, &(_, t_id)) in queries.iter().enumerate().take(k) {
                    let bit = Mask256::from_bit(i);
                    if !(unreached & bit).is_zero() {
                        let t = t_id as usize;
                        if t < n && !(buf.visited[t] & bit).is_zero() {
                            mask256 |= bit; distances[i] = Some(current_hop); newly_reached |= bit;
                        }
                    }
                }
            }
            unreached &= !newly_reached;

            buf.active_nodes.clear();
            for idx in 0..buf.next_active_nodes.len() {
                let v = buf.next_active_nodes[idx];
                let v_idx = v as usize;
                let pruned = buf.next_frontier[v_idx] & unreached;
                buf.next_frontier[v_idx] = Mask256::ZERO;
                if !pruned.is_zero() { buf.frontier[v_idx] = pruned; buf.active_nodes.push(v); }
            }
            buf.next_active_nodes.clear();
        }

        buf.reset();
        MsBfsResult { reachable_mask: mask256.0[0], mask256, distances, operations }
    })
}

pub fn solve_ms_bfs_counterfactual(
    memory: &EpisodicMemory, start: u32, target: u32, world_count: usize,
) -> MsBfsResult {
    let k = world_count.min(256);
    let queries: Vec<(u32, u32)> = (0..k).map(|_| (start, target)).collect();
    solve_ms_bfs_batch(memory, &queries)
}
