//! # relic_core::quantum
//!
//! Primitivas cuántico-inspiradas en Rust puro para representación de amplitudes complejas,
//! interferencia de fase en desambiguación léxica polisémica, Regla de Born ($p = |\alpha|^2$)
//! y regla de decisión proyectiva bajo costo asimétrico ($p^* = \frac{\lambda - c}{\lambda + 1}$).

use std::ops::{Add, AddAssign, Mul, Sub};
use serde::{Deserialize, Serialize};

/// Número complejo en precisión doble (`f64`) alojado en stack (`Copy`, `Clone`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };

    #[inline] pub const fn new(re: f64, im: f64) -> Self { Self { re, im } }
    #[inline] pub fn from_polar(r: f64, theta: f64) -> Self { Self { re: r * theta.cos(), im: r * theta.sin() } }
    #[inline] pub fn norm_sqr(self) -> f64 { self.re * self.re + self.im * self.im }
    #[inline] pub fn norm(self) -> f64 { self.norm_sqr().sqrt() }
    #[inline] pub fn phase(self) -> f64 { self.im.atan2(self.re) }
    #[inline] pub const fn conj(self) -> Self { Self { re: self.re, im: -self.im } }
    #[inline] pub fn scale(self, factor: f64) -> Self { Self { re: self.re * factor, im: self.im * factor } }
}

impl Add for Complex {
    type Output = Self;
    #[inline] fn add(self, rhs: Self) -> Self::Output { Self { re: self.re + rhs.re, im: self.im + rhs.im } }
}
impl AddAssign for Complex {
    #[inline] fn add_assign(&mut self, rhs: Self) { self.re += rhs.re; self.im += rhs.im; }
}
impl Sub for Complex {
    type Output = Self;
    #[inline] fn sub(self, rhs: Self) -> Self::Output { Self { re: self.re - rhs.re, im: self.im - rhs.im } }
}
impl Mul for Complex {
    type Output = Self;
    #[inline] fn mul(self, rhs: Self) -> Self::Output {
        Self { re: self.re * rhs.re - self.im * rhs.im, im: self.re * rhs.im + self.im * rhs.re }
    }
}

/// Paso auditable de interferencia cuántica sobre un candidato $k$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterferenceStepTrace<T> {
    pub target: T,
    pub prior_amplitude: Complex,
    pub context_amplitude: Complex,
    pub combined_amplitude: Complex,
    pub phase_delta_rad: f64,
    pub cos_delta: f64,
    pub interference_kind: String,
    pub born_probability_after: f64,
}

/// Resultado del verificador proyectivo con regla de costo asimétrico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollapseDecision<T> {
    Collapsed {
        target: T,
        born_probability: f64,
        required_threshold: f64,
        von_neumann_shannon_entropy: f64,
        trace: Vec<InterferenceStepTrace<T>>,
    },
    Abstained {
        reason: String,
        max_prob: f64,
        required_threshold: f64,
        entropy: f64,
        trace: Vec<InterferenceStepTrace<T>>,
    },
}

/// Superposición coherente de candidatos discretos $\sum_k \alpha_k |k\rangle$.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantumSuperposition<T> {
    pub candidates: Vec<(T, Complex)>,
    pub trace: Vec<InterferenceStepTrace<T>>,
}

/// Calcula el umbral crítico de probabilidad bajo costo asimétrico: $p^* = \frac{\lambda - c}{\lambda + 1}$.
#[inline]
pub fn compute_asymmetric_threshold(lambda: f64, abstention_cost: f64) -> f64 {
    if lambda <= 0.0 { return 1.0; }
    ((lambda - abstention_cost.max(0.0)) / (lambda + 1.0)).clamp(0.0, 1.0)
}

impl<T: Clone + PartialEq> QuantumSuperposition<T> {
    pub fn new(candidates: Vec<(T, Complex)>) -> Self {
        let mut qs = Self { candidates, trace: Vec::new() };
        qs.normalize_unitary();
        qs
    }
    pub fn new_unnormalized(candidates: Vec<(T, Complex)>) -> Self {
        Self { candidates, trace: Vec::new() }
    }
    pub fn normalize_unitary(&mut self) {
        let total: f64 = self.candidates.iter().map(|(_, a)| a.norm_sqr()).sum();
        if total > 1e-15 {
            let inv = 1.0 / total.sqrt();
            for (_, amp) in &mut self.candidates { *amp = amp.scale(inv); }
        }
    }
    pub fn born_probabilities(&self) -> Vec<(T, f64)> {
        let total: f64 = self.candidates.iter().map(|(_, a)| a.norm_sqr()).sum();
        if total <= 1e-15 {
            return self.candidates.iter().map(|(t, _)| (t.clone(), 0.0)).collect();
        }
        self.candidates.iter().map(|(t, a)| (t.clone(), a.norm_sqr() / total)).collect()
    }
    pub fn projective_entropy(&self) -> f64 {
        self.born_probabilities().into_iter().filter(|(_, p)| *p > 1e-15).map(|(_, p)| -p * p.log2()).sum::<f64>().max(0.0)
    }
    pub fn apply_phase_interference(&mut self, context_shifts: &[(T, Complex)]) {
        let mut step_records = Vec::new();
        for (target, amp) in &mut self.candidates {
            if let Some((_, ctx_amp)) = context_shifts.iter().find(|(t, _)| t == target) {
                let prior = *amp;
                let combined = prior + *ctx_amp;
                let delta_theta = ctx_amp.phase() - prior.phase();
                let cos_d = delta_theta.cos();
                let kind = if cos_d > 1e-6 { "constructiva" } else if cos_d < -1e-6 { "destructiva" } else { "ortogonal" };
                *amp = combined;
                step_records.push((target.clone(), prior, *ctx_amp, combined, delta_theta, cos_d, kind.to_string()));
            }
        }
        self.normalize_unitary();
        let probs = self.born_probabilities();
        for (target, prior, ctx_amp, combined, delta_theta, cos_d, kind) in step_records {
            let p_after = probs.iter().find(|(t, _)| *t == target).map_or(0.0, |(_, p)| *p);
            self.trace.push(InterferenceStepTrace {
                target, prior_amplitude: prior, context_amplitude: ctx_amp, combined_amplitude: combined,
                phase_delta_rad: delta_theta, cos_delta: cos_d, interference_kind: kind, born_probability_after: p_after,
            });
        }
    }
    pub fn collapse_with_asymmetric_cost(&self, lambda: f64, abstention_cost: f64) -> CollapseDecision<T> {
        let required_threshold = compute_asymmetric_threshold(lambda, abstention_cost);
        let probs = self.born_probabilities();
        let entropy = self.projective_entropy();
        if probs.is_empty() {
            return CollapseDecision::Abstained { reason: "Superposición vacía.".to_string(), max_prob: 0.0, required_threshold, entropy: 0.0, trace: self.trace.clone() };
        }
        let mut sorted = probs;
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let (best_target, max_prob) = sorted[0].clone();
        if max_prob <= 1e-12 {
            return CollapseDecision::Abstained { reason: "Cancelación destructiva total.".to_string(), max_prob: 0.0, required_threshold, entropy, trace: self.trace.clone() };
        }
        if sorted.len() > 1 && (max_prob - sorted[1].1).abs() < 1e-6 {
            return CollapseDecision::Abstained { reason: format!("Colisión de simetría/fase: empate (p = {max_prob:.4})."), max_prob, required_threshold, entropy, trace: self.trace.clone() };
        }
        if max_prob <= required_threshold {
            return CollapseDecision::Abstained { reason: format!("Incertidumbre bajo costo asimétrico: p_max ({max_prob:.4}) <= p* ({required_threshold:.4})."), max_prob, required_threshold, entropy, trace: self.trace.clone() };
        }
        CollapseDecision::Collapsed { target: best_target, born_probability: max_prob, required_threshold, von_neumann_shannon_entropy: entropy, trace: self.trace.clone() }
    }
}
