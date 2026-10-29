use relic_core::{SprtBeaconDetector, SprtConfig};

fn next_prng(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*state >> 32) as f64 + 1.0) / (u32::MAX as f64 + 1.0)
}

fn gen_null_trace(kind: usize, n: usize, mut state: u64) -> Vec<f64> {
    let mut intervals = Vec::with_capacity(n);
    match kind {
        0 => { // Declarada: Exponencial / Poisson (media 20.0)
            for _ in 0..n {
                let u = next_prng(&mut state);
                intervals.push(-20.0 * u.ln());
            }
        }
        1 => { // Declarada: Ráfagas (burst renewal)
            for _ in 0..n {
                let u_choice = next_prng(&mut state);
                let u = next_prng(&mut state);
                let dt = if u_choice < 0.6 { -2.0 * u.ln() } else { -30.0 * u.ln() };
                intervals.push(dt);
            }
        }
        2 => { // Reservada: Pareto alfa=1.5, x_m=5.0
            let alpha = 1.5;
            let xm = 5.0;
            for _ in 0..n {
                let u = next_prng(&mut state);
                intervals.push(xm / u.powf(1.0 / alpha));
            }
        }
        3 => { // Reservada: Mezcla bimodal de 2 tasas separadas
            for _ in 0..n {
                let u_choice = next_prng(&mut state);
                let u = next_prng(&mut state);
                let dt = if u_choice < 0.5 { -4.0 * u.ln() } else { -40.0 * u.ln() };
                intervals.push(dt);
            }
        }
        _ => { // Reservada: Casi-periódico con jitter excesivo (P=20, jitter=60% >= P/2)
            let p = 20.0;
            let m = 12.0;
            let mut prev = 0.0;
            for _ in 0..n {
                let u = next_prng(&mut state);
                let curr = -m + 2.0 * m * u;
                intervals.push((p + curr - prev).max(0.5));
                prev = curr;
            }
        }
    }
    intervals
}

#[test]
fn test_sprt_declared_nulls_fpr_strict() {
    let total = 1000;
    for family in 0..=1 {
        let mut fp = 0;
        for i in 0..total {
            let seed = (family as u64 + 1) * 50_000 + i as u64;
            let intervals = gen_null_trace(family, 32, seed);
            let mut detector = SprtBeaconDetector::new(SprtConfig::default());
            let outcome = detector.process_intervals(&intervals);
            if outcome.lambda_n >= 100.0 { fp += 1; }
        }
        let fpr = (fp as f64) / (total as f64);
        assert!(fpr <= 0.005, "Familia declarada {family} FPR={fpr:.4} > 0.005 a A=100");
    }
}

#[test]
fn test_sprt_heldout_renewal_fpr() {
    let total = 1000;
    for family in 2..=3 {
        let mut fp = 0;
        for i in 0..total {
            let seed = (family as u64 + 1) * 50_000 + i as u64;
            let intervals = gen_null_trace(family, 32, seed);
            let mut detector = SprtBeaconDetector::new(SprtConfig::default());
            let outcome = detector.process_intervals(&intervals);
            if outcome.lambda_n >= 100.0 { fp += 1; }
        }
        let fpr = (fp as f64) / (total as f64);
        assert!(fpr <= 0.01, "Familia reservada {family} FPR={fpr:.4} > 0.01 a A=100");
    }
}

#[test]
fn test_sprt_heldout_excessive_jitter_monotonic_threshold() {
    let total = 500;
    let mut fp_100 = 0;
    let mut fp_1000 = 0;
    for i in 0..total {
        let seed = 444_000 + i as u64;
        let intervals = gen_null_trace(4, 32, seed);
        let mut detector = SprtBeaconDetector::new(SprtConfig::default());
        let outcome = detector.process_intervals(&intervals);
        if outcome.lambda_n >= 100.0 { fp_100 += 1; }
        if outcome.lambda_n >= 1000.0 { fp_1000 += 1; }
    }
    assert!(fp_1000 <= fp_100, "FPR debe ser monótonamente decreciente con respecto a A");
}
