//! Monitor Lateral de Deriva Causal (TelemetryDriftMonitor).
//!
//! Implementa CUSUM bilateral estandarizado con referencia fija (mu_0, sigma_0)
//! calculada durante el periodo de calibración inicial. Opera exclusivamente como
//! compuerta lateral de seguridad sin alterar el score NAB.
//!
//! Calibración de Page (1954):
//! Con k = 0.5 y h = 10.0:
//! - ARL_0 (Average Run Length bajo H_0: N(0, 1)): ~7.2e4 pasos (~240 días a 5 min/paso).
//! - ARL_1 (Detección bajo corrimiento de +1.0 sigma): ~20-25 pasos.

pub const DEFAULT_CUSUM_K: f64 = 0.5;
pub const DEFAULT_CUSUM_H: f64 = 10.0;

#[derive(Clone, Debug)]
pub struct TelemetryDriftMonitor {
    pub mu_0: f64,
    pub sigma_0: f64,
    pub s_pos: f64,
    pub s_neg: f64,
    pub k: f64,
    pub h: f64,
    pub calibrated: bool,
    pub alarm_fired: bool,
}

impl TelemetryDriftMonitor {
    pub fn new() -> Self {
        Self::with_parameters(DEFAULT_CUSUM_K, DEFAULT_CUSUM_H)
    }

    pub fn with_parameters(k: f64, h: f64) -> Self {
        Self {
            mu_0: 0.0,
            sigma_0: 1.0,
            s_pos: 0.0,
            s_neg: 0.0,
            k,
            h,
            calibrated: false,
            alarm_fired: false,
        }
    }

    pub fn calibrate(&mut self, mean: f64, std: f64) {
        self.mu_0 = mean;
        self.sigma_0 = if std > 1e-9 { std } else { 1.0 };
        self.s_pos = 0.0;
        self.s_neg = 0.0;
        self.calibrated = true;
        self.alarm_fired = false;
    }

    pub fn reset(&mut self) {
        self.s_pos = 0.0;
        self.s_neg = 0.0;
        self.calibrated = false;
        self.alarm_fired = false;
    }

    /// Evaluación pura causal sin mutar el acumulador.
    pub fn evaluate_pure(&self, val: f64) -> (bool, f64, f64) {
        if !self.calibrated || !val.is_finite() {
            return (false, self.s_pos, self.s_neg);
        }
        let z = (val - self.mu_0) / self.sigma_0;
        let s_pos_cand = (self.s_pos + z - self.k).max(0.0);
        let s_neg_cand = (self.s_neg - z - self.k).max(0.0);
        let alarm = s_pos_cand > self.h || s_neg_cand > self.h;
        (alarm, s_pos_cand, s_neg_cand)
    }

    /// Actualiza los acumuladores CUSUM y retorna `true` si se dispara una alarma de deriva.
    /// Al dispararse la alarma, reinicia los acumuladores (esquema de Page 1954).
    pub fn update(&mut self, val: f64) -> bool {
        if !self.calibrated || !val.is_finite() {
            self.alarm_fired = false;
            return false;
        }
        let z = (val - self.mu_0) / self.sigma_0;
        self.s_pos = (self.s_pos + z - self.k).max(0.0);
        self.s_neg = (self.s_neg - z - self.k).max(0.0);
        if self.s_pos > self.h || self.s_neg > self.h {
            self.s_pos = 0.0;
            self.s_neg = 0.0;
            self.alarm_fired = true;
            true
        } else {
            self.alarm_fired = false;
            false
        }
    }

    #[inline]
    pub fn is_alarm(&self) -> bool {
        self.alarm_fired
    }
}

impl Default for TelemetryDriftMonitor {
    fn default() -> Self {
        Self::new()
    }
}

