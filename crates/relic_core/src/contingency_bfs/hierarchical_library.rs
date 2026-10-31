//! # Hierarchical Contingency Integration
//! Selección y conmutación de contingencias jerárquicas N-1 y N-2 en stack.
//! Simulación in-place con rollback diferencial O(1) exacto (Zero-Copy, Zero-Alloc).

use super::bridge::HierarchicalNode;
use super::hierarchy::HierarchicalBitGraph;
use super::library::{ContingencyOutcome, SwitchActionType, MAX_CONTINGENCIES};
use super::sensitivity::{LinearizedSensitivityMatrix, MAX_SENSITIVITY_EDGES};
use crate::repair::EdgeOperatingBounds;

type IntraDelta = Option<(u8, u8, u8, u64, u64)>;
type InterDelta = Option<bool>;

#[derive(Clone, Copy)]
pub struct ContingencyContext<'a> {
    pub sources: &'a [HierarchicalNode; 4],
    pub sinks_c: &'a [u8; 4],
    pub sinks_m: &'a [u64; 4],
    pub sens: Option<&'a LinearizedSensitivityMatrix>,
    pub flows: Option<&'a [f32; MAX_SENSITIVITY_EDGES]>,
    pub limits: Option<&'a [f32; MAX_SENSITIVITY_EDGES]>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HierarchicalSwitchAction {
    pub src: HierarchicalNode, pub dst: HierarchicalNode,
    pub action: SwitchActionType, pub edge_id: u32, pub new_max_capacity: f64,
}

impl HierarchicalSwitchAction {
    pub const fn empty() -> Self {
        Self { src: HierarchicalNode { cluster: 0, local: 0 }, dst: HierarchicalNode { cluster: 0, local: 0 }, action: SwitchActionType::OpenIsolate, edge_id: 0, new_max_capacity: 0.0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HierarchicalContingencyPlan {
    pub contingency_id: u16, pub failed_edge_id: u32,
    pub actions: [HierarchicalSwitchAction; 4], pub action_count: u8,
    pub thermal_safety_margin: f32,
}

impl HierarchicalContingencyPlan {
    pub const fn empty() -> Self {
        Self { contingency_id: 0, failed_edge_id: 0, actions: [HierarchicalSwitchAction::empty(); 4], action_count: 0, thermal_safety_margin: 0.0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct HierarchicalContingencyLibrary {
    pub plans: [HierarchicalContingencyPlan; MAX_CONTINGENCIES], pub count: u8,
}

impl HierarchicalContingencyLibrary {
    pub const fn new() -> Self {
        Self { plans: [HierarchicalContingencyPlan::empty(); MAX_CONTINGENCIES], count: 0 }
    }

    pub fn register_plan(&mut self, plan: HierarchicalContingencyPlan) -> Result<(), &'static str> {
        if (self.count as usize) >= MAX_CONTINGENCIES { return Err("Biblioteca saturada (máx 32)"); }
        self.plans[self.count as usize] = plan; self.count += 1; Ok(())
    }

    fn apply_in_place<const K: usize>(graph: &mut HierarchicalBitGraph<K>, act: &HierarchicalSwitchAction) -> (IntraDelta, InterDelta) {
        let (s, d) = (act.src, act.dst);
        if s.cluster == d.cluster {
            let (c, u, v) = (s.cluster as usize, s.local as usize, d.local as usize);
            let (pu, pv) = (graph.islands[c].adj[u], graph.islands[c].adj[v]);
            match act.action {
                SwitchActionType::OpenIsolate => graph.islands[c].quarantine_edge(s.local, d.local),
                SwitchActionType::CloseBackup => graph.islands[c].restore_edge(s.local, d.local),
            }
            (Some((s.cluster, s.local, d.local, pu, pv)), None)
        } else {
            let prev = graph.bridges.bridge_count(s, d);
            match act.action {
                SwitchActionType::OpenIsolate => { let _ = graph.quarantine_bridge(s, d); }
                SwitchActionType::CloseBackup => { let _ = graph.restore_bridge(s, d); }
            }
            (None, Some(prev != graph.bridges.bridge_count(s, d)))
        }
    }

    fn rollback_in_place<const K: usize>(graph: &mut HierarchicalBitGraph<K>, act: &HierarchicalSwitchAction, d_intra: IntraDelta, d_inter: InterDelta) {
        if let Some((c, u, v, pu, pv)) = d_intra {
            graph.islands[c as usize].adj[u as usize] = pu; graph.islands[c as usize].adj[v as usize] = pv;
        } else if let Some(true) = d_inter {
            match act.action {
                SwitchActionType::OpenIsolate => { let _ = graph.restore_bridge(act.src, act.dst); }
                SwitchActionType::CloseBackup => { let _ = graph.quarantine_bridge(act.src, act.dst); }
            }
        }
    }

    pub fn select_and_apply_contingency_hierarchical<const K: usize>(
        &self, graph: &mut HierarchicalBitGraph<K>, bounds: &mut [EdgeOperatingBounds],
        failed: (HierarchicalNode, HierarchicalNode, u32), ctx: ContingencyContext,
    ) -> ContingencyOutcome {
        let (sf, df, fid) = failed;
        let sources = ctx.sources;
        let sinks_c = ctx.sinks_c;
        let sinks_m = ctx.sinks_m;
        let iso = HierarchicalSwitchAction { src: sf, dst: df, action: SwitchActionType::OpenIsolate, edge_id: fid, new_max_capacity: 0.0 };
        let _ = Self::apply_in_place(graph, &iso);

        let mut best = None; let mut max_margin = -1.0f32;
        for i in 0..self.count as usize {
            let plan = &self.plans[i];
            if plan.failed_edge_id != fid { continue; }
            let mut scarred = false;
            for j in 0..plan.action_count as usize {
                let act = &plan.actions[j];
                if act.action == SwitchActionType::CloseBackup {
                    let b = act.edge_id as usize;
                    if b < bounds.len() && (bounds[b].active_scar_mask != 0 || bounds[b].quarantined) { scarred = true; break; }
                }
            }
            if scarred { continue; }

            if let (Some(s), Some(f), Some(lim)) = (ctx.sens, ctx.flows, ctx.limits) {
                let out_e = (fid as u8).min(MAX_SENSITIVITY_EDGES as u8 - 1);
                let mut scratch = [0.0f32; MAX_SENSITIVITY_EDGES];
                if s.evaluate_post_contingency_flows(f, &[out_e], lim, &mut scratch).is_err() { continue; }
            }

            let n_act = plan.action_count as usize;
            let mut deltas = [(None, None); 4];
            for (j, delta) in deltas.iter_mut().enumerate().take(n_act) {
                *delta = Self::apply_in_place(graph, &plan.actions[j]);
            }
            let mask = graph.verify_hierarchical_pairs(sources, sinks_c, sinks_m);
            for j in (0..n_act).rev() { Self::rollback_in_place(graph, &plan.actions[j], deltas[j].0, deltas[j].1); }

            if mask == 0b1111 && plan.thermal_safety_margin > max_margin {
                max_margin = plan.thermal_safety_margin; best = Some(i);
            }
        }

        if let Some(idx) = best {
            let bp = &self.plans[idx];
            if (fid as usize) < bounds.len() { bounds[fid as usize].quarantined = true; bounds[fid as usize].max_val = 0.0; }
            let mut backup_id = 0u32;
            for j in 0..bp.action_count as usize {
                let act = &bp.actions[j];
                let _ = Self::apply_in_place(graph, act);
                if act.action == SwitchActionType::CloseBackup {
                    backup_id = act.edge_id;
                    let b = act.edge_id as usize;
                    if b < bounds.len() { bounds[b].quarantined = false; bounds[b].max_val = act.new_max_capacity; }
                }
            }
            ContingencyOutcome::Applied { contingency_id: bp.contingency_id, thermal_safety_margin: bp.thermal_safety_margin, backup_edge_id: backup_id }
        } else {
            if (fid as usize) < bounds.len() { bounds[fid as usize].quarantined = true; bounds[fid as usize].max_val = 0.0; }
            ContingencyOutcome::NoViableTopologyFound
        }
    }
}

impl Default for HierarchicalContingencyLibrary {
    fn default() -> Self { Self::new() }
}
