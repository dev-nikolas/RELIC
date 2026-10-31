//! # RollingNorm
//!
//! Buffer circular rodante W=6400 para estandarización y normalización de telemetría.

use std::collections::VecDeque;

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

    pub fn mean_and_std(&self) -> (f64, f64) {
        let n = self.buffer.len();
        if n < 2 {
            return (0.0, 1.0);
        }
        let mean = self.sum / (n as f64);
        let var = ((self.sum_sq / (n as f64)) - mean * mean).max(0.0);
        let std = var.sqrt();
        (mean, if std > 1e-9 { std } else { 1.0 })
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

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

