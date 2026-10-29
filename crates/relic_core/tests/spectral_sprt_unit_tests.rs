use relic_core::{SprtBeaconDetector, SprtConfig};

fn synthesize_test_beacon(period: f64, jitter_frac: f64, n_cycles: usize, seed: u64) -> Vec<f64> {
    let mut intervals = Vec::with_capacity(n_cycles);
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let m = period * jitter_frac;
    let mut prev_delta = 0.0;
    for _ in 0..n_cycles {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u = ((state >> 32) as f64) / (u32::MAX as f64);
        let curr_delta = -m + 2.0 * m * u;
        let dt = (period + curr_delta - prev_delta).max(0.5);
        intervals.push(dt);
        prev_delta = curr_delta;
    }
    intervals
}

#[test]
fn test_sprt_zero_jitter_rapid_convergence() {
    let mut detector = SprtBeaconDetector::new(SprtConfig::default());
    let intervals = synthesize_test_beacon(20.0, 0.0, 16, 42);
    let outcome = detector.process_intervals(&intervals);

    assert_eq!(outcome.n_steps, 16);
    assert!(outcome.lambda_n > 1000.0, "Lambda={}, expected > 1000", outcome.lambda_n);
    assert!(outcome.alpha_n < 0.001, "Alpha={}, expected < 0.001", outcome.alpha_n);
    assert_eq!(outcome.p_hat.round(), 20.0);
    assert_eq!(outcome.m_hat.round(), 0.0);
}

#[test]
fn test_sprt_moderate_jitter_25pct_detection() {
    let mut detector = SprtBeaconDetector::new(SprtConfig::default());
    let intervals = synthesize_test_beacon(20.0, 0.25, 32, 12345);
    let outcome = detector.process_intervals(&intervals);

    assert_eq!(outcome.n_steps, 32);
    assert!(outcome.lambda_n > 100.0, "Lambda={}, expected > 100 for 25% jitter", outcome.lambda_n);
    assert!(outcome.alpha_n < 0.01);
    assert!((outcome.p_hat - 20.0).abs() <= 1.0);
    assert!(outcome.m_hat >= 3.0 && outcome.m_hat <= 7.0);
}

#[test]
fn test_sprt_high_jitter_40pct_detection() {
    let mut detector = SprtBeaconDetector::new(SprtConfig::default());
    let intervals = synthesize_test_beacon(20.0, 0.40, 48, 99999);
    let outcome = detector.process_intervals(&intervals);

    assert_eq!(outcome.n_steps, 48);
    assert!(outcome.lambda_n > 100.0, "Lambda={}, expected > 100 for 40% jitter", outcome.lambda_n);
    assert!(outcome.alpha_n <= 0.01);
    assert!((outcome.p_hat - 20.0).abs() <= 2.0);
}

#[test]
fn test_sprt_no_internal_decision() {
    let mut detector = SprtBeaconDetector::new(SprtConfig::default());
    let intervals = synthesize_test_beacon(20.0, 0.10, 8, 777);
    let outcome = detector.process_intervals(&intervals);

    assert!(outcome.lambda_n > 0.0);
    assert!(outcome.alpha_n > 0.0 && outcome.alpha_n <= 1.0);
    assert_eq!(outcome.alpha_n, (1.0 / outcome.lambda_n).min(1.0));
}

#[test]
fn test_sprt_process_signal_equivalence() {
    let mut sig = vec![0.0; 200];
    for c in 0..10 {
        sig[c * 20] = 1.0;
    }
    let mut detector = SprtBeaconDetector::new(SprtConfig::default());
    let outcome = detector.process_signal(&sig);
    assert_eq!(outcome.n_steps, 9);
    assert_eq!(outcome.p_hat.round(), 20.0);
    assert!(outcome.lambda_n > 50.0);
}
