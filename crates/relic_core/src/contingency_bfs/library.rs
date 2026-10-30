//! # PreCertifiedContingencyLibrary
//!
//! Catálogo pre-certificado de topologías de contingencia N-1 y N-2.
//! Selección y conmutación en stack mediante restauración diferencial O(1).

use crate::repair::EdgeOperatingBounds;
use super::graph::BitAdjacencyGraph;
use super::reachability::CausalReachabilityFilter;

pub const MAX_CONTINGENCIES: usize = 32;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SwitchActionType {
    OpenIsolate = 0,
    CloseBackup = 1,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SwitchAction {
    pub u: u8,
    pub v: u8,
    pub action: SwitchActionType,
    pub edge_id: u32,
    pub new_max_capacity: f64,
}

impl SwitchAction {
    pub const fn empty() -> Self {
        Self { u: 0, v: 0, action: SwitchActionType::OpenIsolate, edge_id: 0, new_max_capacity: 0.0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PreCertifiedTopology {
    pub contingency_id: u16,
    pub failed_edge_id: u32,
    pub actions: [SwitchAction; 4],
    pub action_count: u8,
    pub thermal_safety_margin: f32,
    pub _padding: [u8; 1],
}

impl PreCertifiedTopology {
    pub const fn empty() -> Self {
        Self {
            contingency_id: 0,
            failed_edge_id: 0,
            actions: [SwitchAction::empty(); 4],
            action_count: 0,
            thermal_safety_margin: 0.0,
            _padding: [0u8; 1],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ContingencyOutcome {
    Applied { contingency_id: u16, thermal_safety_margin: f32, backup_edge_id: u32 },
    NoViableTopologyFound,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PreCertifiedContingencyLibrary {
    pub topologies: [PreCertifiedTopology; MAX_CONTINGENCIES],
    pub count: u8,
}

impl PreCertifiedContingencyLibrary {
    pub const fn new() -> Self {
        Self { topologies: [PreCertifiedTopology::empty(); MAX_CONTINGENCIES], count: 0 }
    }

    pub fn register_contingency(&mut self, topo: PreCertifiedTopology) -> Result<(), &'static str> {
        if (self.count as usize) >= MAX_CONTINGENCIES {
            return Err("Biblioteca saturada: máximo 32 contingencias");
        }
        self.topologies[self.count as usize] = topo;
        self.count += 1;
        Ok(())
    }

    pub fn select_and_apply_contingency(
        &self,
        graph: &mut BitAdjacencyGraph,
        bounds: &mut [EdgeOperatingBounds],
        failed_edge: (u8, u8, u32),
        sources: &[u8; 4],
        sinks_mask: &[u64; 4],
    ) -> ContingencyOutcome {
        let (u_fail, v_fail, edge_fail_id) = failed_edge;
        let mut sim_base = *graph;
        sim_base.quarantine_edge(u_fail, v_fail);

        let total = self.count as usize;
        let mut best_candidate_idx = None;
        let mut max_thermal_margin = -1.0f32;

        for i in 0..total {
            let topo = &self.topologies[i];
            if topo.failed_edge_id != edge_fail_id {
                continue;
            }

            let mut scar_violated = false;
            let n_actions = topo.action_count as usize;
            for j in 0..n_actions {
                let action = &topo.actions[j];
                if action.action == SwitchActionType::CloseBackup {
                    let b_idx = action.edge_id as usize;
                    if b_idx < bounds.len() && bounds[b_idx].active_scar_mask != 0 {
                        scar_violated = true;
                        break;
                    }
                }
            }
            if scar_violated {
                continue;
            }

            // Aplicación diferencial O(1) de las acciones sobre sim_base
            for j in 0..n_actions {
                let action = &topo.actions[j];
                match action.action {
                    SwitchActionType::OpenIsolate => sim_base.quarantine_edge(action.u, action.v),
                    SwitchActionType::CloseBackup => sim_base.restore_edge(action.u, action.v),
                }
            }

            let reachability = CausalReachabilityFilter::verify_critical_pairs(&sim_base, sources, sinks_mask);

            // Reversión diferencial O(1) inmediata para evitar re-copias del grafo
            for j in 0..n_actions {
                let action = &topo.actions[j];
                match action.action {
                    SwitchActionType::OpenIsolate => sim_base.restore_edge(action.u, action.v),
                    SwitchActionType::CloseBackup => sim_base.quarantine_edge(action.u, action.v),
                }
            }

            if reachability == 0b1111 && topo.thermal_safety_margin > max_thermal_margin {
                max_thermal_margin = topo.thermal_safety_margin;
                best_candidate_idx = Some(i);
            }
        }

        if let Some(best_idx) = best_candidate_idx {
            let best_topo = &self.topologies[best_idx];
            graph.quarantine_edge(u_fail, v_fail);
            let fail_idx = edge_fail_id as usize;
            if fail_idx < bounds.len() {
                bounds[fail_idx].quarantined = true;
                bounds[fail_idx].max_val = 0.0;
            }

            let mut applied_backup_id = 0u32;
            let n_actions = best_topo.action_count as usize;
            for j in 0..n_actions {
                let action = &best_topo.actions[j];
                match action.action {
                    SwitchActionType::OpenIsolate => graph.quarantine_edge(action.u, action.v),
                    SwitchActionType::CloseBackup => {
                        graph.restore_edge(action.u, action.v);
                        applied_backup_id = action.edge_id;
                        let b_idx = action.edge_id as usize;
                        if b_idx < bounds.len() {
                            bounds[b_idx].quarantined = false;
                            bounds[b_idx].max_val = action.new_max_capacity;
                        }
                    }
                }
            }

            ContingencyOutcome::Applied {
                contingency_id: best_topo.contingency_id,
                thermal_safety_margin: best_topo.thermal_safety_margin,
                backup_edge_id: applied_backup_id,
            }
        } else {
            ContingencyOutcome::NoViableTopologyFound
        }
    }
}

impl Default for PreCertifiedContingencyLibrary {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

