//! # DelayEmbedding
//!
//! Buffer causal de incrustación de coordenadas de retardo (Teorema de Takens).

use std::collections::VecDeque;
use super::types::EMBED_DIM;

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

    #[inline(always)]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= self.dim
    }

    /// Construye el vector Takens proyectado [z_t, z_{t-1}, ..., z_{t-15}].
    #[allow(clippy::needless_range_loop)]
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

