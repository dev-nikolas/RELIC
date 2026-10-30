//! # CircuitBreaker & TacticalDecision
//!
//! Disyuntor lógico de última milla para intercepción determinista (< 1 µs) de TOCTOU.

use crate::contingency_bfs::ContingencyOutcome;
use crate::repair::{EdgeOperatingBounds, StagedEnvelope};
use super::ring_buffer::StagedPolicySlot;

/// Razón formal del veto inmediato emitido por el Disyuntor Lógico.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VetoReason {
    MartingaleAnomalyTriggered = 0,
    ActiveScarViolation = 1,
    StagedOrderExpired = 2,
    PostStagingQuarantineDetected = 3,
}

/// Decisión táctica emitida en el bucle reflejo de microsegundos.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TacticalDecision {
    TransmitDMA {
        envelope: StagedEnvelope,
        latency_ticks: u64,
    },
    VetoedByCircuitBreaker {
        reason: VetoReason,
        contingency: ContingencyOutcome,
        scar_assimilated: bool,
    },
    IdleNoStagedOrder {
        contingency: Option<ContingencyOutcome>,
    },
}

/// Disyuntor Lógico de Interrupción Táctica Inmediata (< 1 µs).
pub struct CircuitBreaker;

impl CircuitBreaker {
    #[inline(always)]
    pub fn evaluate_last_mile(
        slot: &StagedPolicySlot,
        current_tick: u64,
        bounds: &[EdgeOperatingBounds],
    ) -> Result<(), VetoReason> {
        if current_tick > slot.valid_until_tick {
            return Err(VetoReason::StagedOrderExpired);
        }

        for op in slot.envelope.ops_slice() {
            let edge_idx = op.edge_id as usize;
            if edge_idx < bounds.len() {
                if bounds[edge_idx].quarantined {
                    return Err(VetoReason::PostStagingQuarantineDetected);
                }
                if bounds[edge_idx].active_scar_mask != 0 {
                    return Err(VetoReason::ActiveScarViolation);
                }
            }
        }

        Ok(())
    }
}

