//! # Avx2PredicateBlock & LocalScarRule
//!
//! Evaluación vectorial de condiciones paramétricas continuas (hasta 8 dimensiones).
//! Alineación exacta a 32 bytes para carga sin penalización en registros YMM.

use super::signature::{CriticalEdgeKey, WL1Signature};

pub const MAX_SCARS_PER_EDGE: usize = 4;
pub const MAX_TRACKED_EDGES: usize = 64;

/// Bloque de predicados vectoriales evaluables en paralelo mediante AVX2 (256 bits).
#[repr(C, align(32))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Avx2PredicateBlock {
    pub thresholds: [f32; 8],
    pub active_lane_mask: u8,
    pub _padding: [u8; 31],
}

const _: () = assert!(std::mem::size_of::<Avx2PredicateBlock>() == 64);
const _: () = assert!(std::mem::align_of::<Avx2PredicateBlock>() == 32);

impl Avx2PredicateBlock {
    pub const fn empty() -> Self {
        Self {
            thresholds: [f32::INFINITY; 8],
            active_lane_mask: 0,
            _padding: [0u8; 31],
        }
    }

    pub const fn new(thresholds: [f32; 8], active_lane_mask: u8) -> Self {
        Self {
            thresholds,
            active_lane_mask,
            _padding: [0u8; 31],
        }
    }

    /// Evalúa en 1 ciclo de CPU si el vector de telemetría viola los umbrales.
    #[inline(always)]
    pub fn evaluates_true(&self, telemetry: &[f32; 8]) -> bool {
        if self.active_lane_mask == 0 {
            return false;
        }

        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        {
            unsafe {
                use core::arch::x86_64::*;
                let v_telem = _mm256_loadu_ps(telemetry.as_ptr());
                let v_thresh = _mm256_load_ps(self.thresholds.as_ptr());
                let cmp = _mm256_cmp_ps(v_telem, v_thresh, 0x1d);
                let mask = _mm256_movemask_ps(cmp) as u8;
                (mask & self.active_lane_mask) == self.active_lane_mask
            }
        }
        #[cfg(all(target_arch = "x86_64", not(target_feature = "avx2")))]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe {
                    use core::arch::x86_64::*;
                    let v_telem = _mm256_loadu_ps(telemetry.as_ptr());
                    let v_thresh = _mm256_load_ps(self.thresholds.as_ptr());
                    let cmp = _mm256_cmp_ps(v_telem, v_thresh, 0x1d);
                    let mask = _mm256_movemask_ps(cmp) as u8;
                    (mask & self.active_lane_mask) == self.active_lane_mask
                }
            } else {
                Self::scalar_matches(self.active_lane_mask, telemetry, &self.thresholds)
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Self::scalar_matches(self.active_lane_mask, telemetry, &self.thresholds)
        }
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn scalar_matches(active_mask: u8, telemetry: &[f32; 8], thresholds: &[f32; 8]) -> bool {
        for k in 0..8 {
            if (active_mask & (1 << k)) != 0 && telemetry[k] < thresholds[k] {
                return false;
            }
        }
        true
    }

    #[inline(always)]
    pub fn subsumes(&self, other: &Self) -> bool {
        if (self.active_lane_mask & other.active_lane_mask) != other.active_lane_mask {
            return false;
        }
        for k in 0..8 {
            if (other.active_lane_mask & (1 << k)) != 0 && self.thresholds[k] > other.thresholds[k] {
                return false;
            }
        }
        true
    }
}

/// Regla de cicatriz local con firma topológica y predicados numéricos.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LocalScarRule {
    pub scar_id: u8,
    pub wl1_sig: WL1Signature,
    pub predicates: Avx2PredicateBlock,
    pub active: bool,
    pub _padding: [u8; 7],
}

impl LocalScarRule {
    pub const fn empty() -> Self {
        Self {
            scar_id: 0,
            wl1_sig: WL1Signature::empty(),
            predicates: Avx2PredicateBlock::empty(),
            active: false,
            _padding: [0u8; 7],
        }
    }
}

/// Entrada estática para una arista crítica con hasta 4 slots de cicatrices.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EdgeScarSlot {
    pub edge_key: CriticalEdgeKey,
    pub scars: [LocalScarRule; MAX_SCARS_PER_EDGE],
    pub scar_count: u8,
    pub occupied: bool,
    pub _padding: [u8; 6],
}

impl EdgeScarSlot {
    pub const fn empty() -> Self {
        Self {
            edge_key: CriticalEdgeKey(0),
            scars: [LocalScarRule::empty(); MAX_SCARS_PER_EDGE],
            scar_count: 0,
            occupied: false,
            _padding: [0u8; 6],
        }
    }
}

