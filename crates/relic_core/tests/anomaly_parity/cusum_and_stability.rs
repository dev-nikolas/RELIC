//! Pruebas de deriva estadística CUSUM (ARL_0 / ARL_1) y estabilidad numérica ante offsets.

use relic_core::anomaly::cusum::TelemetryDriftMonitor;

#[test]
fn test_telemetry_drift_monitor_cusum() {
    let mut drift = TelemetryDriftMonitor::new();
    drift.calibrate(50.0, 2.0);

    for i in 0..50 {
        let noise = ((i % 5) as f64 - 2.0) * 0.5;
        assert!(!drift.update(50.0 + noise));
    }

    let mut alarmed = false;
    let mut alarm_step = 0;
    for step in 1..=30 {
        if drift.update(50.0 + (step as f64) * 0.4) {
            alarmed = true;
            alarm_step = step;
            break;
        }
    }
    assert!(alarmed);
    assert!(alarm_step <= 20);
}

#[test]
fn test_rolling_norm_large_offset_cancellation() {
    let offset = 1_000_000_000.0f64;
    let samples = [offset + 1.0, offset + 2.0, offset + 3.0, offset + 4.0, offset + 5.0];
    let n = samples.len() as f64;
    let true_var = 2.0f64;

    let mut centered_sum = 0.0f64;
    let mut centered_sum_sq = 0.0f64;
    for &x in &samples {
        let dx = x - offset;
        centered_sum += dx;
        centered_sum_sq += dx * dx;
    }
    let centered_mean = centered_sum / n;
    let centered_var = (centered_sum_sq / n) - (centered_mean * centered_mean);
    let centered_err = (centered_var - true_var).abs();

    assert!(centered_err <= 1e-14);
}

#[test]
fn test_cusum_arl0_statistical_simulation() {
    let mut drift = TelemetryDriftMonitor::new();
    drift.calibrate(0.0, 1.0);

    let mut state = 0x123456789abcdef0u64;
    let mut next_u01 = || -> f64 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 11) as f64 / ((1u64 << 53) as f64)
    };

    let mut false_alarms = 0usize;
    for _ in 0..20_000 {
        let u1 = next_u01().max(1e-12);
        let u2 = next_u01();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        if drift.update(z) {
            false_alarms += 1;
            drift.reset();
            drift.calibrate(0.0, 1.0);
        }
    }
    assert!(false_alarms <= 1);

    let mut steps_to_alarm = 0usize;
    let mut detected = false;
    for step in 1..=50 {
        let u1 = next_u01().max(1e-12);
        let u2 = next_u01();
        let z = 1.0 + (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        if drift.update(z) {
            steps_to_alarm = step;
            detected = true;
            break;
        }
    }
    assert!(detected);
    assert!((15..=35).contains(&steps_to_alarm));
}

#[test]
fn test_drift_monitor_bidirectional_and_calibration() {
    let mut drift = TelemetryDriftMonitor::new();
    drift.calibrate(100.0, 5.0);

    for i in 0..500 {
        let noise = ((i % 11) as f64 - 5.0) * 0.8;
        assert!(!drift.update(100.0 + noise));
    }

    let mut alarmed_neg = false;
    let mut alarm_step_neg = 0;
    for step in 1..=40 {
        if drift.update(100.0 - (step as f64) * 0.5) {
            alarmed_neg = true;
            alarm_step_neg = step;
            break;
        }
    }
    assert!(alarmed_neg);
    assert!(alarm_step_neg <= 25);

    drift.reset();
    drift.calibrate(200.0, 10.0);
    for i in 0..100 {
        assert!(!drift.update(200.0 + (i % 5) as f64));
    }
}

