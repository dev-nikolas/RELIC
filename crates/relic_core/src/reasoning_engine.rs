//! Motor de razonamiento algorítmico determinista (BFS Bidireccional de Horizonte Elástico).
use crate::certificate::{ReachabilityResult, SearchOutcome};
use crate::episodic_memory::EpisodicMemory;
use crate::search_buffer::BUFFERS;

#[derive(Debug, Default, Clone, Copy)]
pub struct ReasoningEngine;

impl ReasoningEngine {
    pub fn default_budget(mem: &EpisodicMemory) -> usize { (mem.total_node_count() + mem.edge_count()).saturating_mul(4).max(10_000) }
    pub fn solve_ms_bfs_batch(m: &EpisodicMemory, q: &[(u32, u32)]) -> crate::ms_bfs::MsBfsResult { crate::ms_bfs::solve_ms_bfs_batch(m, q) }
    pub fn solve_ms_bfs_counterfactual(m: &EpisodicMemory, s: u32, t: u32, w: usize) -> crate::ms_bfs::MsBfsResult { crate::ms_bfs::solve_ms_bfs_counterfactual(m, s, t, w) }

    pub fn solve_reachability_bfs(mem: &mut EpisodicMemory, s: &str, t: &str, depth: usize) -> ReachabilityResult {
        Self::solve_reachability_bidirectional_opts(mem, s, t, Some(depth), &[], Self::default_budget(mem), true)
    }

    pub fn solve_reachability_bfs_ids(mem: &mut EpisodicMemory, s: &str, t: &str, depth: usize) -> ReachabilityResult {
        Self::solve_reachability_bidirectional_opts(mem, s, t, Some(depth), &[], Self::default_budget(mem), false)
    }

    pub fn solve_reachability_bfs_constrained(mem: &mut EpisodicMemory, s: &str, t: &str, depth: usize, avoid: &[&str]) -> ReachabilityResult {
        Self::solve_reachability_bidirectional_opts(mem, s, t, Some(depth), avoid, Self::default_budget(mem), true)
    }

    pub fn solve_reachability_bidirectional(mem: &mut EpisodicMemory, s: &str, t: &str, max_hops: Option<usize>, avoid: &[&str], budget: usize) -> ReachabilityResult {
        Self::solve_reachability_bidirectional_opts(mem, s, t, max_hops, avoid, budget, true)
    }

    pub fn solve_reachability_bidirectional_opts(
        mem: &mut EpisodicMemory, start: &str, target: &str,
        max_hops: Option<usize>, avoid_nodes: &[&str], budget_ops: usize, mat_str: bool,
    ) -> ReachabilityResult {
        let (Some(s_id), Some(t_id)) = (mem.interner.get_id(start), mem.interner.get_id(target)) else {
            return Self::unreachable_res(Vec::new(), Vec::new(), 0, false);
        };
        for &a in avoid_nodes {
            if a == start { return Self::unreachable_res(vec![start.to_string()], vec![s_id], 0, false); }
            if a == target { return Self::unreachable_res(vec![target.to_string()], vec![t_id], 0, false); }
        }
        if s_id == t_id {
            let p = if mat_str { vec![start.to_string()] } else { Vec::new() };
            return Self::reachable_res(p, vec![s_id], 0);
        }
        let n = mem.interner.len();

        BUFFERS.with(|cell| {
            let mut b = cell.borrow_mut();
            b.prepare(n);
            let gen = b.current_gen;
            for &a in avoid_nodes {
                if let Some(id) = mem.interner.get_id(a) { if (id as usize) < n { b.avoid_gen[id as usize] = gen; } }
            }
            if b.avoid_gen[s_id as usize] == gen {
                b.reset();
                let p = if mat_str { vec![start.to_string()] } else { Vec::new() };
                return Self::unreachable_res(p, vec![s_id], 0, false);
            }

            b.visited_gen_fwd[s_id as usize] = gen; b.dist_fwd[s_id as usize] = 0; b.parent_fwd[s_id as usize] = s_id; b.touched_fwd.push(s_id);
            let (mut frontier_fwd, mut frontier_bwd) = (vec![s_id], Vec::new());
            let avoid_t = b.avoid_gen[t_id as usize] == gen;
            if !avoid_t {
                b.visited_gen_bwd[t_id as usize] = gen; b.dist_bwd[t_id as usize] = 0; b.parent_bwd[t_id as usize] = t_id;
                b.touched_bwd.push(t_id); frontier_bwd.push(t_id);
            }
            let (mut ops, mut lf, mut lb) = (0usize, 0usize, 0usize);

            while !frontier_fwd.is_empty() && (!frontier_bwd.is_empty() || avoid_t) {
                let cf: usize = frontier_fwd.iter().map(|&u| mem.get_neighbors_u32(u).len()).sum();
                let cb: usize = if frontier_bwd.is_empty() { usize::MAX } else { frontier_bwd.iter().map(|&v| mem.get_predecessors_u32(v).len()).sum() };
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
                    let (mut nxt, mut meets) = (Vec::new(), Vec::new());
                    frontier_fwd.sort_by(|&a, &b_id| mem.interner.resolve(a).cmp(&mem.interner.resolve(b_id)));
                    for curr in frontier_fwd {
                        for &nbr in mem.get_neighbors_u32(curr) {
                            ops += 1;
                            let idx = nbr as usize;
                            if idx < n && b.avoid_gen[idx] == gen { continue; }
                            if idx < n && b.visited_gen_bwd[idx] == gen { meets.push((curr, nbr, b.dist_fwd[curr as usize] + 1 + b.dist_bwd[idx])); }
                            if idx < n && b.visited_gen_fwd[idx] != gen {
                                b.visited_gen_fwd[idx] = gen; b.dist_fwd[idx] = b.dist_fwd[curr as usize] + 1;
                                b.parent_fwd[idx] = curr; b.touched_fwd.push(nbr); nxt.push(nbr);
                            }
                        }
                    }
                    if let Some(res) = Self::try_resolve(mem, meets, max_hops, &b.parent_fwd, &b.parent_bwd, s_id, t_id, ops, mat_str) {
                        b.reset(); return res;
                    }
                    frontier_fwd = nxt;
                } else {
                    lb += 1;
                    let (mut nxt, mut meets) = (Vec::new(), Vec::new());
                    frontier_bwd.sort_by(|&a, &b_id| mem.interner.resolve(a).cmp(&mem.interner.resolve(b_id)));
                    for curr in frontier_bwd {
                        for &pred in mem.get_predecessors_u32(curr) {
                            ops += 1;
                            let idx = pred as usize;
                            if idx < n && b.avoid_gen[idx] == gen { continue; }
                            if idx < n && b.visited_gen_fwd[idx] == gen { meets.push((pred, curr, b.dist_fwd[idx] + 1 + b.dist_bwd[curr as usize])); }
                            if idx < n && b.visited_gen_bwd[idx] != gen {
                                b.visited_gen_bwd[idx] = gen; b.dist_bwd[idx] = b.dist_bwd[curr as usize] + 1;
                                b.parent_bwd[idx] = curr; b.touched_bwd.push(pred); nxt.push(pred);
                            }
                        }
                    }
                    if let Some(res) = Self::try_resolve(mem, meets, max_hops, &b.parent_fwd, &b.parent_bwd, s_id, t_id, ops, mat_str) {
                        b.reset(); return res;
                    }
                    frontier_bwd = nxt;
                }
            }

            let mut c_ids = if frontier_bwd.is_empty() && !avoid_t { b.touched_bwd.clone() } else { b.touched_fwd.clone() };
            b.reset();
            c_ids.sort_unstable();
            let c_set = if mat_str { c_ids.iter().filter_map(|&id| mem.interner.resolve(id).map(|s| s.to_string())).collect() } else { Vec::new() };
            Self::unreachable_res(c_set, c_ids, ops, max_hops.is_some() && (lf + lb >= max_hops.unwrap()))
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn try_resolve(
        mem: &EpisodicMemory, mut meets: Vec<(u32, u32, u32)>, max_h: Option<usize>,
        pf: &[u32], pb: &[u32], sid: u32, tid: u32, ops: usize, mat_str: bool,
    ) -> Option<ReachabilityResult> {
        if let Some(lim) = max_h { meets.retain(|&(_, _, len)| len as usize <= lim); }
        if meets.is_empty() { return None; }
        meets.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| mem.interner.resolve(a.1).cmp(&mem.interner.resolve(b.1))));
        let (u, v, _) = meets[0];
        let (mut ids, mut curr) = (Vec::new(), u);
        ids.push(curr);
        while curr != sid { curr = pf[curr as usize]; ids.push(curr); }
        ids.reverse();
        curr = v;
        ids.push(curr);
        while curr != tid { curr = pb[curr as usize]; ids.push(curr); }
        let path = if mat_str { ids.iter().map(|&id| mem.interner.resolve(id).unwrap().to_string()).collect() } else { Vec::new() };
        Some(Self::reachable_res(path, ids, ops))
    }

    fn reachable_res(p: Vec<String>, ids: Vec<u32>, ops: usize) -> ReachabilityResult {
        let len = if !ids.is_empty() { ids.len() - 1 } else if !p.is_empty() { p.len() - 1 } else { 0 };
        ReachabilityResult { is_reachable: true, path_length: len, path: p.clone(), queries_used: ops, depth_exhausted: false, outcome: SearchOutcome::Reachable { path: p, path_ids: ids } }
    }

    fn unreachable_res(s: Vec<String>, ids: Vec<u32>, ops: usize, d_ex: bool) -> ReachabilityResult {
        ReachabilityResult { is_reachable: false, path_length: 0, path: Vec::new(), queries_used: ops, depth_exhausted: d_ex, outcome: SearchOutcome::Unreachable { closed_set: s, closed_set_ids: ids } }
    }
}
