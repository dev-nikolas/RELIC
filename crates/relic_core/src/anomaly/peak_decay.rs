//! Supresión de No-Máximos (NMS) causal para streaming: StreamingPeakDecay.
//!
//! Port fiel línea por línea de `peak_decay.py` (K=24, τ=2.0).
//! Atenúa mesetas posteriores mediante exp(-(t - t_peak) / τ) sin perjudicar
//! la detección de la cresta en el frente de ataque.

use std::collections::VecDeque;

pub const DEFAULT_K_WINDOW: usize = 24;
pub const DEFAULT_TAU: f64 = 2.0;

#[derive(Clone, Debug)]
pub struct StreamingPeakDecay {
    pub k_window: usize,
    pub tau: f64,
    history: VecDeque<f64>,
    t: usize,
    t_peak: usize,
}

impl StreamingPeakDecay {
    pub fn new(k_window: usize, tau: f64) -> Self {
        Self {
            k_window,
            tau,
            history: VecDeque::with_capacity(k_window),
            t: 0,
            t_peak: 0,
        }
    }

    pub fn default_config() -> Self {
        Self::new(DEFAULT_K_WINDOW, DEFAULT_TAU)
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.t = 0;
        self.t_peak = 0;
    }

    /// Evaluación pura e inmutable del decaimiento exponencial.
    /// Retorna `(is_peak, score_atenuado)`.
    pub fn evaluate_pure(&self, raw_score: f64) -> (bool, f64) {
        if !raw_score.is_finite() {
            return (false, 0.0);
        }
        let score = raw_score.clamp(0.0, 1.0);

        let is_peak = self.history.is_empty()
            || score > self.history.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let score_out = if is_peak {
            score
        } else {
            let decay = (-(self.t.saturating_sub(self.t_peak) as f64) / self.tau).exp();
            score * decay
        };

        (is_peak, score_out.clamp(0.0, 1.0))
    }

    /// Compromiso atómico de estado.
    /// Ingesta estrictamente `raw_score` (la magnitud un-decayed del calibrador ECDF).
    pub fn commit(&mut self, raw_score: f64, is_peak: bool) {
        if !raw_score.is_finite() {
            return;
        }
        let score = raw_score.clamp(0.0, 1.0);

        if is_peak {
            self.t_peak = self.t;
        }

        if self.history.len() >= self.k_window {
            self.history.pop_front();
        }
        self.history.push_back(score);
        self.t += 1;
    }

    #[inline]
    pub fn current_t(&self) -> usize {
        self.t
    }

    #[inline]
    pub fn t_peak(&self) -> usize {
        self.t_peak
    }
}

