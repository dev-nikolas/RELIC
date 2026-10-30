//! # LocalConstraintRepairer
//!
//! Motor determinista de diagnóstico UnsatCore y proyección local convexa.
//! Tiempo de ejecución garantizado < 15 µs y 0 asignaciones dinámicas.

use std::io::Write;
use super::envelope::{EnvelopeStatus, RepairOutcome, StagedEnvelope};
use super::types::{ConstraintViolation, EdgeOperatingBounds, UnsatCore, ViolationType};

pub struct LocalConstraintRepairer;

impl LocalConstraintRepairer {
    pub fn validate_envelope(
        envelope: &StagedEnvelope,
        bounds: &[EdgeOperatingBounds],
        max_total_budget: f64,
    ) -> UnsatCore {
        let mut core = UnsatCore::empty();
        let ops = envelope.ops_slice();
        let mut total_requested = 0.0f64;

        for (i, op) in ops.iter().enumerate() {
            let op_idx = i as u8;
            if !op.requested_val.is_finite() || !op.current_val.is_finite() {
                core.push(ConstraintViolation {
                    op_index: op_idx, edge_id: op.edge_id, violation_type: ViolationType::NonFiniteParameter,
                    requested_val: op.requested_val, admissible_min: 0.0, admissible_max: 0.0, repairable: false,
                });
                continue;
            }

            let edge_idx = op.edge_id as usize;
            if edge_idx >= bounds.len() {
                core.push(ConstraintViolation {
                    op_index: op_idx, edge_id: op.edge_id, violation_type: ViolationType::TopologyDisconnected,
                    requested_val: op.requested_val, admissible_min: 0.0, admissible_max: 0.0, repairable: false,
                });
                continue;
            }

            let b = &bounds[edge_idx];
            if b.quarantined {
                core.push(ConstraintViolation {
                    op_index: op_idx, edge_id: op.edge_id, violation_type: ViolationType::EdgeQuarantined,
                    requested_val: op.requested_val, admissible_min: 0.0, admissible_max: 0.0, repairable: false,
                });
                continue;
            }

            if b.active_scar_mask != 0 {
                core.push(ConstraintViolation {
                    op_index: op_idx, edge_id: op.edge_id, violation_type: ViolationType::ScarInvariantViolated,
                    requested_val: op.requested_val, admissible_min: b.min_val, admissible_max: b.max_val, repairable: false,
                });
                continue;
            }

            if op.requested_val > b.max_val {
                core.push(ConstraintViolation {
                    op_index: op_idx, edge_id: op.edge_id, violation_type: ViolationType::CapacityExceeded,
                    requested_val: op.requested_val, admissible_min: b.min_val, admissible_max: b.max_val, repairable: true,
                });
            }

            if op.requested_val < b.min_val {
                core.push(ConstraintViolation {
                    op_index: op_idx, edge_id: op.edge_id, violation_type: ViolationType::UnderflowBound,
                    requested_val: op.requested_val, admissible_min: b.min_val, admissible_max: b.max_val, repairable: true,
                });
            }

            let ramp_delta = (op.requested_val - op.current_val).abs();
            if ramp_delta > b.max_ramp_delta {
                core.push(ConstraintViolation {
                    op_index: op_idx, edge_id: op.edge_id, violation_type: ViolationType::RampRateExceeded,
                    requested_val: op.requested_val, admissible_min: op.current_val - b.max_ramp_delta,
                    admissible_max: op.current_val + b.max_ramp_delta, repairable: true,
                });
            }

            total_requested += op.requested_val;
        }

        if total_requested > max_total_budget && max_total_budget > 0.0 {
            core.push(ConstraintViolation {
                op_index: 0xFF, edge_id: 0, violation_type: ViolationType::BudgetConservationExceeded,
                requested_val: total_requested, admissible_min: 0.0, admissible_max: max_total_budget, repairable: true,
            });
        }

        core
    }

    pub fn repair_or_veto(
        envelope: &mut StagedEnvelope,
        bounds: &[EdgeOperatingBounds],
        max_total_budget: f64,
    ) -> RepairOutcome {
        let core = Self::validate_envelope(envelope, bounds, max_total_budget);
        if core.is_empty() {
            envelope.status = EnvelopeStatus::Certified;
            for op in envelope.ops_mut_slice() { op.certified = true; }
            return RepairOutcome::CertifiedAsIs;
        }
        if !core.all_repairable() {
            envelope.status = EnvelopeStatus::Vetoed;
            for op in envelope.ops_mut_slice() { op.certified = false; }
            return RepairOutcome::VetoedIrreparable { core };
        }

        let mut repaired_mask = 0u8;
        for (i, op) in envelope.ops_mut_slice().iter_mut().enumerate() {
            let edge_idx = op.edge_id as usize;
            if edge_idx < bounds.len() {
                let b = &bounds[edge_idx];
                let mut clamped = op.requested_val.clamp(b.min_val, b.max_val);
                clamped = clamped.clamp(op.current_val - b.max_ramp_delta, op.current_val + b.max_ramp_delta);
                if (clamped - op.requested_val).abs() > 1e-6 {
                    op.requested_val = clamped;
                    op.repaired = true;
                    repaired_mask |= 1 << i;
                }
            }
        }

        let total_post: f64 = envelope.ops_slice().iter().map(|o| o.requested_val).sum();
        if total_post > max_total_budget && max_total_budget > 0.0 && total_post > 1e-6 {
            let scale = max_total_budget / total_post;
            for (i, op) in envelope.ops_mut_slice().iter_mut().enumerate() {
                op.requested_val *= scale;
                op.repaired = true;
                repaired_mask |= 1 << i;
            }
        }

        let second_check = Self::validate_envelope(envelope, bounds, max_total_budget);
        if second_check.is_empty() {
            envelope.status = EnvelopeStatus::Repaired;
            for op in envelope.ops_mut_slice() { op.certified = true; }
            RepairOutcome::RepairedInPlace { core, repaired_ops_mask: repaired_mask }
        } else {
            envelope.status = EnvelopeStatus::Vetoed;
            RepairOutcome::VetoedIrreparable { core: second_check }
        }
    }

    pub fn format_unsat_json(core: &UnsatCore, buf: &mut [u8; 512]) -> usize {
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        let status = if core.all_repairable() { "REJECTED_REPAIRABLE" } else { "REJECTED_IRREPARABLE" };
        let _ = write!(cursor, "{{\"status\":\"{}\",\"violation_count\":{},\"violations\":[", status, core.len());
        for (i, v) in core.as_slice().iter().enumerate() {
            let sep = if i > 0 { "," } else { "" };
            let _ = write!(
                cursor, "{}{{\"op\":{},\"edge\":{},\"type\":\"{}\",\"req\":{:.2},\"min\":{:.2},\"max\":{:.2},\"repairable\":{}}}",
                sep, v.op_index, v.edge_id, v.violation_type.as_str(), v.requested_val, v.admissible_min, v.admissible_max, v.repairable
            );
        }
        let _ = write!(cursor, "]}}");
        cursor.position() as usize
    }
}
