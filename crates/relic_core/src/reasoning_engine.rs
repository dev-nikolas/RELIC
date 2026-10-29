//! Motor de razonamiento algorítmico determinista (BFS Bidireccional de Horizonte Elástico).
//! Menos de 180 líneas.

use crate::certificate::{ReachabilityResult, SearchOutcome};
use crate::episodic_memory::EpisodicMemory;
use crate::search_buffer::BUFFERS;

#[derive(Debug, Default, Clone, Copy)]
pub struct ReasoningEngine;

impl ReasoningEngine {
    pub fn default_budget(memory: &EpisodicMemory) -> usize {
        let size = memory.total_node_count() + memory.edge_count();
        (size * 4).max(10_000)
    }

    pub fn solve_ms_bfs_batch(mem: &EpisodicMemory, q: &[(u32, u32)]) -> crate::ms_bfs::MsBfsResult {
        crate::ms_bfs::solve_ms_bfs_batch(mem, q)
    }

    pub fn solve_ms_bfs_counterfactual(mem: &EpisodicMemory, s: u32, t: u32, w: usize) -> crate::ms_bfs::MsBfsResult {
        crate::ms_bfs::solve_ms_bfs_counterfactual(mem, s, t, w)
    }

    pub fn solve_reachability_bfs(
        memory: &mut EpisodicMemory, start: &str, target: &str, max_depth: usize,
    ) -> ReachabilityResult {
        let budget = Self::default_budget(memory);
        Self::solve_reachability_bidirectional(memory, start, target, Some(max_depth), &[], budget)
    }

    pub fn solve_reachability_bfs_constrained(
        memory: &mut EpisodicMemory, start: &str, target: &str, max_depth: usize, avoid: &[&str],
    ) -> ReachabilityResult {
        let budget = Self::default_budget(memory);
        Self::solve_reachability_bidirectional(memory, start, target, Some(max_depth), avoid, budget)
    }

    pub fn solve_reachability_bidirectional(
        memory: &mut EpisodicMemory, start: &str, target: &str,
        max_hops: Option<usize>, avoid_nodes: &[&str], budget_ops: usize,
    ) -> ReachabilityResult {
        let (Some(s_id), Some(t_id)) = (memory.interner.get_id(start), memory.interner.get_id(target)) else {
            return Self::unreachable_res(Vec::new(), Vec::new(), 0, false);
        };
        for &a in avoid_nodes {
            if a == start { return Self::unreachable_res(vec![start.to_string()], vec![s_id], 0, false); }
            if a == target { return Self::unreachable_res(vec![target.to_string()], vec![t_id], 0, false); }
        }
        if s_id == t_id { return Self::reachable_res(vec![start.to_string()], vec![s_id], 0); }
        let n = memory.interner.len();

        BUFFERS.with(|cell| {
            let mut b = cell.borrow_mut();
            b.prepare(n);
            for &a in avoid_nodes {
                if let Some(id) = memory.interner.get_id(a) {
                    if (id as usize) < n { b.avoid_mask[id as usize] = true; b.touched_avoid.push(id); }
                }
            }
            if b.avoid_mask[s_id as usize] {
                b.reset();
                return Self::unreachable_res(vec![start.to_string()], vec![s_id], 0, false);
            }

            b.dist_fwd[s_id as usize] = 0; b.parent_fwd[s_id as usize] = s_id; b.touched_fwd.push(s_id);
            let mut frontier_fwd = vec![s_id];
            let mut frontier_bwd = Vec::new();
            let avoid_t = b.avoid_mask[t_id as usize];
            if !avoid_t {
                b.dist_bwd[t_id as usize] = 0; b.parent_bwd[t_id as usize] = t_id; b.touched_bwd.push(t_id);
                frontier_bwd.push(t_id);
            }
            let (mut ops, mut lf, mut lb) = (0usize, 0usize, 0usize);

            while !frontier_fwd.is_empty() && (!frontier_bwd.is_empty() || avoid_t) {
                let cf: usize = frontier_fwd.iter().map(|&u| memory.get_neighbors_u32(u).len()).sum();
                let cb: usize = if frontier_bwd.is_empty() { usize::MAX } else { frontier_bwd.iter().map(|&v| memory.get_predecessors_u32(v).len()).sum() };
                let exp_fwd = cf <= cb;
                let cost = if exp_fwd { cf } else { cb };
                if ops + cost > budget_ops {
                    let out = SearchOutcome::BudgetExceeded {
                        levels_forward: lf, levels_backward: lb,
                        frontier_size: if exp_fwd { frontier_fwd.len() } else { frontier_bwd.len() },
                        estimated_next_cost: cost, operations_spent: ops,
                    };
                    b.reset();
                    return ReachabilityResult { is_reachable: false, path_length: 0, path: Vec::new(), queries_used: ops, depth_exhausted: true, outcome: out };
                }
                if let Some(lim) = max_hops { if lf + lb >= lim { break; } }

                if exp_fwd {
                    lf += 1;
                    let mut nxt = Vec::new();
                    let mut meets = Vec::new();
                    frontier_fwd.sort_by(|&a, &b_id| memory.interner.resolve(a).cmp(&memory.interner.resolve(b_id)));
                    for curr in frontier_fwd {
                        for &nbr in memory.get_neighbors_u32(curr) {
                            ops += 1;
                            let idx = nbr as usize;
                            if idx < n && b.avoid_mask[idx] { continue; }
                            if idx < n && b.dist_bwd[idx] != u32::MAX { meets.push((curr, nbr, b.dist_fwd[curr as usize] + 1 + b.dist_bwd[idx])); }
                            if idx < n && b.dist_fwd[idx] == u32::MAX {
                                b.dist_fwd[idx] = b.dist_fwd[curr as usize] + 1;
                                b.parent_fwd[idx] = curr;
                                b.touched_fwd.push(nbr);
                                nxt.push(nbr);
                            }
                        }
                    }
                    if let Some(res) = Self::try_resolve(memory, meets, max_hops, &b.parent_fwd, &b.parent_bwd, s_id, t_id, ops) {
                        b.reset(); return res;
                    }
                    frontier_fwd = nxt;
                } else {
                    lb += 1;
                    let mut nxt = Vec::new();
                    let mut meets = Vec::new();
                    frontier_bwd.sort_by(|&a, &b_id| memory.interner.resolve(a).cmp(&memory.interner.resolve(b_id)));
                    for curr in frontier_bwd {
                        for &pred in memory.get_predecessors_u32(curr) {
                            ops += 1;
                            let idx = pred as usize;
                            if idx < n && b.avoid_mask[idx] { continue; }
                            if idx < n && b.dist_fwd[idx] != u32::MAX { meets.push((pred, curr, b.dist_fwd[idx] + 1 + b.dist_bwd[curr as usize])); }
                            if idx < n && b.dist_bwd[idx] == u32::MAX {
                                b.dist_bwd[idx] = b.dist_bwd[curr as usize] + 1;
                                b.parent_bwd[idx] = curr;
                                b.touched_bwd.push(pred);
                                nxt.push(pred);
                            }
                        }
                    }
                    if let Some(res) = Self::try_resolve(memory, meets, max_hops, &b.parent_fwd, &b.parent_bwd, s_id, t_id, ops) {
                        b.reset(); return res;
                    }
                    frontier_bwd = nxt;
                }
            }

            let mut c_ids = if frontier_bwd.is_empty() && !avoid_t { b.touched_bwd.clone() } else { b.touched_fwd.clone() };
            b.reset();
            c_ids.sort();
            let c_set = c_ids.iter().filter_map(|&id| memory.interner.resolve(id).map(|s| s.to_string())).collect();
            Self::unreachable_res(c_set, c_ids, ops, max_hops.is_some() && (lf + lb >= max_hops.unwrap()))
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn try_resolve(
        mem: &EpisodicMemory, mut meets: Vec<(u32, u32, u32)>, max_h: Option<usize>,
        pf: &[u32], pb: &[u32], sid: u32, tid: u32, ops: usize,
    ) -> Option<ReachabilityResult> {
        if let Some(lim) = max_h { meets.retain(|&(_, _, len)| len as usize <= lim); }
        if meets.is_empty() { return None; }
        meets.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| mem.interner.resolve(a.1).cmp(&mem.interner.resolve(b.1))));
        let (u, v, _) = meets[0];
        let mut ids = Vec::new();
        let mut curr = u;
        ids.push(curr);
        while curr != sid { curr = pf[curr as usize]; ids.push(curr); }
        ids.reverse();
        curr = v;
        ids.push(curr);
        while curr != tid { curr = pb[curr as usize]; ids.push(curr); }
        let path = ids.iter().map(|&id| mem.interner.resolve(id).unwrap().to_string()).collect();
        Some(Self::reachable_res(path, ids, ops))
    }

    fn reachable_res(p: Vec<String>, ids: Vec<u32>, ops: usize) -> ReachabilityResult {
        let len = if p.is_empty() { 0 } else { p.len() - 1 };
        ReachabilityResult { is_reachable: true, path_length: len, path: p.clone(), queries_used: ops, depth_exhausted: false, outcome: SearchOutcome::Reachable { path: p, path_ids: ids } }
    }

    fn unreachable_res(s: Vec<String>, ids: Vec<u32>, ops: usize, d_ex: bool) -> ReachabilityResult {
        ReachabilityResult { is_reachable: false, path_length: 0, path: Vec::new(), queries_used: ops, depth_exhausted: d_ex, outcome: SearchOutcome::Unreachable { closed_set: s, closed_set_ids: ids } }
    }
}
