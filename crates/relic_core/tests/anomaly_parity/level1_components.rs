//! Paridad de componentes individuales (Nivel 1) contra vectores dorados de 500 pasos.

use super::load_500_golden_steps;
use relic_core::anomaly::ecdf::EcdfWindow;
use relic_core::anomaly::echo_state::EchoStatePredictor;
use relic_core::anomaly::peak_decay::StreamingPeakDecay;
use relic_core::anomaly::refractory::RefractorySuppressor;
use relic_core::anomaly::spatial_bypass::SpatialSpikeBypass;
use relic_core::anomaly::{load_weights_from_bytes, WEIGHTS_SEED42_BYTES};

#[test]
fn test_ecdf_parity_level1() {
    let steps = load_500_golden_steps();
    let mut ecdf = EcdfWindow::new();
    let mut checked_calibrated_steps = 0;

    for step in &steps {
        let rust_score = ecdf.score(step.err);
        let diff = (rust_score - step.cal_score).abs();
        assert!(
            diff <= 1e-12,
            "Paso {}: divergencia en ECDF score. Rust={}, Py={}, diff={:e}",
            step.t, rust_score, step.cal_score, diff
        );
        if step.t >= 288 && step.err > 0.0 {
            checked_calibrated_steps += 1;
        }
        ecdf.push(step.err);
    }
    assert!(checked_calibrated_steps > 100, "Debe haber suficientes pasos calibrados evaluados");
}

#[test]
fn test_peak_decay_nms_parity_level1() {
    let steps = load_500_golden_steps();
    let mut decay = StreamingPeakDecay::default_config();

    for step in &steps {
        let (is_peak, nms_score) = decay.evaluate_pure(step.cal_score);
        assert_eq!(is_peak, step.is_peak, "Paso {}: discrepancia en is_peak", step.t);
        let diff = (nms_score - step.nms_score).abs();
        assert!(diff <= 1e-12, "Paso {}: divergencia en NMS score diff={:e}", step.t, diff);
        decay.commit(step.cal_score, is_peak);
    }
}

#[test]
fn test_spatial_spike_bypass_parity_level1() {
    let steps = load_500_golden_steps();
    let mut spatial = SpatialSpikeBypass::default_config();

    for step in &steps {
        let is_spatial = spatial.check_anomaly(step.raw_val);
        assert_eq!(is_spatial, step.is_spatial, "Paso {}: discrepancia en is_spatial", step.t);
        spatial.commit(step.raw_val);
    }
}

#[test]
fn test_refractory_suppressor_parity_level1() {
    let steps = load_500_golden_steps();
    let mut refr = RefractorySuppressor::default_config();

    for step in &steps {
        let raw_detector_score = if step.is_spatial { 1.0 } else { step.nms_score };
        let (_suppressed, score_nab) = refr.evaluate_pure(raw_detector_score);
        let diff = (score_nab - step.score_nab).abs();
        assert!(diff <= 1e-12, "Paso {}: divergencia en score_nab diff={:e}", step.t, diff);
        refr.commit(raw_detector_score);
    }
}

#[test]
fn test_echo_state_teacher_forcing_level1() {
    let steps = load_500_golden_steps();
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut esn = EchoStatePredictor::new();

    for step in &steps {
        let (z, vec_opt, h_cand, pred_z, err) = esn.evaluate_pure(step.raw_val, &win, &wres);
        assert!((z - step.z).abs() <= 1e-9, "Paso {}: divergencia en z", step.t);
        assert_eq!(vec_opt.is_some(), step.ready, "Paso {}: discrepancia en Takens ready", step.t);
        if step.ready {
            assert!((pred_z - step.pred_z).abs() <= 1e-9, "Paso {}: divergencia en pred_z", step.t);
            assert!((err - step.err).abs() <= 1e-9, "Paso {}: divergencia en err", step.t);
        }
        esn.commit(step.raw_val, step.z, step.ready, h_cand, pred_z);
    }
}

