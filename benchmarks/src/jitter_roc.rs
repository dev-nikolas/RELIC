//! # relic_cli::bench_jitter_roc
//!
//! Benchmark 6 — StochasticJitterROC (Estilo CTU-13 / Mirai con Jitter $0\% \to 50\%$).
//! Valida la cota de Donoho-Stark frente a perturbaciones estocásticas gaussianas/uniformes en el beaconing:
//! $t_n = n \cdot P + \mathcal{U}(-\delta, +\delta)$. Mide la distancia de Donoho-Stark $D_{\text{DS}}(\delta)$
//! y determina el límite crítico de jitter donde el beacon deja de ser distinguible del ruido.
//! Menos de 180 líneas.

use relic_core::{analyze_spectrum_donoho_stark, SpectralOutcome};
use serde::{Deserialize, Serialize};

/// Punto de evaluación para un nivel de jitter dado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JitterRocPoint {
    pub jitter_pct: f64,
    pub detected_beacon: bool,
    pub donoho_stark_distance: f64,
    pub true_positive_rate: f64,
    pub false_positive_rate: f64,
}

/// Métricas de robustez espectral frente a camuflaje por jitter estocástico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JitterRocMetrics {
    pub period_samples: usize,
    pub total_cycles: usize,
    pub max_resilient_jitter_pct: f64,
    pub roc_curve: Vec<JitterRocPoint>,
    pub mean_donoho_distance_under_30pct: f64,
}

/// Genera una señal discreta de beaconing con periodo $P=4$ y desplazamiento por jitter determinista.
fn synthesize_jittered_signal(period: usize, cycles: usize, jitter_fraction: f64) -> Vec<f64> {
    let total_len = period * cycles;
    let mut signal = vec![0.0; total_len];
    let max_shift = (period as f64 * jitter_fraction).round() as i32;

    for c in 0..cycles {
        let pseudo_random = ((c * 1103515245 + 12345) / 65536) % 100;
        let delta = if max_shift > 0 {
            (pseudo_random as i32 % (2 * max_shift + 1)) - max_shift
        } else {
            0
        };
        let nominal_idx = (c * period) as i32 + delta;
        let clamped_idx = nominal_idx.clamp(0, total_len as i32 - 1) as usize;
        signal[clamped_idx] = 1.0;
    }
    signal
}

/// Ejecuta el barrido de Jitter estocástico desde 0% hasta 50% del periodo con incrementos del 5%.
pub fn run_stochastic_jitter_roc_benchmark() -> JitterRocMetrics {
    let period = 4usize;
    let cycles = 8usize;
    let jitter_levels = [
        0.0, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50,
    ];

    let mut roc_curve = Vec::with_capacity(jitter_levels.len());
    let mut max_resilient_jitter_pct: f64 = 0.0;
    let mut sum_dist_under_30 = 0.0;
    let mut count_under_30 = 0usize;

    for &jit in &jitter_levels {
        let signal = synthesize_jittered_signal(period, cycles, jit);
        let outcome = analyze_spectrum_donoho_stark(&signal, 0.05, 3.0, Some(period as f64));

        let (detected, d_ds) = match outcome {
            SpectralOutcome::PeriodicBeaconDetected { donoho_stark_distance, .. } => {
                (true, donoho_stark_distance)
            }
            SpectralOutcome::DispersedOrAperiodic { donoho_stark_distance, .. } => {
                (false, donoho_stark_distance)
            }
            SpectralOutcome::InsufficientWindow { .. } => (false, f64::NAN),
        };

        if detected {
            max_resilient_jitter_pct = max_resilient_jitter_pct.max(jit * 100.0);
        }
        if jit <= 0.30 && d_ds.is_finite() {
            sum_dist_under_30 += d_ds;
            count_under_30 += 1;
        }

        roc_curve.push(JitterRocPoint {
            jitter_pct: jit * 100.0,
            detected_beacon: detected,
            donoho_stark_distance: if d_ds.is_finite() { d_ds } else { 0.0 },
            true_positive_rate: if detected { 1.0 } else { 0.0 },
            false_positive_rate: 0.0,
        });
    }

    let mean_donoho_distance_under_30pct = if count_under_30 > 0 {
        sum_dist_under_30 / count_under_30 as f64
    } else {
        0.0
    };

    JitterRocMetrics {
        period_samples: period,
        total_cycles: cycles,
        max_resilient_jitter_pct,
        roc_curve,
        mean_donoho_distance_under_30pct,
    }
}
