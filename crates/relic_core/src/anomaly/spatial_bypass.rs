//! Bypass Espacial de Spikes Históricos (SpatialSpikeBypass).
//!
//! Port fiel de `spatial_spike_bypass.py` (warmup=100, tolerance=0.05).
//! Mantiene cotas dinámicas causales [min, max] y detecta inmediatamente
//! roturas del rango histórico por más de 5% de la amplitud.

pub const DEFAULT_SPATIAL_TOLERANCE: f64 = 0.05;
pub const DEFAULT_WARMUP: usize = 100;

#[derive(Clone, Debug)]
pub struct SpatialSpikeBypass {
    pub tolerance: f64,
    pub warmup: usize,
    min_val: Option<f64>,
    max_val: Option<f64>,
    count: usize,
}

impl SpatialSpikeBypass {
    pub fn new(tolerance: f64, warmup: usize) -> Self {
        Self {
            tolerance,
            warmup,
            min_val: None,
            max_val: None,
            count: 0,
        }
    }

    pub fn default_config() -> Self {
        Self::new(DEFAULT_SPATIAL_TOLERANCE, DEFAULT_WARMUP)
    }

    pub fn reset(&mut self) {
        self.min_val = None;
        self.max_val = None;
        self.count = 0;
    }

    /// Chequeo causal inmutable de anomalía espacial.
    pub fn check_anomaly(&self, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        if self.count >= self.warmup {
            if let (Some(min_v), Some(max_v)) = (self.min_val, self.max_val) {
                let span = max_v - min_v;
                if span > 1e-12 {
                    let margin = span * self.tolerance;
                    if value > max_v + margin || value < min_v - margin {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Actualización causal de las cotas acumuladas.
    pub fn commit(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }
        self.min_val = Some(self.min_val.map_or(value, |m| m.min(value)));
        self.max_val = Some(self.max_val.map_or(value, |m| m.max(value)));
        self.count += 1;
    }

    #[inline]
    pub fn bounds(&self) -> (Option<f64>, Option<f64>) {
        (self.min_val, self.max_val)
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }
}

