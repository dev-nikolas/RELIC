//! Normalizacion Root Mean Square (RMSNorm) en Rust puro.
//! Menos de 180 lineas.

#[derive(Debug, Clone)]
pub struct RMSNorm {
    pub weight: Vec<f32>,
    pub eps: f32,
    pub dim: usize,
}

impl RMSNorm {
    pub fn new(dim: usize, eps: f32) -> Self {
        Self {
            weight: vec![1.0; dim],
            eps,
            dim,
        }
    }

    pub fn from_weight(weight: Vec<f32>, eps: f32) -> Self {
        let dim = weight.len();
        Self { weight, eps, dim }
    }

    /// Aplica RMSNorm sobre el vector x y escribe en out.
    pub fn forward(&self, out: &mut [f32], x: &[f32]) {
        debug_assert_eq!(x.len(), self.dim);
        debug_assert_eq!(out.len(), self.dim);

        let mut sum_sq = 0.0f32;
        for &val in x {
            sum_sq += val * val;
        }
        let variance = sum_sq / (self.dim as f32);
        let scale = 1.0 / (variance + self.eps).sqrt();

        for ((o, &xi), &wi) in out.iter_mut().zip(x.iter()).zip(self.weight.iter()) {
            *o = (xi * scale) * wi;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rmsnorm_forward() {
        let norm = RMSNorm::new(4, 1e-6);
        let x = vec![2.0, 2.0, 2.0, 2.0];
        let mut out = vec![0.0; 4];
        norm.forward(&mut out, &x);
        // La norma RMS de [2, 2, 2, 2] es 2.0, por lo que out debe ser ~1.0
        for val in out {
            assert!((val - 1.0).abs() < 1e-3);
        }
    }
}
