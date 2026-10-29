//! # relic_core::secops_guardrail_tests
//!
//! Pruebas de regresión SecOps: validación de cotas defensivas contra OOM,
//! agotamiento de CPU por complejidad cuadrática y saturación de memoria.
//! Menos de 180 líneas.

use std::time::Instant;
use relic_core::kuramoto::{
    simulate_kuramoto_synchronization, KuramotoNetworkSpec, KuramotoOscillator,
    SynchronizationOutcome, MAX_KURAMOTO_STEPS, MAX_OSCILLATORS,
};
use relic_core::spectral::{analyze_spectrum_donoho_stark, compute_dft_exact, SpectralOutcome, MAX_DFT_SIGNAL_LEN};

#[test]
fn test_kuramoto_steps_clamping_prevents_oom_and_cpu_hang() {
    let osc = vec![
        KuramotoOscillator::new("A", 0.0, 1.0),
        KuramotoOscillator::new("B", 0.1, 1.0),
    ];
    let coupling = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
    let spec = KuramotoNetworkSpec {
        oscillators: osc,
        coupling_matrix: coupling,
        steps: 1_000_000_000, // Ataque DoS de mil millones de pasos
        dt: 0.05,
        required_r: 0.8,
        alignment_cos_threshold: 0.7,
    };

    let t0 = Instant::now();
    let outcome = simulate_kuramoto_synchronization(&spec);
    let elapsed = t0.elapsed();

    // Debe acotar a MAX_KURAMOTO_STEPS (10,000) y completar en milisegundos sin OOM
    let trajectory_len = match outcome {
        SynchronizationOutcome::Synchronized { trace, .. } => trace.r_trajectory.len(),
        SynchronizationOutcome::IncoherentAbstention { trace, .. } => trace.r_trajectory.len(),
    };
    assert_eq!(trajectory_len, MAX_KURAMOTO_STEPS + 1);
    assert!(elapsed.as_millis() < 500, "Tardó demasiado: {:?}", elapsed);
}

#[test]
fn test_kuramoto_max_oscillators_bound_rejects_flood() {
    let flood_count = MAX_OSCILLATORS + 50;
    let osc: Vec<KuramotoOscillator> = (0..flood_count)
        .map(|i| KuramotoOscillator::new(format!("node_{i}"), 0.0, 1.0))
        .collect();
    let coupling = vec![vec![0.5; flood_count]; flood_count];
    let spec = KuramotoNetworkSpec {
        oscillators: osc,
        coupling_matrix: coupling,
        steps: 10,
        dt: 0.05,
        required_r: 0.8,
        alignment_cos_threshold: 0.7,
    };

    let outcome = simulate_kuramoto_synchronization(&spec);
    match outcome {
        SynchronizationOutcome::IncoherentAbstention { reason, .. } => {
            assert!(reason.contains("Exceso de osciladores"));
        }
        _ => panic!("Esperaba IncoherentAbstention por exceder MAX_OSCILLATORS"),
    }
}

#[test]
fn test_spectral_signal_ceiling_protects_dft_quadratic_complexity() {
    let oversized_signal = vec![1.0; MAX_DFT_SIGNAL_LEN + 1000];

    // 1. compute_dft_exact acota el soporte a MAX_DFT_SIGNAL_LEN
    let dft = compute_dft_exact(&oversized_signal);
    assert_eq!(dft.len(), MAX_DFT_SIGNAL_LEN);

    // 2. analyze_spectrum_donoho_stark rechaza como InsufficientWindow con aviso de seguridad
    let outcome = analyze_spectrum_donoho_stark(&oversized_signal, 0.05, 3.0, None);
    match outcome {
        SpectralOutcome::InsufficientWindow { reason, .. } => {
            assert!(reason.contains("MAX_DFT_SIGNAL_LEN"));
        }
        _ => panic!("Esperaba InsufficientWindow por exceder longitud máxima de señal"),
    }
}

