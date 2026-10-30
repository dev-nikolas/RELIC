//! Métricas de evaluación para NAB (Numenta Anomaly Benchmark),
//! TSB-UAD (Time Series Benchmark) y curvas AUC-ROC / AUC-PR.

use serde::{Deserialize, Serialize};

/// Perfil de costos para el cómputo de puntaje NAB.
#[derive(Debug, Clone, Copy)]
pub struct NabProfile {
    pub a_tp: f64,
    pub a_fp: f64,
    pub a_fn: f64,
}

impl NabProfile {
    pub const fn standard() -> Self {
        Self { a_tp: 1.0, a_fp: 0.11, a_fn: 1.0 }
    }
    pub const fn low_fp() -> Self {
        Self { a_tp: 1.0, a_fp: 0.22, a_fn: 1.0 }
    }
    pub const fn low_fn() -> Self {
        Self { a_tp: 1.0, a_fp: 0.11, a_fn: 2.0 }
    }
}

/// Resultados consolidados de NAB, TSB y AUC-ROC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NabTsbMetrics {
    pub nab_standard_score: f64,
    pub nab_low_fp_score: f64,
    pub nab_low_fn_score: f64,
    pub auc_roc: f64,
    pub auc_pr: f64,
    pub optimal_f1: f64,
    pub precision_at_opt: f64,
    pub recall_at_opt: f64,
    pub mean_detection_delay_steps: f64,
    pub mean_latency_us: f64,
    pub p99_latency_us: f64,
    pub total_evaluated_points: usize,
    pub total_anomalies_tested: usize,
}

/// Cómputo del puntaje normalizado NAB (Numenta Anomaly Benchmark).
pub fn compute_nab_score(
    windows: &[(usize, usize)],
    detections: &[usize],
    probation_ranges: &[(usize, usize)],
    profile: NabProfile,
) -> f64 {
    if windows.is_empty() {
        return 100.0;
    }
    let total_windows = windows.len() as f64;
    let s_null = -profile.a_fn * total_windows;
    let s_perfect = profile.a_tp * total_windows;

    let mut window_hit = vec![false; windows.len()];
    let mut raw_score = 0.0;

    for &det_idx in detections {
        if probation_ranges.iter().any(|&(s, e)| det_idx >= s && det_idx < e) {
            continue;
        }
        let mut inside_window = false;
        for (w_i, &(start, end)) in windows.iter().enumerate() {
            if det_idx >= start && det_idx <= end {
                inside_window = true;
                if !window_hit[w_i] {
                    window_hit[w_i] = true;
                    let width = (end.saturating_sub(start)).max(1) as f64;
                    let rel_pos = (det_idx - start) as f64;
                    let y = 2.0 * (rel_pos / width) - 1.0;
                    let sig = 2.0 / (1.0 + (5.0 * y).exp()) - 1.0;
                    raw_score += profile.a_tp * sig;
                }
                break;
            }
        }
        if !inside_window {
            raw_score -= profile.a_fp;
        }
    }

    for &hit in &window_hit {
        if !hit {
            raw_score -= profile.a_fn;
        }
    }

    let denom = (s_perfect - s_null).max(1e-9);
    (100.0 * (raw_score - s_null) / denom).clamp(0.0, 100.0)
}

/// Cálculo de curva ROC, AUC-ROC y AUC-PR mediante integración trapezoidal.
pub fn compute_auc_metrics(
    scores: &[f64],
    labels: &[bool],
) -> (f64, f64, f64, f64, f64) {
    let total_p = labels.iter().filter(|&&b| b).count() as f64;
    let total_n = (labels.len() as f64 - total_p).max(1.0);
    if total_p == 0.0 {
        return (1.0, 1.0, 1.0, 1.0, 1.0);
    }

    let num_thresholds = 300;
    let mut roc_points = Vec::with_capacity(num_thresholds + 4);
    let mut pr_points = Vec::with_capacity(num_thresholds + 4);

    roc_points.push((0.0, 0.0));
    roc_points.push((1.0, 1.0));

    let mut best_f1 = 0.0;
    let mut best_prec = 0.0;
    let mut best_rec = 0.0;

    for i in 0..=num_thresholds {
        let tau = i as f64 / num_thresholds as f64;
        let mut tp: f64 = 0.0;
        let mut fp: f64 = 0.0;
        let mut fn_cnt: f64 = 0.0;

        for (&s, &l) in scores.iter().zip(labels.iter()) {
            let pred = s >= tau;
            match (pred, l) {
                (true, true) => tp += 1.0,
                (true, false) => fp += 1.0,
                (false, true) => fn_cnt += 1.0,
                (false, false) => {}
            }
        }

        let tpr = tp / (tp + fn_cnt).max(1.0);
        let fpr = fp / total_n;
        let prec = if tp + fp > 0.0 { tp / (tp + fp) } else { 1.0 };
        let rec = tpr;

        let f1 = if prec + rec > 0.0 {
            2.0 * prec * rec / (prec + rec)
        } else {
            0.0
        };

        if f1 > best_f1 {
            best_f1 = f1;
            best_prec = prec;
            best_rec = rec;
        }

        roc_points.push((fpr, tpr));
        pr_points.push((rec, prec));
    }

    roc_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
    let mut auc_roc = 0.0;
    for w in roc_points.windows(2) {
        let (fpr0, tpr0) = w[0];
        let (fpr1, tpr1) = w[1];
        let dx = fpr1 - fpr0;
        let avg_y = (tpr0 + tpr1) / 2.0;
        auc_roc += dx * avg_y;
    }

    pr_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut auc_pr = 0.0;
    for w in pr_points.windows(2) {
        let (r0, p0) = w[0];
        let (r1, p1) = w[1];
        let dx = r1 - r0;
        let avg_y = (p0 + p1) / 2.0;
        auc_pr += dx * avg_y;
    }

    (auc_roc.clamp(0.0, 1.0), auc_pr.clamp(0.0, 1.0), best_f1, best_prec, best_rec)
}
