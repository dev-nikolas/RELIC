//! # ResidualSprtMartingale
//!
//! Martingala Secuencial SPRT de Wald con estimador de escala de Welford acotado
//! en `[sigma_min_noise, sigma_max_safe]` contra *Variance Spoofing* y constante `h = 13.8155` inmutable.

/// Umbral de parada analítico de Wald para alpha = 10^-6: ln(1 / 10^-6) = 13.81551
pub const WALD_THRESHOLD_ALPHA_1E6: f32 = 13.81551;

/// Martingala Secuencial SPRT sobre Residuos Topológicos.
#[derive(Clone, Copy, Debug)]
pub struct ResidualSprtMartingale {
    pub s_pos: f32,
    pub s_neg: f32,
    pub k: f32,
    pub threshold_h: f32,
    pub log_martingale: f32,
    pub alarm_fired: bool,
    pub steps: u64,
    // Estimador de escala en línea (Welford acotado)
    pub mean_residual: f32,
    pub m2_residual: f32,
    pub current_sigma: f32,
    pub sigma_min_noise: f32,
    pub sigma_max_safe: f32,
}

impl ResidualSprtMartingale {
    /// Inicializa la martingala con parámetros base y umbral de Wald explícito.
    pub fn new(sigma_base: f32, k: f32, threshold_h: f32) -> Self {
        let s_base = if sigma_base > 1e-6 { sigma_base } else { 1.0 };
        let mut m = Self::with_bounded_welford(s_base, k, s_base * 0.1, s_base * 2.5);
        if threshold_h > 0.0 {
            m.threshold_h = threshold_h;
        }
        m
    }

    /// Inicializa la martingala con cota inmutable de Wald (`alpha = 1e-6`) y límites de Welford explícitos.
    pub fn with_bounded_welford(sigma_base: f32, k: f32, sigma_min: f32, sigma_max: f32) -> Self {
        let s_base = if sigma_base > 1e-6 { sigma_base } else { 1.0 };
        let s_min = sigma_min.max(1e-4);
        let s_max = sigma_max.max(s_min * 1.5);
        Self {
            s_pos: 0.0,
            s_neg: 0.0,
            k: if k > 0.0 { k } else { 0.5 },
            threshold_h: WALD_THRESHOLD_ALPHA_1E6,
            log_martingale: 0.0,
            alarm_fired: false,
            steps: 0,
            mean_residual: 0.0,
            m2_residual: s_base * s_base,
            current_sigma: s_base.clamp(s_min, s_max),
            sigma_min_noise: s_min,
            sigma_max_safe: s_max,
        }
    }

    /// Reinicia los acumuladores de la martingala conservando las cotas físicas de escala.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.s_pos = 0.0;
        self.s_neg = 0.0;
        self.log_martingale = 0.0;
        self.alarm_fired = false;
        self.steps = 0;
        self.mean_residual = 0.0;
        self.m2_residual = self.current_sigma * self.current_sigma;
    }

    /// Evaluación pura e inmutable sin alterar el estado interno.
    #[inline(always)]
    pub fn evaluate_pure(&self, raw_residual: f32) -> (bool, f32) {
        if !raw_residual.is_finite() {
            return (true, f32::INFINITY);
        }

        let z = raw_residual / self.current_sigma;
        let next_s_pos = (self.s_pos + z - self.k).max(0.0);
        let next_s_neg = (self.s_neg - z - self.k).max(0.0);
        let current_log = next_s_pos.max(next_s_neg);

        (current_log >= self.threshold_h, current_log)
    }

    /// Actualiza la martingala con un nuevo residuo observado y adapta la escala de Welford en quiescencia.
    #[inline(always)]
    pub fn update(&mut self, raw_residual: f32) -> (bool, f32) {
        if !raw_residual.is_finite() {
            self.alarm_fired = true;
            self.log_martingale = f32::INFINITY;
            return (true, self.log_martingale);
        }

        self.steps = self.steps.wrapping_add(1);

        // 1. Actualización de escala Welford solo en régimen de calma estricta
        if self.log_martingale == 0.0 && self.steps > 1 {
            let delta = raw_residual - self.mean_residual;
            self.mean_residual += delta / (self.steps as f32);
            let delta2 = raw_residual - self.mean_residual;
            self.m2_residual += delta * delta2;
            let sample_variance = self.m2_residual / ((self.steps - 1) as f32);
            let raw_sigma = sample_variance.sqrt();
            // Cota estricta contra Variance Spoofing
            self.current_sigma = raw_sigma.clamp(self.sigma_min_noise, self.sigma_max_safe);
        }

        // 2. Normalización pre-martingala
        let z = raw_residual / self.current_sigma;

        // 3. Acumulador secuencial de Wald
        self.s_pos = (self.s_pos + z - self.k).max(0.0);
        self.s_neg = (self.s_neg - z - self.k).max(0.0);
        self.log_martingale = self.s_pos.max(self.s_neg);
        self.alarm_fired = self.log_martingale >= self.threshold_h;

        (self.alarm_fired, self.log_martingale)
    }
}

impl Default for ResidualSprtMartingale {
    #[inline]
    fn default() -> Self {
        Self::with_bounded_welford(1.0, 0.5, 0.1, 2.5)
    }
}
