//! # relic_lexicon::calibrator
//!
//! Auto-Calibrador de Fase sin ángulos manuales (`auto_calibrate_phases`).
//! Aprende amplitudes $r_k$ (con normalización unitaria $\sum |\alpha_k|^2 = 1$)
//! y fases de interferencia ($\theta = 0$ constructiva para la operación observada
//! y $\theta = \pi$ destructiva hacia las operaciones competidoras) a partir de
//! observaciones empíricas `(palabra_ambigua, token_contexto, operacion_objetivo)`.
//! Menos de 180 líneas.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;
use relic_core::ArithmeticOp;
use serde::{Deserialize, Serialize};
use crate::log::LexiconPayload;

/// Observación contextual cruda para entrenar/calibrar fases de un término polisémico sin ángulos manuales.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextObservation {
    pub ambiguous_word: String,
    pub context_token: String,
    pub target_op: ArithmeticOp,
}

impl ContextObservation {
    pub fn new(
        ambiguous_word: impl Into<String>,
        context_token: impl Into<String>,
        target_op: ArithmeticOp,
    ) -> Self {
        Self {
            ambiguous_word: ambiguous_word.into(),
            context_token: context_token.into(),
            target_op,
        }
    }
}

/// Reporte determinista del proceso de auto-calibración de fase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseCalibrationReport {
    pub ambiguous_word: String,
    pub competing_ops: Vec<ArithmeticOp>,
    pub prior_amplitudes: Vec<(ArithmeticOp, f64)>,
    pub synthesized_payloads: Vec<LexiconPayload>,
}

fn op_order(op: ArithmeticOp) -> u8 {
    match op {
        ArithmeticOp::Add => 0,
        ArithmeticOp::Subtract => 1,
        ArithmeticOp::Multiply => 2,
        ArithmeticOp::Divide => 3,
    }
}

/// Sintetiza automáticamente entradas `QuantumPolysemousAlias` (priors normalizados + desplazamientos de fase
/// constructivos $\theta = 0$ y destructivos $\theta = \pi$) a partir de observaciones crudas.
pub fn auto_calibrate_phases(observations: &[ContextObservation]) -> Vec<PhaseCalibrationReport> {
    let mut by_word: BTreeMap<String, Vec<&ContextObservation>> = BTreeMap::new();
    for obs in observations {
        let w = obs.ambiguous_word.trim().to_lowercase();
        if !w.is_empty() {
            by_word.entry(w).or_default().push(obs);
        }
    }

    let mut reports = Vec::new();
    let laplace_alpha = 0.5_f64;

    for (word, obs_list) in by_word {
        let mut op_counts: BTreeMap<u8, (ArithmeticOp, usize)> = BTreeMap::new();
        let mut ctx_op_counts: BTreeMap<String, BTreeMap<u8, (ArithmeticOp, usize)>> = BTreeMap::new();

        for obs in &obs_list {
            let ord = op_order(obs.target_op);
            op_counts.entry(ord).or_insert((obs.target_op, 0)).1 += 1;
            let ctx = obs.context_token.trim().to_lowercase();
            if !ctx.is_empty() {
                ctx_op_counts
                    .entry(ctx)
                    .or_default()
                    .entry(ord)
                    .or_insert((obs.target_op, 0))
                    .1 += 1;
            }
        }

        let competing_ops: Vec<ArithmeticOp> = op_counts.values().map(|(op, _)| *op).collect();
        if competing_ops.is_empty() {
            continue;
        }

        let k_ops = competing_ops.len() as f64;
        let total_obs = obs_list.len() as f64;
        let denom = total_obs + laplace_alpha * k_ops;

        let mut prior_amplitudes = Vec::with_capacity(competing_ops.len());
        let mut payloads = Vec::new();

        for &op in &competing_ops {
            let c = op_counts.get(&op_order(op)).map_or(0, |(_, cnt)| *cnt) as f64;
            let prob_smoothed = (c + laplace_alpha) / denom;
            let r_k = prob_smoothed.sqrt();
            prior_amplitudes.push((op, r_k));
            payloads.push(LexiconPayload::quantum_polysemous_alias(
                word.clone(),
                op,
                r_k,
                0.0,
                None,
            ));
        }

        for (ctx_tok, counts_for_ctx) in ctx_op_counts {
            let mut best_ord = 0u8;
            let mut best_op = competing_ops[0];
            let mut best_cnt = 0usize;
            for (&ord, &(op, cnt)) in &counts_for_ctx {
                if cnt > best_cnt || (cnt == best_cnt && ord < best_ord) {
                    best_cnt = cnt;
                    best_op = op;
                    best_ord = ord;
                }
            }

            let prior_r_best = prior_amplitudes
                .iter()
                .find(|(op, _)| *op == best_op)
                .map_or((1.0 / k_ops).sqrt(), |(_, r)| *r);

            payloads.push(LexiconPayload::quantum_polysemous_alias(
                word.clone(),
                best_op,
                prior_r_best,
                0.0,
                Some(ctx_tok.clone()),
            ));

            let mut seen_competitors = BTreeSet::new();
            for &comp_op in &competing_ops {
                if comp_op != best_op && seen_competitors.insert(op_order(comp_op)) {
                    let prior_r_comp = prior_amplitudes
                        .iter()
                        .find(|(op, _)| *op == comp_op)
                        .map_or((1.0 / k_ops).sqrt(), |(_, r)| *r);
                    payloads.push(LexiconPayload::quantum_polysemous_alias(
                        word.clone(),
                        comp_op,
                        prior_r_comp,
                        PI,
                        Some(ctx_tok.clone()),
                    ));
                }
            }
        }

        reports.push(PhaseCalibrationReport {
            ambiguous_word: word,
            competing_ops,
            prior_amplitudes,
            synthesized_payloads: payloads,
        });
    }

    reports
}

