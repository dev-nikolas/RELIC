//! # relic_core::spectral
//!
//! Motor espectral determinista y Principio de Incertidumbre Discreto de Donoho-Stark en Rust puro.
//! Incluye Transformada Discreta de Fourier exacta (DFT), soporte temporal/espectral y
//! correlación de Pearson numéricamente estable (Welford). Menos de 180 líneas.

use std::f64::consts::PI;
use serde::{Deserialize, Serialize};
use crate::quantum::Complex;

pub use crate::network_audit::{compute_destination_entropy, compute_syn_ack_ratio, verify_network_alert, NetworkAlertSpec, VerifiedAlertReport};
pub use crate::spectral_sprt::{SprtBeaconDetector, SprtConfig, SprtOutcome};

/// Resultado auditable del análisis espectral mediante DFT y Principio de Donoho-Stark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpectralOutcome {
    PeriodicBeaconDetected {
        n_samples: usize,
        dominant_bin: usize,
        estimated_period: f64,
        peak_to_average_ratio: f64,
        temporal_support: usize,
        spectral_support: usize,
        donoho_stark_product: usize,
        donoho_stark_holds: bool,
        donoho_stark_distance: f64,
        magnitudes: Vec<f64>,
        trace: Vec<String>,
    },
    DispersedOrAperiodic {
        n_samples: usize,
        dominant_bin: usize,
        estimated_period: f64,
        peak_to_average_ratio: f64,
        temporal_support: usize,
        spectral_support: usize,
        donoho_stark_product: usize,
        donoho_stark_holds: bool,
        donoho_stark_distance: f64,
        magnitudes: Vec<f64>,
        trace: Vec<String>,
    },
    InsufficientWindow {
        n_samples: usize,
        candidate_period: f64,
        min_cycles_required: f64,
        min_samples_required: usize,
        spectral_resolution_df: f64,
        reason: String,
        trace: Vec<String>,
    },
}

pub const MAX_DFT_SIGNAL_LEN: usize = 2048;

/// Calcula la Transformada Discreta de Fourier exacta (DFT): $X_k = \sum_{n=0}^{N-1} x_n e^{-2\pi i k n / N}$.
pub fn compute_dft_exact(signal: &[f64]) -> Vec<Complex> {
    let n = signal.len().min(MAX_DFT_SIGNAL_LEN);
    if n == 0 { return Vec::new(); }
    let signal = &signal[..n];
    let n_f64 = n as f64;
    let mut spectrum = Vec::with_capacity(n);
    for k in 0..n {
        let mut sum = Complex::ZERO;
        for (t, &x_t) in signal.iter().enumerate() {
            sum += Complex::from_polar(x_t, -2.0 * PI * (k as f64) * (t as f64) / n_f64);
        }
        spectrum.push(sum);
    }
    spectrum
}

/// Analiza una secuencia discreta $x \in \mathbb{R}^N$ mediante DFT exacta y Donoho-Stark.
pub fn analyze_spectrum_donoho_stark(
    signal: &[f64],
    support_epsilon: f64,
    min_cycles: f64,
    expected_period_hint: Option<f64>,
) -> SpectralOutcome {
    let n = signal.len();
    let mut trace = Vec::new();
    if !(4..=MAX_DFT_SIGNAL_LEN).contains(&n) {
        let reason = if n < 4 {
            format!("Ventana N={n} corta para análisis (mínimo 4).")
        } else {
            format!("Señal N={n} excede cota MAX_DFT_SIGNAL_LEN={MAX_DFT_SIGNAL_LEN}.")
        };
        trace.push(reason.clone());
        return SpectralOutcome::InsufficientWindow {
            n_samples: n, candidate_period: expected_period_hint.unwrap_or(0.0), min_cycles_required: min_cycles,
            min_samples_required: if n < 4 { 4 } else { MAX_DFT_SIGNAL_LEN }, spectral_resolution_df: if n > 0 { 1.0 / (n as f64) } else { 1.0 }, reason, trace,
        };
    }
    let n_f64 = n as f64;
    let df = 1.0 / n_f64;
    if let Some(hint_p) = expected_period_hint {
        let required_n = (min_cycles * hint_p).ceil() as usize;
        if n < required_n {
            let reason = format!("Abstención por ventana insuficiente: N={n} no cubre min_cycles={min_cycles:.1} de P={hint_p:.2} (req >= {required_n}, df={df:.4}).");
            trace.push(reason.clone());
            return SpectralOutcome::InsufficientWindow {
                n_samples: n, candidate_period: hint_p, min_cycles_required: min_cycles,
                min_samples_required: required_n, spectral_resolution_df: df, reason, trace,
            };
        }
    }

    let spectrum = compute_dft_exact(signal);
    let magnitudes: Vec<f64> = spectrum.iter().map(|c| c.norm()).collect();
    let max_x = signal.iter().fold(0.0_f64, |acc, &v| acc.max(v.abs()));
    let max_spec = magnitudes.iter().fold(0.0_f64, |acc, &m| acc.max(m));
    let temp_thresh = (max_x * support_epsilon).max(1e-9);
    let spec_thresh = (max_spec * support_epsilon).max(1e-9);

    let temporal_support = signal.iter().filter(|&&v| v.abs() > temp_thresh).count();
    let spectral_support = magnitudes.iter().filter(|&&m| m > spec_thresh).count();
    let ds_product = temporal_support * spectral_support;
    let ds_holds = ds_product >= n;
    let ds_distance = (ds_product as f64) / n_f64 - 1.0;

    trace.push(format!("DFT exacta N={n} (df={df:.4}): |sop(x)|={temporal_support}, |sop(X)|={spectral_support}, producto={ds_product} >= {n} ({ds_holds}), D_DS={ds_distance:.4}"));

    let nyquist = n / 2;
    let (mut dominant_bin, mut max_ac_mag, mut sum_ac_mag) = (1, 0.0, 0.0);
    for (k, &mag) in magnitudes.iter().enumerate().take(nyquist + 1).skip(1) {
        sum_ac_mag += mag;
        if mag > max_ac_mag { max_ac_mag = mag; dominant_bin = k; }
    }
    let avg_ac_mag = if nyquist > 0 { sum_ac_mag / (nyquist as f64) } else { 0.0 };
    let par = if avg_ac_mag > 1e-12 { max_ac_mag / avg_ac_mag } else { 1.0 };
    let estimated_period = n_f64 / (dominant_bin as f64);
    trace.push(format!("Frecuencia dominante AC: k*={dominant_bin} (|X_k*|={max_ac_mag:.4}), P={estimated_period:.2}, PAR={par:.4}"));

    let observed_cycles = dominant_bin as f64;
    if observed_cycles + 1e-9 < min_cycles {
        let required_n = (min_cycles * estimated_period).ceil() as usize;
        let reason = format!("Abstención: bin k*={dominant_bin} (P={estimated_period:.2}) solo observa {observed_cycles:.1} ciclos en N={n} (req {min_cycles:.1}, N >= {required_n}).");
        trace.push(reason.clone());
        return SpectralOutcome::InsufficientWindow {
            n_samples: n, candidate_period: estimated_period, min_cycles_required: min_cycles,
            min_samples_required: required_n, spectral_resolution_df: df, reason, trace,
        };
    }

    if par >= 1.5 && ds_distance <= 0.5 {
        trace.push(format!("Veredicto espectral: BEACON PERIÓDICO confirmado (P={estimated_period:.2}, D_DS={ds_distance:.4})."));
        SpectralOutcome::PeriodicBeaconDetected {
            n_samples: n, dominant_bin, estimated_period, peak_to_average_ratio: par, temporal_support, spectral_support,
            donoho_stark_product: ds_product, donoho_stark_holds: ds_holds, donoho_stark_distance: ds_distance, magnitudes, trace,
        }
    } else {
        trace.push(format!("Veredicto espectral: TRÁFICO DISPERSO / APERIÓDICO (PAR={par:.4}, D_DS={ds_distance:.4})."));
        SpectralOutcome::DispersedOrAperiodic {
            n_samples: n, dominant_bin, estimated_period, peak_to_average_ratio: par, temporal_support, spectral_support,
            donoho_stark_product: ds_product, donoho_stark_holds: ds_holds, donoho_stark_distance: ds_distance, magnitudes, trace,
        }
    }
}

/// Calcula la correlación de Pearson $r \in [-1.0, 1.0]$ usando el algoritmo en línea de Welford.
pub fn pearson_correlation_welford(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 2 { return None; }
    let (mut mean_x, mut mean_y, mut m2_x, mut m2_y, mut cov_xy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, (&xi, &yi)) in x.iter().zip(y.iter()).enumerate() {
        let n = (i + 1) as f64;
        let (dx, dy) = (xi - mean_x, yi - mean_y);
        mean_x += dx / n;
        mean_y += dy / n;
        m2_x += dx * (xi - mean_x);
        m2_y += dy * (yi - mean_y);
        cov_xy += dx * (yi - mean_y);
    }
    let denom = (m2_x * m2_y).sqrt();
    if denom <= 1e-15 { None } else { Some((cov_xy / denom).clamp(-1.0, 1.0)) }
}
