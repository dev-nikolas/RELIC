//! Silenciador Refractario Causal (RefractorySuppressor).
//!
//! Port fiel de `refractory_suppressor.py` (w_rest=80, trigger=0.990).
//! Suprime ráfagas redundantes de alertas tras un pico extremo (score >= trigger_threshold),
//! estabilizando la transferencia en validación cruzada LOCO de NAB.

pub const DEFAULT_W_REST: usize = 80;
pub const DEFAULT_TRIGGER_THRESHOLD: f64 = 0.990;

#[derive(Clone, Debug)]
pub struct RefractorySuppressor {
    pub w_rest: usize,
    pub trigger_threshold: f64,
    t: usize,
    silence_until: usize,
}

impl RefractorySuppressor {
    pub fn new(w_rest: usize, trigger_threshold: f64) -> Self {
        Self {
            w_rest,
            trigger_threshold,
            t: 0,
            silence_until: 0,
        }
    }

    pub fn default_config() -> Self {
        Self::new(DEFAULT_W_REST, DEFAULT_TRIGGER_THRESHOLD)
    }

    pub fn reset(&mut self) {
        self.t = 0;
        self.silence_until = 0;
    }

    /// Evaluación pura de la compuerta refractaria para NAB.
    /// Retorna `(is_suppressed, score_salida)`.
    pub fn evaluate_pure(&self, raw_score: f64) -> (bool, f64) {
        if !raw_score.is_finite() {
            return (false, 0.0);
        }
        if self.t < self.silence_until {
            return (true, 0.0);
        }
        (false, raw_score)
    }

    /// Actualización causal del temporizador de alerta idéntica a Python.
    pub fn commit(&mut self, raw_score: f64) {
        let t = self.t;
        self.t += 1;

        if t < self.silence_until {
            return;
        }

        if raw_score >= self.trigger_threshold && self.w_rest > 0 {
            self.silence_until = t + 1 + self.w_rest;
        }
    }

    #[inline]
    pub fn is_refractory_active(&self) -> bool {
        self.t < self.silence_until
    }
}
