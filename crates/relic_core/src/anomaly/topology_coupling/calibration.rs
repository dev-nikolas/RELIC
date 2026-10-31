//! # QuiescentCalibrator
//!
//! Calibración en línea de pesos espaciales con compuerta de quiescencia y cotas convexas.
//! Garantiza complejidad O(1) acotada, cero asignaciones y resistencia al ataque *Boiling Frog*.

use super::types::{NodeSpatialNeighborhood, MAX_BOUNDED_NEIGHBORS};

/// Calibrador espacial NLMS gobernado por quiescencia de martingala.
pub struct QuiescentCalibrator;

impl QuiescentCalibrator {
    /// Actualiza los pesos espaciales `w_ji` mediante NLMS acotado SOLO si la martingala
    /// está estrictamente inerte (`log_martingale == 0.0`).
    /// Si existe cualquier sospecha incipiente (`log_martingale > 0.0`), retorna inmediatamente en O(1).
    #[inline(always)]
    pub fn adapt_weights(
        neighborhood: &mut NodeSpatialNeighborhood,
        _target_val: f32,
        sensor_snapshot: &[f32],
        residual: f32,
        log_martingale: f32,
        learning_rate_mu: f32,
    ) {
        // Compuerta estricta: silencio absoluto requerido para aprender
        if log_martingale > 0.0 || !residual.is_finite() {
            return;
        }

        let n = (neighborhood.count as usize).min(MAX_BOUNDED_NEIGHBORS);
        let len = sensor_snapshot.len();

        // 1. Norma del vector de entrada local para normalizar el paso
        let mut norm_sq = 1e-4f32;
        for i in 0..n {
            let idx = neighborhood.indices[i] as usize;
            if idx < len {
                let x_j = sensor_snapshot[idx];
                norm_sq = x_j.mul_add(x_j, norm_sq);
            }
        }

        let step_factor = learning_rate_mu / norm_sq;

        // 2. Micro-actualización NLMS y proyección politópica dura en caja [weight_min, weight_max]
        for i in 0..n {
            let idx = neighborhood.indices[i] as usize;
            if idx < len {
                let x_j = sensor_snapshot[idx];
                let delta = step_factor * residual * x_j;
                let updated = neighborhood.weights[i] + delta;

                // Clampeo convexo inmediato dentro de la región de confianza física
                neighborhood.weights[i] = updated.clamp(
                    neighborhood.weight_min[i],
                    neighborhood.weight_max[i],
                );
            }
        }
    }
}

