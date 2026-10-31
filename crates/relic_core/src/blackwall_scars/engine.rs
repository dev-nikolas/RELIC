//! # StructuralDiscriminationEngine
//!
//! Motor de cicatrices con separación SoA de llaves críticas para optimización de caché L1.
//! Búsqueda en 512 bytes contiguos (8 cache lines) sin tocar slots pesados.

use crate::repair::EdgeOperatingBounds;
use super::predicates::{Avx2PredicateBlock, EdgeScarSlot, LocalScarRule, MAX_SCARS_PER_EDGE, MAX_TRACKED_EDGES};
use super::signature::{CriticalEdgeKey, WL1Signature};
use super::token::EmpiricalFailureToken;

/// Motor de Discriminación Estructural de Cicatrices en Tiempo Real (Zero-Alloc, SoA).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StructuralDiscriminationEngine {
    /// Llaves empaquetadas en un único bloque plano SoA de 512 bytes (8 líneas de caché L1).
    pub edge_keys: [u64; MAX_TRACKED_EDGES],
    /// Slots pesados conteniendo reglas y bloques AVX2.
    pub slots: [EdgeScarSlot; MAX_TRACKED_EDGES],
    pub slot_count: u8,
    pub _padding: [u8; 7],
}

impl StructuralDiscriminationEngine {
    pub const fn new() -> Self {
        Self {
            edge_keys: [0u64; MAX_TRACKED_EDGES],
            slots: [EdgeScarSlot::empty(); MAX_TRACKED_EDGES],
            slot_count: 0,
            _padding: [0u8; 7],
        }
    }

    #[inline(always)]
    fn find_slot_idx(&self, key: u64) -> Option<usize> {
        let count = (self.slot_count as usize).min(MAX_TRACKED_EDGES);
        for i in 0..count {
            if self.edge_keys[i] == key {
                return Some(i);
            }
        }
        None
    }

    pub fn assimilate_failure(
        &mut self,
        _token: &EmpiricalFailureToken,
        edge_key: CriticalEdgeKey,
        scar_id: u8,
        wl1_sig: WL1Signature,
        predicates: Avx2PredicateBlock,
    ) -> Result<bool, &'static str> {
        let key_val = edge_key.0;
        let slot_idx = if let Some(idx) = self.find_slot_idx(key_val) {
            idx
        } else {
            if (self.slot_count as usize) >= MAX_TRACKED_EDGES {
                return Err("Capacidad de aristas críticas saturada");
            }
            let idx = self.slot_count as usize;
            self.edge_keys[idx] = key_val;
            self.slots[idx].edge_key = edge_key;
            self.slots[idx].occupied = true;
            self.slot_count += 1;
            idx
        };

        let slot = &mut self.slots[slot_idx];
        let n_scars = slot.scar_count as usize;

        // Subsunción por Dominancia Convexa in-place para la misma firma WL-1
        for i in 0..n_scars {
            let existing = &mut slot.scars[i];
            if existing.active && existing.wl1_sig.matches_simd(&wl1_sig) {
                for k in 0..8 {
                    if (predicates.active_lane_mask & (1 << k)) != 0 {
                        if (existing.predicates.active_lane_mask & (1 << k)) != 0 {
                            existing.predicates.thresholds[k] =
                                existing.predicates.thresholds[k].min(predicates.thresholds[k]);
                        } else {
                            existing.predicates.thresholds[k] = predicates.thresholds[k];
                        }
                    }
                }
                existing.predicates.active_lane_mask |= predicates.active_lane_mask;
                return Ok(true);
            }
        }

        if n_scars >= MAX_SCARS_PER_EDGE {
            slot.scars[MAX_SCARS_PER_EDGE - 1] = LocalScarRule {
                scar_id,
                wl1_sig,
                predicates,
                active: true,
                _padding: [0u8; 7],
            };
            return Ok(false);
        }

        slot.scars[n_scars] = LocalScarRule {
            scar_id,
            wl1_sig,
            predicates,
            active: true,
            _padding: [0u8; 7],
        };
        slot.scar_count += 1;
        Ok(false)
    }

    #[inline(always)]
    pub fn evaluate_edge_scars(
        &self,
        edge_key: CriticalEdgeKey,
        wl1_sig: &WL1Signature,
        telemetry: &[f32; 8],
    ) -> u64 {
        let idx = match self.find_slot_idx(edge_key.0) {
            Some(i) => i,
            None => return 0,
        };

        let slot = &self.slots[idx];
        let mut active_mask = 0u64;
        let n = slot.scar_count as usize;

        for i in 0..n {
            let scar = &slot.scars[i];
            if scar.active && scar.wl1_sig.matches_simd(wl1_sig) && scar.predicates.evaluates_true(telemetry) {
                active_mask |= 1u64 << (scar.scar_id % 64);
            }
        }

        active_mask
    }

    #[inline(always)]
    pub fn sync_operating_bounds(
        &self,
        edge_key: CriticalEdgeKey,
        wl1_sig: &WL1Signature,
        telemetry: &[f32; 8],
        bounds: &mut EdgeOperatingBounds,
    ) {
        bounds.active_scar_mask = self.evaluate_edge_scars(edge_key, wl1_sig, telemetry);
    }
}

impl Default for StructuralDiscriminationEngine {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

