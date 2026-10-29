//! Predictor Recurrente ESN con Adaptación Lineal RLS Completa.
//!
//! Port fiel de `echo_state.py` (N=64, ρ=0.9, Takens d=16, RLS λ=0.999).
//! Predice el estado latente mediante proyección en el espacio atractor y mide
//! el error cuadrático respecto a la dinámica aprendida.

use std::collections::VecDeque;

pub const EMBED_DIM: usize = 16;
pub const RESERVOIR_SIZE: usize = 64;
pub const NORM_WINDOW: usize = 6400;
pub const RLS_FORGET: f64 = 0.999;
pub const RLS_DELTA: f64 = 1.0;

/// Buffer circular para normalización rodante W=6400.
#[derive(Clone, Debug)]
pub struct RollingNorm {
    buffer: VecDeque<f64>,
    sum: f64,
    sum_sq: f64,
    max_len: usize,
}

impl RollingNorm {
    pub fn new(max_len: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_len),
            sum: 0.0,
            sum_sq: 0.0,
            max_len,
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.sum = 0.0;
        self.sum_sq = 0.0;
    }

    pub fn standardize(&self, val: f64) -> f64 {
        let n = self.buffer.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.sum / (n as f64);
        let var = ((self.sum_sq / (n as f64)) - mean * mean).max(0.0);
        let std = var.sqrt();
        if std <= 1e-9 {
            0.0
        } else {
            (val - mean) / std
        }
    }

    pub fn ingest(&mut self, val: f64) {
        if !val.is_finite() {
            return;
        }
        if self.buffer.len() == self.max_len {
            if let Some(old) = self.buffer.pop_front() {
                self.sum -= old;
                self.sum_sq -= old * old;
            }
        }
        self.buffer.push_back(val);
        self.sum += val;
        self.sum_sq += val * val;
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}

/// Buffer causal de incrustación de coordenadas de retardo (Takens).
#[derive(Clone, Debug)]
pub struct DelayEmbedding {
    buffer: VecDeque<f64>,
    dim: usize,
}

impl DelayEmbedding {
    pub fn new(dim: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(dim),
            dim,
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= self.dim
    }

    /// Construye el vector Takens proyectado [z_t, z_{t-1}, ..., z_{t-15}].
    pub fn construct_vector(&self, current_z: f64) -> Option<[f64; EMBED_DIM]> {
        if self.buffer.len() < self.dim - 1 {
            return None;
        }
        let mut vec = [0.0; EMBED_DIM];
        vec[0] = current_z;
        for i in 1..self.dim {
            vec[i] = self.buffer[self.buffer.len() - i];
        }
        Some(vec)
    }

    pub fn ingest(&mut self, z: f64) {
        if self.buffer.len() == self.dim {
            self.buffer.pop_front();
        }
        self.buffer.push_back(z);
    }
}

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
    /// Retorna `(z, Option<TakensVector>, h_new, pred_z, err)`.
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
            let err = if resid.is_nan() {
                0.0
            } else {
                (resid * resid).min(1e12)
            };

            (z, Some(vec), h_new, pred_z, err)
        } else {
            (z, None, self.h, 0.0, 0.0)
        }
    }

    /// Actualización causal RLS idéntica a Python.
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

