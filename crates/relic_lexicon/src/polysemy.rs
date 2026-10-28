//! # relic_lexicon::polysemy
//!
//! Resolución de polisemia léxica mediante el Baseline Clásico (pesos positivos)
//! y mediante Interferencia Cuántica de Fase + Regla de Costo Asimétrico. Menos de 180 líneas.

use std::collections::HashMap;
use relic_core::{ArithmeticOp, CollapseDecision, Complex, QuantumSuperposition};
use serde::{Deserialize, Serialize};
use crate::memory::LexiconView;

/// Amplitud compleja asociada a una interpretación de un término polisémico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolysemousEntry {
    pub target_op: ArithmeticOp,
    pub amplitude: Complex,
    pub context_word: Option<String>,
}

/// Resultado de desambiguación en el Baseline Clásico (pesos reales positivos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClassicalPolysemyResult {
    Resolved {
        target_op: ArithmeticOp,
        probability: f64,
    },
    Ambiguous {
        candidates: Vec<(ArithmeticOp, f64)>,
        reason: String,
    },
    NotFound,
}

impl LexiconView {
    /// Resuelve un término polisémico usando el Baseline Clásico (suma de pesos positivos sin fase).
    /// Sirve como control experimental contra `resolve_polysemous_quantum`.
    pub fn resolve_polysemous_classical(
        &self,
        word: &str,
        context_words: &[&str],
    ) -> ClassicalPolysemyResult {
        let key = word.trim().to_lowercase();
        let Some(entries) = self.quantum_polysemy.get(&key) else {
            return ClassicalPolysemyResult::NotFound;
        };
        let ctx_lower: Vec<String> = context_words.iter().map(|w| w.trim().to_lowercase()).collect();
        let mut totals: HashMap<ArithmeticOp, f64> = HashMap::new();
        for e in entries {
            let applies = match &e.context_word {
                None => true,
                Some(cw) => ctx_lower.iter().any(|c| c == cw),
            };
            if applies {
                *totals.entry(e.target_op).or_insert(0.0) += e.amplitude.norm();
            }
        }
        let sum_w: f64 = totals.values().sum();
        if sum_w <= 1e-12 {
            return ClassicalPolysemyResult::NotFound;
        }
        let mut probs: Vec<(ArithmeticOp, f64)> =
            totals.into_iter().map(|(op, w)| (op, w / sum_w)).collect();
        probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if probs.len() > 1 && (probs[0].1 - probs[1].1).abs() < 1e-6 {
            return ClassicalPolysemyResult::Ambiguous {
                candidates: probs,
                reason: format!("Empate clásico en pesos positivos para '{key}'"),
            };
        }
        ClassicalPolysemyResult::Resolved {
            target_op: probs[0].0,
            probability: probs[0].1,
        }
    }

    /// Resuelve un término polisémico mediante superposición cuántica e interferencia de fase contextual
    /// ($\psi_k = \alpha_{k,\text{prior}} + \alpha_{k,\text{ctx}}$) y regla de costo asimétrico $p^* = \frac{\lambda - c}{\lambda + 1}$.
    pub fn resolve_polysemous_quantum(
        &self,
        word: &str,
        context_words: &[&str],
        lambda: f64,
        abstention_cost: f64,
    ) -> Option<CollapseDecision<ArithmeticOp>> {
        let key = word.trim().to_lowercase();
        let entries = self.quantum_polysemy.get(&key)?;
        let ctx_lower: Vec<String> = context_words.iter().map(|w| w.trim().to_lowercase()).collect();

        let mut prior_map: Vec<(ArithmeticOp, Complex)> = Vec::new();
        let mut ctx_shifts: Vec<(ArithmeticOp, Complex)> = Vec::new();

        for e in entries {
            match &e.context_word {
                None => {
                    if let Some((_, amp)) = prior_map.iter_mut().find(|(op, _)| *op == e.target_op) {
                        *amp += e.amplitude;
                    } else {
                        prior_map.push((e.target_op, e.amplitude));
                    }
                }
                Some(cw) => {
                    if ctx_lower.iter().any(|c| c == cw) {
                        if let Some((_, amp)) = ctx_shifts.iter_mut().find(|(op, _)| *op == e.target_op) {
                            *amp += e.amplitude;
                        } else {
                            ctx_shifts.push((e.target_op, e.amplitude));
                        }
                    }
                }
            }
        }

        if prior_map.is_empty() {
            return None;
        }

        let mut superposition = QuantumSuperposition::new_unnormalized(prior_map);
        if !ctx_shifts.is_empty() {
            superposition.apply_phase_interference(&ctx_shifts);
        } else {
            superposition.normalize_unitary();
        }
        Some(superposition.collapse_with_asymmetric_cost(lambda, abstention_cost))
    }
}

