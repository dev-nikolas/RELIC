//! Pruebas de expulsión FIFO en ventanas acotadas y simulación de horizonte largo (25.000 pasos).

use relic_core::anomaly::channel::SixEyesChannel;
use relic_core::anomaly::ecdf::EcdfWindow;
use relic_core::anomaly::echo_state::RollingNorm;
use relic_core::anomaly::{load_weights_from_bytes, WEIGHTS_SEED42_BYTES};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenEcdfEvictionStep {
    t: usize,
    raw: f64,
    score: f64,
    hist_len: usize,
}

#[test]
fn test_ecdf_and_norm_window_eviction() {
    let mut ecdf = EcdfWindow::with_capacity(30, 10);
    for i in 1..=100 {
        let val = (i as f64 * 17.0) % 97.0 + 1.0;
        ecdf.push(val);
        if i >= 30 {
            assert_eq!(ecdf.len(), 30);
        }
        let (menores, iguales, n) = ecdf.rank(val);
        assert_eq!(n, ecdf.len());
        assert!(iguales >= 1);
        assert!(menores + iguales <= n);
    }

    let mut norm = RollingNorm::new(25);
    for i in 1..=100 {
        norm.ingest(i as f64);
        if i >= 25 {
            assert_eq!(norm.len(), 25);
        }
    }
    let std_val = norm.standardize(88.0);
    assert!(std_val.abs() < 1e-10);
}

#[test]
fn test_25000_full_scale_window_eviction_and_refractory_cycle() {
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut channel = SixEyesChannel::new();

    let total_steps = 25_000usize;
    let mut total_alerts = 0usize;
    let mut total_refractory_steps = 0usize;
    let injection_steps = [1_000, 5_000, 10_000, 17_000, 22_000];

    for step in 1..=total_steps {
        let is_spike = injection_steps.contains(&step);
        let val = if is_spike {
            500.0
        } else {
            25.0 + 3.0 * ((step as f64) * 0.03).sin() + (step % 5) as f64 * 0.1
        };

        let assess = channel.evaluate_pure(val, &win, &wres);
        if assess.score_relic >= 0.990 {
            total_alerts += 1;
        }
        if assess.is_refractory_suppressed {
            total_refractory_steps += 1;
            assert_eq!(assess.score_nab, 0.0);
        }
        channel.commit(val, &assess);

        if step >= 6_400 {
            assert_eq!(channel.esn.norm.len(), 6_400);
        }
        if step >= 16_015 {
            assert_eq!(channel.ecdf.len(), 16_000);
        }
    }

    assert!(total_alerts >= 5);
    assert!(total_refractory_steps >= 5 * 75);
    assert!(channel.ecdf.is_sorted());
}

#[test]
fn test_ecdf_eviction_parity_against_python() {
    let json_bytes = include_bytes!("../../src/anomaly/data/golden_ecdf_eviction_1200.json");
    let steps: Vec<GoldenEcdfEvictionStep> = serde_json::from_slice(json_bytes)
        .expect("Error al deserializar golden_ecdf_eviction_1200.json");

    let mut ecdf = EcdfWindow::with_capacity(300, 50);
    let mut eviction_count = 0usize;

    for step in &steps {
        let rust_score = ecdf.score(step.raw);
        let diff = (rust_score - step.score).abs();
        assert!(diff <= 1e-12, "Paso {}: divergencia tras expulsión diff={:e}", step.t, diff);
        ecdf.push(step.raw);
        if step.t >= 300 {
            eviction_count += 1;
            assert_eq!(ecdf.len(), 300);
        }
        assert_eq!(ecdf.len(), step.hist_len);
        assert!(ecdf.is_sorted());
    }
    assert_eq!(eviction_count, 900);
}

