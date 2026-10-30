//! # relic_core::spectral_sprt
//!
//! Detector secuencial SPRT con mezcla de hipótesis para beaconing periódico con jitter.
//! Martingala de prueba con familia nula compuesta (Poisson, ráfagas, disperso) con supremo
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SprtConfig {
    pub p_min: f64,
    pub p_max: f64,
    pub p_step: f64,
    pub jitter_fractions: Vec<f64>,
    pub contamination_eps: f64,
    pub delta_max: f64,
}

impl Default for SprtConfig {
    fn default() -> Self {
        Self {
            p_min: 4.0,
            p_max: 50.0,
            p_step: 1.0,
            jitter_fractions: vec![0.0, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50],
            contamination_eps: 0.02,
            delta_max: 100.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SprtOutcome {
    pub lambda_n: f64,
    pub alpha_n: f64,
    pub n_steps: usize,
    pub p_hat: f64,
    pub m_hat: f64,
    pub log_lambda: f64,
}

#[derive(Debug, Clone)]
struct Hypothesis {
    p: f64,
    m: f64,
    log_lik: f64,
}

#[derive(Debug, Clone)]
struct BurstModel {
    w: f64,
    mu1: f64,
    mu2: f64,
    log_lik: f64,
}

#[derive(Debug, Clone)]
pub struct SprtBeaconDetector {
    config: SprtConfig,
    hypotheses: Vec<Hypothesis>,
    burst_models: Vec<BurstModel>,
    n_steps: usize,
    sum_delta: f64,
    sum_ln_delta: f64,
    min_delta: f64,
    max_delta: f64,
}

impl SprtBeaconDetector {
    pub fn new(config: SprtConfig) -> Self {
        let mut hypotheses = Vec::new();
        let mut p = config.p_min;
        while p <= config.p_max + 1e-6 {
            for &jf in &config.jitter_fractions {
                let m = (p * jf).round();
                hypotheses.push(Hypothesis { p, m, log_lik: 0.0 });
            }
            p += config.p_step;
        }
        let burst_models = vec![
            BurstModel { w: 0.5, mu1: 2.0, mu2: 20.0, log_lik: 0.0 },
            BurstModel { w: 0.4, mu1: 1.5, mu2: 35.0, log_lik: 0.0 },
            BurstModel { w: 0.6, mu1: 1.0, mu2: 15.0, log_lik: 0.0 },
        ];
        Self { config, hypotheses, burst_models, n_steps: 0, sum_delta: 0.0, sum_ln_delta: 0.0, min_delta: f64::INFINITY, max_delta: 0.0 }
    }

    pub fn step(&mut self, delta: f64) -> SprtOutcome {
        let delta_clamped = delta.clamp(0.001, self.config.delta_max);
        self.n_steps += 1;
        self.sum_delta += delta_clamped;
        self.sum_ln_delta += delta_clamped.ln();
        self.min_delta = self.min_delta.min(delta_clamped);
        self.max_delta = self.max_delta.max(delta_clamped);
        let eps = self.config.contamination_eps;
        let unif_bg = 1.0 / self.config.delta_max;

        for hyp in &mut self.hypotheses {
            let h = (2.0 * hyp.m).max(0.5);
            let diff = (delta_clamped - hyp.p).abs();
            let tri_density = if diff <= h { (h - diff) / (h * h) } else { 0.0 };
            let f_h1 = (1.0 - eps) * tri_density + eps * unif_bg;
            hyp.log_lik += f_h1.ln();
        }

        for bm in &mut self.burst_models {
            let f1 = (1.0 / bm.mu1) * (-delta_clamped / bm.mu1).exp();
            let f2 = (1.0 / bm.mu2) * (-delta_clamped / bm.mu2).exp();
            let f_burst = (bm.w * f1 + (1.0 - bm.w) * f2).max(1e-12);
            bm.log_lik += f_burst.ln();
        }

        self.outcome()
    }

    pub fn process_intervals(&mut self, intervals: &[f64]) -> SprtOutcome {
        let mut last_outcome = self.outcome();
        for &dt in intervals { last_outcome = self.step(dt); }
        last_outcome
    }

    pub fn process_signal(&mut self, signal: &[f64]) -> SprtOutcome {
        let arrivals: Vec<f64> = signal.iter().enumerate()
            .filter_map(|(idx, &v)| if v > 0.5 { Some(idx as f64) } else { None })
            .collect();
        if arrivals.len() < 2 { return self.outcome(); }
        let intervals: Vec<f64> = arrivals.windows(2).map(|w| w[1] - w[0]).collect();
        self.process_intervals(&intervals)
    }

    pub fn outcome(&self) -> SprtOutcome {
        if self.n_steps == 0 || self.hypotheses.is_empty() {
            return SprtOutcome { lambda_n: 1.0, alpha_n: 1.0, n_steps: 0, p_hat: 0.0, m_hat: 0.0, log_lambda: 0.0 };
        }
        let k_f64 = self.hypotheses.len() as f64;
        let mut max_log_h1 = f64::NEG_INFINITY;
        let mut best_p = self.hypotheses[0].p;
        let mut best_m = self.hypotheses[0].m;
        for hyp in &self.hypotheses {
            if hyp.log_lik > max_log_h1 {
                max_log_h1 = hyp.log_lik;
                best_p = hyp.p;
                best_m = hyp.m;
            }
        }
        let sum_exp: f64 = self.hypotheses.iter().map(|h| (h.log_lik - max_log_h1).exp()).sum();
        let log_p_h1 = max_log_h1 + (sum_exp / k_f64).ln();

        let mean_delta = self.sum_delta / (self.n_steps as f64);
        let log_p_exp = (self.n_steps as f64) * (-1.0 - mean_delta.ln());
        let mut max_log_null = log_p_exp;
        for bm in &self.burst_models {
            if bm.log_lik > max_log_null { max_log_null = bm.log_lik; }
        }
        if self.min_delta > 0.0 && (self.max_delta / self.min_delta) >= 2.5 {
            let xm = self.min_delta.max(0.5);
            let sum_ratio = (self.sum_ln_delta - (self.n_steps as f64) * xm.ln()).max(1e-4);
            let alpha_hat = ((self.n_steps as f64) / sum_ratio).clamp(0.5, 3.5);
            let log_p_pareto = (self.n_steps as f64) * (alpha_hat.ln() + alpha_hat * xm.ln()) - (alpha_hat + 1.0) * self.sum_ln_delta;
            if log_p_pareto > max_log_null { max_log_null = log_p_pareto; }
        }

        let log_unif = -(self.n_steps as f64) * self.config.delta_max.ln();
        if log_unif > max_log_null { max_log_null = log_unif; }

        let log_lambda = log_p_h1 - max_log_null;
        let (lambda_n, alpha_n) = if log_lambda >= 700.0 {
            (f64::INFINITY, 0.0)
        } else if log_lambda <= -700.0 {
            (0.0, 1.0)
        } else {
            let lam = log_lambda.exp();
            (lam, (1.0 / lam).min(1.0))
        };

        SprtOutcome { lambda_n, alpha_n, n_steps: self.n_steps, p_hat: best_p, m_hat: best_m, log_lambda }
    }
}
