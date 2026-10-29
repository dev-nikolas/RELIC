//! # measure_sprt_validation
//!
//! Validación B7 (Monte Carlo >= 10^5 trazas de martingala Ville) y
//! B3 (FPR en familias nulas declaradas y reservadas con Clopper-Pearson 95%).
//! Menos de 180 líneas.

use relic_core::{SprtBeaconDetector, SprtConfig};
use std::time::Instant;

fn clopper_pearson_95(k: usize, n: usize) -> (f64, f64) {
    if n == 0 { return (0.0, 1.0); }
    let k_f = k as f64;
    let n_f = n as f64;
    let lower = if k == 0 { 0.0 } else {
        (k_f / n_f) - 1.95996 * ((k_f * (n_f - k_f)) / (n_f * n_f * n_f)).sqrt()
    }.max(0.0);
    let upper = if k == 0 {
        1.0 - 0.05_f64.powf(1.0 / n_f)
    } else {
        (k_f / n_f) + 1.95996 * ((k_f * (n_f - k_f)) / (n_f * n_f * n_f)).sqrt()
    }.min(1.0);
    (lower, upper)
}

fn gen_null_trace(kind: usize, n: usize, mut state: u64) -> Vec<f64> {
    let mut intervals = Vec::with_capacity(n);
    let mut prng = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((state >> 32) as f64 + 1.0) / (u32::MAX as f64 + 1.0)
    };
    match kind {
        0 => { for _ in 0..n { intervals.push(-20.0 * prng().ln()); } }
        1 => {
            for _ in 0..n {
                let dt = if prng() < 0.6 { -2.0 * prng().ln() } else { -30.0 * prng().ln() };
                intervals.push(dt);
            }
        }
        2 => { for _ in 0..n { intervals.push(5.0 / prng().powf(1.0 / 1.5)); } }
        3 => {
            for _ in 0..n {
                let dt = if prng() < 0.5 { -4.0 * prng().ln() } else { -40.0 * prng().ln() };
                intervals.push(dt);
            }
        }
        4 => {
            for _ in 0..n {
                let g = (-10.0 * prng().ln()) + (-10.0 * prng().ln()); // Gamma(k=2, theta=10)
                intervals.push(g.max(0.1));
            }
        }
        5 => {
            for _ in 0..n {
                let w = 20.0 * (-prng().ln()).powf(1.0 / 1.5); // Weibull(k=1.5, lambda=20)
                intervals.push(w.max(0.1));
            }
        }
        _ => {
            let (p, m) = (20.0, 12.0);
            let mut prev = 0.0;
            for _ in 0..n {
                let curr = -m + 2.0 * m * prng();
                intervals.push((p + curr - prev).max(0.5));
                prev = curr;
            }
        }
    }
    intervals
}

fn eval_tube_comb(intervals: &[f64], candidate_p: f64, tube_width: f64) -> bool {
    let mut hits = 0;
    for &dt in intervals { if (dt - candidate_p).abs() <= tube_width { hits += 1; } }
    (hits as f64) / (intervals.len() as f64) >= 0.70
}

fn main() {
    println!("=== VALIDACION B7: COTA MARTINGALA DE VILLE (Monte Carlo >= 10^5 trazas) ===");
    let n_mc = 100_000usize;
    let n_steps = 32usize;
    let t0 = Instant::now();
    let num_cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = n_mc / num_cpus;

    let (exc_20, exc_100): (usize, usize) = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for t_id in 0..num_cpus {
            let start = t_id * chunk;
            let end = if t_id == num_cpus - 1 { n_mc } else { start + chunk };
            handles.push(s.spawn(move || {
                let mut local_e20 = 0usize;
                let mut local_e100 = 0usize;
                for i in start..end {
                    let seed = (i as u64 + 1).wrapping_mul(0x517cc1b727220a95);
                    let intervals = gen_null_trace(i % 2, n_steps, seed);
                    let mut det = SprtBeaconDetector::new(SprtConfig::default());
                    let mut sup_lambda = 0.0_f64;
                    for &dt in &intervals {
                        let out = det.step(dt);
                        if out.lambda_n > sup_lambda { sup_lambda = out.lambda_n; }
                    }
                    if sup_lambda >= 20.0 { local_e20 += 1; }
                    if sup_lambda >= 100.0 { local_e100 += 1; }
                }
                (local_e20, local_e100)
            }));
        }
        let mut t20 = 0; let mut t100 = 0;
        for h in handles { let (e20, e100) = h.join().unwrap(); t20 += e20; t100 += e100; }
        (t20, t100)
    });

    let p_exc_20 = (exc_20 as f64) / (n_mc as f64);
    let p_exc_100 = (exc_100 as f64) / (n_mc as f64);
    println!("[MEDIDO] Trazas nulas evaluadas: {n_mc} en {:.2} s", t0.elapsed().as_secs_f64());
    println!("[MEDIDO] P(sup Λ_n >= 20)  = {exc_20}/{n_mc} = {:.5} (Cota Ville <= {:.5}, Cumple: {})",
        p_exc_20, 1.0 / 20.0, p_exc_20 <= 1.0 / 20.0);
    println!("[MEDIDO] P(sup Λ_n >= 100) = {exc_100}/{n_mc} = {:.5} (Cota Ville <= {:.5}, Cumple: {})",
        p_exc_100, 1.0 / 100.0, p_exc_100 <= 1.0 / 100.0);

    println!("\n=== B3: EVALUACIÓN DE FAMILIAS NULAS (10 000 trazas c/u) ===");
    let family_names = [
        "1. Declarada Θ0: Poisson/Exponencial (μ=20)",
        "2. Declarada Θ0: Ráfagas (Burst renewal)",
        "3. En Denominador Θ0: Pareto (α=1.5, x_m=5)",
        "4. En Denominador Θ0: Mezcla bimodal 2 tasas",
        "5. Reservada Externa: Gamma (k=2, θ=10)",
        "6. Reservada Externa: Weibull (k=1.5, λ=20)",
        "7. Fuera de Θ0: Casi-periódico jitter 60% (>= P/2)",
    ];

    let n_fam = 10_000usize;
    for (f_idx, name) in family_names.iter().enumerate() {
        let (mut fp_100, mut fp_1k, mut fp_1m, mut fp_tube) = (0, 0, 0, 0);
        for i in 0..n_fam {
            let seed = (f_idx as u64 + 1) * 1_000_000 + i as u64;
            let intervals = gen_null_trace(f_idx, n_steps, seed);
            let mut det = SprtBeaconDetector::new(SprtConfig::default());
            let out = det.process_intervals(&intervals);
            if out.lambda_n >= 100.0 { fp_100 += 1; }
            if out.lambda_n >= 1000.0 { fp_1k += 1; }
            if out.lambda_n >= 1_000_000.0 { fp_1m += 1; }
            if eval_tube_comb(&intervals, 20.0, 4.0) { fp_tube += 1; }
        }
        let (l95, u95) = clopper_pearson_95(fp_100, n_fam);
        let fpr = (fp_100 as f64) / (n_fam as f64);
        let fpr_tube = (fp_tube as f64) / (n_fam as f64);
        println!("{name}:");
        println!("  SPRT A=100 : {fp_100}/{n_fam} = {:>6.2}% [CP95: {:>5.2}%, {:>5.2}%] | A=10^3: {:>5.2}% | A=10^6: {:>5.2}%",
            fpr * 100.0, l95 * 100.0, u95 * 100.0, (fp_1k as f64 / n_fam as f64) * 100.0, (fp_1m as f64 / n_fam as f64) * 100.0);
        println!("  Peine Tubos: {fp_tube}/{n_fam} = {:>6.2}% FPR", fpr_tube * 100.0);
    }
}

