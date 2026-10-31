//! # EchoStatePredictor
//!
//! Predictor neuronal recurrente ESN con adaptación causal RLS y anti-windup estricto.

use super::embedding::DelayEmbedding;
use super::norm::RollingNorm;
use super::types::{EMBED_DIM, NORM_WINDOW, RESERVOIR_SIZE, RLS_DELTA, RLS_FORGET};

/// Predictor recurrente ESN con pesos estáticos compartidos.
#[derive(Clone)]
pub struct EchoStatePredictor {
    pub h: [f64; RESERVOIR_SIZE],
    pub wout: [f64; RESERVOIR_SIZE],
    pub p_mat: Box<[[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE]>,
    pub norm: RollingNorm,
    pub embed: DelayEmbedding,
}

impl EchoStatePredictor {
    pub fn new() -> Self {
        let mut p_mat = Box::new([[0.0; RESERVOIR_SIZE]; RESERVOIR_SIZE]);
        for i in 0..RESERVOIR_SIZE {
            p_mat[i][i] = RLS_DELTA;
        }

        Self {
            h: [0.0; RESERVOIR_SIZE],
            wout: [0.0; RESERVOIR_SIZE],
            p_mat,
            norm: RollingNorm::new(NORM_WINDOW),
            embed: DelayEmbedding::new(EMBED_DIM),
        }
    }

    pub fn reset(&mut self) {
        self.h = [0.0; RESERVOIR_SIZE];
        self.wout = [0.0; RESERVOIR_SIZE];
        for i in 0..RESERVOIR_SIZE {
            for j in 0..RESERVOIR_SIZE {
                self.p_mat[i][j] = if i == j { RLS_DELTA } else { 0.0 };
            }
        }
        self.norm.reset();
        self.embed.reset();
    }

    /// Inferencia pura inmutable.
    #[allow(clippy::needless_range_loop)]
    pub fn evaluate_pure(
        &self,
        value: f64,
        win: &[[f64; EMBED_DIM]; RESERVOIR_SIZE],
        wres: &[[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE],
    ) -> (f64, Option<[f64; EMBED_DIM]>, [f64; RESERVOIR_SIZE], f64, f64) {
        if !value.is_finite() {
            return (0.0, None, self.h, 0.0, 0.0);
        }

        let z = self.norm.standardize(value);
        let takens_opt = self.embed.construct_vector(z);

        if let Some(vec) = takens_opt {
            let mut h_new = [0.0; RESERVOIR_SIZE];
            for i in 0..RESERVOIR_SIZE {
                let mut acc = 0.0;
                for j in 0..EMBED_DIM {
                    acc += win[i][j] * vec[j];
                }
                for j in 0..RESERVOIR_SIZE {
                    acc += wres[i][j] * self.h[j];
                }
                h_new[i] = acc.tanh();
            }

            let mut pred_z = 0.0;
            for i in 0..RESERVOIR_SIZE {
                pred_z += self.wout[i] * h_new[i];
            }

            let resid = z - pred_z;
            let err = if resid.is_nan() { 0.0 } else { (resid * resid).min(1e12) };

            (z, Some(vec), h_new, pred_z, err)
        } else {
            (z, None, self.h, 0.0, 0.0)
        }
    }

    /// Actualización causal RLS idéntica al modelo original.
    #[allow(clippy::needless_range_loop)]
    pub fn commit(
        &mut self,
        value: f64,
        z: f64,
        ready: bool,
        h_candidate: [f64; RESERVOIR_SIZE],
        pred_z: f64,
    ) {
        if !value.is_finite() {
            return;
        }

        if !ready {
            self.embed.ingest(z);
            self.norm.ingest(value);
            return;
        }

        let h = h_candidate;
        let mut ph = [0.0; RESERVOIR_SIZE];
        for i in 0..RESERVOIR_SIZE {
            let mut acc = 0.0;
            for j in 0..RESERVOIR_SIZE {
                acc += self.p_mat[i][j] * h[j];
            }
            ph[i] = acc;
        }

        let mut h_ph = 0.0;
        for i in 0..RESERVOIR_SIZE {
            h_ph += h[i] * ph[i];
        }

        let denom = (RLS_FORGET + h_ph).max(1e-12);
        let mut k_gain = [0.0; RESERVOIR_SIZE];
        for i in 0..RESERVOIR_SIZE {
            k_gain[i] = ph[i] / denom;
        }

        let resid = z - pred_z;
        for i in 0..RESERVOIR_SIZE {
            self.wout[i] += k_gain[i] * resid;
        }

        for i in 0..RESERVOIR_SIZE {
            for j in 0..RESERVOIR_SIZE {
                self.p_mat[i][j] = (self.p_mat[i][j] - k_gain[i] * ph[j]) / RLS_FORGET;
            }
        }

        // Anti-Windup estricto: re-ancla P y wout ante divergencias numéricas
        let mut p_finite = true;
        for i in 0..RESERVOIR_SIZE {
            for j in 0..RESERVOIR_SIZE {
                if !self.p_mat[i][j].is_finite() {
                    p_finite = false;
                    break;
                }
            }
        }

        if !pred_z.is_finite() || !p_finite {
            self.wout = [0.0; RESERVOIR_SIZE];
            for i in 0..RESERVOIR_SIZE {
                for j in 0..RESERVOIR_SIZE {
                    self.p_mat[i][j] = if i == j { RLS_DELTA } else { 0.0 };
                }
            }
        }

        self.h = h_candidate;
        self.embed.ingest(z);
        self.norm.ingest(value);
    }
}

impl Default for EchoStatePredictor {
    fn default() -> Self {
        Self::new()
    }
}

