//! Feed-Forward Network con SwiGLU en Rust puro.
use crate::config::RelicConfig;
use crate::ops::{linear_forward, swiglu_forward};

#[derive(Debug, Clone)]
pub struct SwiGluMlp {
    pub gate_proj: Vec<f32>, // [intermediate_size, hidden_size]
    pub up_proj: Vec<f32>,   // [intermediate_size, hidden_size]
    pub down_proj: Vec<f32>, // [hidden_size, intermediate_size]
    pub hidden_size: usize,
    pub intermediate_size: usize,
}

impl SwiGluMlp {
    pub fn new(config: &RelicConfig) -> Self {
        let gate_size = config.intermediate_size * config.hidden_size;
        let up_size = config.intermediate_size * config.hidden_size;
        let down_size = config.hidden_size * config.intermediate_size;

        Self {
            gate_proj: vec![0.01; gate_size],
            up_proj: vec![0.01; up_size],
            down_proj: vec![0.01; down_size],
            hidden_size: config.hidden_size,
            intermediate_size: config.intermediate_size,
        }
    }

    /// Ejecuta el forward pass de SwiGLU sobre un vector de activacion x.
    pub fn forward(&self, out: &mut [f32], x: &[f32]) {
        debug_assert_eq!(x.len(), self.hidden_size);
        debug_assert_eq!(out.len(), self.hidden_size);

        let mut gate = vec![0.0f32; self.intermediate_size];
        let mut up = vec![0.0f32; self.intermediate_size];
        let mut activated = vec![0.0f32; self.intermediate_size];

        linear_forward(&mut gate, x, &self.gate_proj, self.hidden_size, self.intermediate_size);
        linear_forward(&mut up, x, &self.up_proj, self.hidden_size, self.intermediate_size);
        swiglu_forward(&mut activated, &gate, &up);
        linear_forward(out, &activated, &self.down_proj, self.intermediate_size, self.hidden_size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mlp_forward_dimensions() {
        let cfg = RelicConfig {
            hidden_size: 4,
            intermediate_size: 8,
            num_attention_heads: 2,
            num_key_value_heads: 1,
            head_dim: 2,
            ..Default::default()
        };
        let mlp = SwiGluMlp::new(&cfg);
        let x = vec![1.0, -1.0, 0.5, -0.5];
        let mut out = vec![0.0; 4];
        mlp.forward(&mut out, &x);
        assert_eq!(out.len(), 4);
    }
}
