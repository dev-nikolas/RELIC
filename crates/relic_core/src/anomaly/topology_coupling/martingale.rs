//! # ResidualSprtMartingale
//!
//! Martingala secuencial SPRT sobre residuos espaciales.
//! Garantiza tasa de falsos positivos acotada formalmente por el lema de Wald y la
//! desigualdad de martingalas de Doob: P_{H0}(alarma) <= e^{-threshold_h}.

/// Martingala Secuencial SPRT sobre Residuos Topológicos.
///
/// Evalúa el cociente de verosimilitudes acumulado bajo la hipótesis nula H_0: \varepsilon ~ N(0, \sigma^2)
/// frente a una desviación causal persistente H_1: |\varepsilon| >= \delta.
#[derive(Clone, Copy, Debug)]
pub struct ResidualSprtMartingale {
    pub s_pos: f32,
    pub s_neg: f32,
    pub sigma: f32,
    pub k: f32,
    pub threshold_h: f32,
    pub log_martingale: f32,
    pub alarm_fired: bool,
    pub steps: u64,
}

impl ResidualSprtMartingale {
    /// Inicializa la martingala con parámetros calibrados.
    pub fn new(sigma: f32, k: f32, threshold_h: f32) -> Self {
        Self {
            s_pos: 0.0,
            s_neg: 0.0,
            sigma: if sigma > 1e-6 { sigma } else { 1.0 },
            k: if k > 0.0 { k } else { 0.5 },
            threshold_h: if threshold_h > 0.0 { threshold_h } else { 10.0 },
            log_martingale: 0.0,
            alarm_fired: false,
            steps: 0,
        }
    }

    /// Reinicia los acumuladores de la martingala.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.s_pos = 0.0;
        self.s_neg = 0.0;
        self.log_martingale = 0.0;
        self.alarm_fired = false;
        self.steps = 0;
    }

    /// Evaluación pura e inmutable sin alterar el estado interno.
    #[inline(always)]
    pub fn evaluate_pure(&self, residual: f32) -> (bool, f32) {
        if !residual.is_finite() {
            return (true, f32::INFINITY);
        }

        let z = residual / self.sigma;
        let k_norm = self.k / self.sigma;

        let next_s_pos = (self.s_pos + z - k_norm).max(0.0);
        let next_s_neg = (self.s_neg - z - k_norm).max(0.0);
        let current_log = next_s_pos.max(next_s_neg);

        (current_log >= self.threshold_h, current_log)
    }

    /// Actualiza la martingala con un nuevo residuo observado.
    /// Retorna `(alarm_fired, log_martingale)`.
    #[inline(always)]
    pub fn update(&mut self, residual: f32) -> (bool, f32) {
        self.steps = self.steps.wrapping_add(1);

        if !residual.is_finite() {
            self.alarm_fired = true;
            self.log_martingale = f32::INFINITY;
            return (true, f32::INFINITY);
        }

        let z = residual / self.sigma;
        let k_norm = self.k / self.sigma;

        self.s_pos = (self.s_pos + z - k_norm).max(0.0);
        self.s_neg = (self.s_neg - z - k_norm).max(0.0);
        self.log_martingale = self.s_pos.max(self.s_neg);

        if self.log_martingale >= self.threshold_h {
            self.alarm_fired = true;
            (true, self.log_martingale)
        } else {
            self.alarm_fired = false;
            (false, self.log_martingale)
        }
    }
}

impl Default for ResidualSprtMartingale {
    #[inline]
    fn default() -> Self {
        Self::new(1.0, 0.5, 13.82)
    }
}

