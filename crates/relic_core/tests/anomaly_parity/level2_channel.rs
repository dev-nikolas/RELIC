//! Paridad de canal completo en modo libre (Nivel 2) y robustez ante entradas patológicas.

use super::{load_1000_golden_steps, load_500_golden_steps};
use relic_core::anomaly::channel::SixEyesChannel;
use relic_core::anomaly::{load_weights_from_bytes, WEIGHTS_SEED42_BYTES};

#[test]
fn test_six_eyes_channel_free_running_level2() {
    let steps = load_500_golden_steps();
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut channel = SixEyesChannel::new();

    for step in &steps {
        let assess = channel.evaluate_pure(step.raw_val, &win, &wres);
        let nab_diff = (assess.score_nab - step.score_nab).abs();
        assert!(
            nab_diff <= 1e-6,
            "Paso {}: divergencia libre en score_nab. Rust={}, Py={}, diff={:e}",
            step.t, assess.score_nab, step.score_nab, nab_diff
        );
        channel.commit(step.raw_val, &assess);
    }
}

#[test]
fn test_1000_steps_full_parity_with_alerts_and_spikes() {
    let steps = load_1000_golden_steps();
    assert_eq!(steps.len(), 1000, "La traza debe contener 1000 pasos");

    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut channel = SixEyesChannel::new();

    let mut count_alerts = 0;
    let mut count_spatials = 0;
    let mut count_suppressed = 0;

    for step in &steps {
        let assess = channel.evaluate_pure(step.raw_val, &win, &wres);
        if assess.score_nab >= 0.990 {
            count_alerts += 1;
        }
        if assess.range_violation {
            count_spatials += 1;
        }
        if assess.is_refractory_suppressed {
            count_suppressed += 1;
        }
        let diff = (assess.score_nab - step.score_nab).abs();
        assert!(
            diff <= 1e-6,
            "Paso {}: divergencia en 1000 pasos. Rust={}, Py={}, diff={:e}",
            step.t, assess.score_nab, step.score_nab, diff
        );
        channel.commit(step.raw_val, &assess);
    }

    assert!(count_alerts >= 2, "Debe haber al menos 2 alertas >= 0.990");
    assert!(count_spatials >= 1, "Debe haber al menos 1 ruptura espacial");
    assert!(count_suppressed >= 50, "Debe haber periodos refractarios de silencio activo");
}

#[test]
fn test_anomaly_pipeline_panic_free_on_pathological_inputs() {
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut channel = SixEyesChannel::new();

    let pathological = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        0.0,
        -0.0,
        1e300,
        -1e300,
    ];

    for &val in &pathological {
        let assess = channel.evaluate_pure(val, &win, &wres);
        assert!(assess.score_nab.is_finite());
        assert!(assess.score_relic.is_finite());
        channel.commit(val, &assess);
    }
}

