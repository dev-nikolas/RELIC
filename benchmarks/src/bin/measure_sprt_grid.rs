//! # measure_sprt_grid
//!
//! Benchmark B4 (Grid Jitter 0..50% x Ventana 8..64 ciclos, >= 1000 trazas/celda).
//! Paralelizado con std::thread::scope. Comparación SPRT vs DFT y evaluación B5.
use relic_core::{analyze_spectrum_donoho_stark, SpectralOutcome, SprtBeaconDetector, SprtConfig};
use std::time::Instant;

fn synthesize_trace(period: f64, jitter_frac: f64, cycles: usize, mut seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut intervals = Vec::with_capacity(cycles);
    let total_samples = (period * cycles as f64).round() as usize;
    let mut signal = vec![0.0; total_samples];
    let m = period * jitter_frac;
    let (mut prev_d, mut curr_t): (f64, f64) = (0.0, 0.0);
    for _ in 0..cycles {
        let idx = curr_t.round() as usize;
        if idx < signal.len() { signal[idx] = 1.0; }
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u = ((seed >> 32) as f64 + 1.0) / (u32::MAX as f64 + 1.0);
        let d = -m + 2.0 * m * u;
        let dt = (period + d - prev_d).max(0.5);
        intervals.push(dt);
        curr_t += dt;
        prev_d = d;
    }
    (intervals, signal)
}

fn eval_tube_comb(intervals: &[f64], candidate_p: f64, tube_width: f64) -> bool {
    let mut hits = 0;
    for &dt in intervals { if (dt - candidate_p).abs() <= tube_width { hits += 1; } }
    (hits as f64) / (intervals.len() as f64) >= 0.70
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

struct CellResult {
    alphas: Vec<f64>,
    sprt_100: usize,
    sprt_1k: usize,
    sprt_1m: usize,
    dft_det: usize,
    tube_det: usize,
    p_exact: usize,
    p_harm: usize,
    sprt_ns: u128,
    dft_ns: u128,
}

fn main() {
    println!("=== BENCHMARK B4: GRID JITTER x VENTANA (SPRT vs DFT + Donoho-Stark) ===");
    let jitters = [0.0, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50];
    let windows = [8usize, 16, 32, 64];
    let n_traces = 1000usize;
    let period = 20.0;
    let num_cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

    println!("Jitter | Win | SPRT A=10^2 | A=10^3 | A=10^6 | α_med   | DFT Det | P̂_exact | Tubos | SPRT(µs) | DFT(µs)");
    println!("-------+-----+-------------+--------+--------+---------+---------+---------+-------+----------+--------");

    for &w in &windows {
        for &jit in &jitters {
            let chunk = n_traces / num_cpus;
            let results: Vec<CellResult> = std::thread::scope(|s| {
                let mut handles = Vec::new();
                for t_id in 0..num_cpus {
                    let start = t_id * chunk;
                    let end = if t_id == num_cpus - 1 { n_traces } else { start + chunk };
                    handles.push(s.spawn(move || {
                        let mut r = CellResult {
                            alphas: Vec::with_capacity(end - start),
                            sprt_100: 0, sprt_1k: 0, sprt_1m: 0, dft_det: 0, tube_det: 0,
                            p_exact: 0, p_harm: 0, sprt_ns: 0, dft_ns: 0,
                        };
                        for i in start..end {
                            let seed = ((w as u64) * 10_000 + (jit * 100.0) as u64) * 10_000 + i as u64;
                            let (intervals, signal) = synthesize_trace(period, jit, w, seed);

                            let t0 = Instant::now();
                            let mut det = SprtBeaconDetector::new(SprtConfig::default());
                            let out = det.process_intervals(&intervals);
                            r.sprt_ns += t0.elapsed().as_nanos();

                            r.alphas.push(out.alpha_n);
                            if out.lambda_n >= 100.0 { r.sprt_100 += 1; }
                            if out.lambda_n >= 1000.0 { r.sprt_1k += 1; }
                            if out.lambda_n >= 1_000_000.0 { r.sprt_1m += 1; }
                            if (out.p_hat - period).abs() <= 1.0 { r.p_exact += 1; }
                            else if (out.p_hat - period / 2.0).abs() <= 1.0 || (out.p_hat - 2.0 * period).abs() <= 1.0 {
                                r.p_harm += 1;
                            }

                            let t1 = Instant::now();
                            let dft_res = analyze_spectrum_donoho_stark(&signal, 0.05, 3.0, Some(period));
                            r.dft_ns += t1.elapsed().as_nanos();
                            if matches!(dft_res, SpectralOutcome::PeriodicBeaconDetected { .. }) { r.dft_det += 1; }

                            let tube_width = (2.0 * jit * period).max(1.0);
                            if eval_tube_comb(&intervals, period, tube_width) { r.tube_det += 1; }
                        }
                        r
                    }));
                }
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });

            let mut all_alphas = Vec::with_capacity(n_traces);
            let (mut s100, mut s1k, mut s1m, mut dft, mut tube) = (0, 0, 0, 0, 0);
            let (mut p_ex, mut _p_hm) = (0, 0);
            let (mut sprt_ns, mut dft_ns) = (0u128, 0u128);
            for r in results {
                all_alphas.extend(r.alphas);
                s100 += r.sprt_100; s1k += r.sprt_1k; s1m += r.sprt_1m;
                dft += r.dft_det; tube += r.tube_det;
                p_ex += r.p_exact; _p_hm += r.p_harm;
                sprt_ns += r.sprt_ns; dft_ns += r.dft_ns;
            }

            all_alphas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let alpha_med = percentile(&all_alphas, 0.50);
            let t_sprt_us = (sprt_ns as f64) / (n_traces as f64 * 1000.0);
            let t_dft_us = (dft_ns as f64) / (n_traces as f64 * 1000.0);

            println!("{:>5.0}% | {:>3} | {:>10.1}% | {:>5.1}% | {:>5.1}% | {:>7.1e} | {:>6.1}% | {:>6.1}% | {:>4.1}% | {:>8.1} | {:>7.1}",
                jit * 100.0, w,
                (s100 as f64) / (n_traces as f64) * 100.0,
                (s1k as f64) / (n_traces as f64) * 100.0,
                (s1m as f64) / (n_traces as f64) * 100.0,
                alpha_med,
                (dft as f64) / (n_traces as f64) * 100.0,
                (p_ex as f64) / (n_traces as f64) * 100.0,
                (tube as f64) / (n_traces as f64) * 100.0,
                t_sprt_us, t_dft_us,
            );
        }
    }
}
