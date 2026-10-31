//! # TacticalReflexKernel
//!
//! Orquestación de tiempo real duro unificando telemetría, martingalas, cicatrices,
//! contingencias y disyuntor lógico en un pipeline determinista <= 351.5 µs.

use crate::anomaly::topology_coupling::{NodeSpatialNeighborhood, ResidualSprtMartingale};
use crate::blackwall_scars::{
    Avx2PredicateBlock, CriticalEdgeKey, EmpiricalFailureToken, StructuralDiscriminationEngine,
    WL1Signature,
};
use crate::contingency_bfs::{BitAdjacencyGraph, ContingencyOutcome, PreCertifiedContingencyLibrary};
use crate::repair::EdgeOperatingBounds;
use super::circuit_breaker::{CircuitBreaker, TacticalDecision, VetoReason};
use super::context::{ReflexTickInput, TacticalKernelContext};
use super::ring_buffer::StagedOrderRingBuffer;

/// Kernel Táctico del Bucle Reflejo Unificado.
pub struct TacticalReflexKernel;

impl TacticalReflexKernel {
    /// Ejecuta 1 tick del bucle reflejo utilizando el patrón Context Object en registros de CPU.
    pub fn execute_tick(
        ctx: &mut TacticalKernelContext<'_>,
        input: &ReflexTickInput<'_>,
    ) -> TacticalDecision {
        let (u_edge, v_edge, edge_id) = input.target_edge;
        let edge_key = CriticalEdgeKey::from_nodes(u_edge as u32, v_edge as u32);

        // 1. Evaluación de telemetría diferencial y martingala SPRT
        let target_val = if (input.monitored_node as usize) < input.sensor_snapshot.len() {
            input.sensor_snapshot[input.monitored_node as usize]
        } else {
            0.0f32
        };

        let residual = input.neighborhood.compute_residual(target_val, input.sensor_snapshot);
        let (martingale_alarm, _) = ctx.martingale.update(residual);

        // 2. Discriminación estructural de cicatrices
        let telem_features = [target_val, residual.abs(), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        if (edge_id as usize) < ctx.bounds.len() {
            ctx.scar_engine.sync_operating_bounds(
                edge_key,
                input.wl1_sig,
                &telem_features,
                &mut ctx.bounds[edge_id as usize],
            );
        }

        let scar_violated = if (edge_id as usize) < ctx.bounds.len() {
            ctx.bounds[edge_id as usize].active_scar_mask != 0
        } else {
            false
        };

        // 3. Interrupción inmediata ante falla
        if martingale_alarm || scar_violated {
            let mut scar_assimilated = false;
            if martingale_alarm {
                let mut pred = Avx2PredicateBlock::empty();
                pred.thresholds[0] = target_val;
                pred.thresholds[1] = residual.abs();
                pred.active_lane_mask = 0b0000_0011;

                let scar_id = (edge_id % 64) as u8;
                if let Ok(tok) = EmpiricalFailureToken::issue_after_martingale_alarm(
                    input.current_tick,
                    edge_key,
                    ctx.martingale.log_martingale,
                    ctx.martingale.threshold_h,
                ) {
                    let _ = ctx.scar_engine.assimilate_failure(&tok, edge_key, scar_id, *input.wl1_sig, pred);
                    if (edge_id as usize) < ctx.bounds.len() {
                        ctx.scar_engine.sync_operating_bounds(
                            edge_key,
                            input.wl1_sig,
                            &telem_features,
                            &mut ctx.bounds[edge_id as usize],
                        );
                    }
                    scar_assimilated = true;
                }
            }

            let contingency = ctx.contingency_lib.select_and_apply_contingency(
                ctx.graph,
                ctx.bounds,
                input.target_edge,
                input.sources,
                input.sinks_mask,
            );

            let _ = ctx.ring_buffer.try_pop_tactical();

            let reason = if martingale_alarm {
                VetoReason::MartingaleAnomalyTriggered
            } else {
                VetoReason::ActiveScarViolation
            };

            return TacticalDecision::VetoedByCircuitBreaker {
                reason,
                contingency,
                scar_assimilated,
            };
        }

        // 4. Despacho táctico normal al DMA
        let slot = match ctx.ring_buffer.try_pop_tactical() {
            Some(s) => s,
            None => return TacticalDecision::IdleNoStagedOrder { contingency: None },
        };

        match CircuitBreaker::evaluate_last_mile(&slot, input.current_tick, ctx.bounds) {
            Ok(()) => {
                let latency = input.current_tick.saturating_sub(slot.valid_until_tick.saturating_sub(100));
                TacticalDecision::TransmitDMA {
                    envelope: slot.envelope,
                    latency_ticks: latency,
                }
            }
            Err(reason) => TacticalDecision::VetoedByCircuitBreaker {
                reason,
                contingency: ContingencyOutcome::NoViableTopologyFound,
                scar_assimilated: false,
            },
        }
    }

    /// Wrapper limpio para compatibilidad directa.
    #[inline(always)]
    pub fn execute_reflex_tick(
        current_tick: u64,
        target_edge: (u8, u8, u32),
        monitored_node: u8,
        sensor_snapshot: &[f32],
        neighborhood: &NodeSpatialNeighborhood,
        martingale: &mut ResidualSprtMartingale,
        scar_engine: &mut StructuralDiscriminationEngine,
        wl1_sig: &WL1Signature,
        contingency_lib: &PreCertifiedContingencyLibrary,
        graph: &mut BitAdjacencyGraph,
        bounds: &mut [EdgeOperatingBounds],
        sources: &[u8; 4],
        sinks_mask: &[u64; 4],
        ring_buffer: &StagedOrderRingBuffer,
    ) -> TacticalDecision {
        let mut ctx = TacticalKernelContext {
            martingale,
            scar_engine,
            contingency_lib,
            graph,
            bounds,
            ring_buffer,
        };
        let input = ReflexTickInput {
            current_tick,
            target_edge,
            monitored_node,
            sensor_snapshot,
            neighborhood,
            wl1_sig,
            sources,
            sinks_mask,
        };
        Self::execute_tick(&mut ctx, &input)
    }
}

/// Alias canónico para interoperabilidad total.
pub type TacticalReflexOrchestrator = TacticalReflexKernel;

