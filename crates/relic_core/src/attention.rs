//! Atencion Causal Multi-Cabeza GQA con QK-RMSNorm, RoPE y soporte de KV-Cache.
//! Menos de 180 lineas.

use crate::config::RelicConfig;
use crate::kv_cache::LayerKvCache;
use crate::norm::RMSNorm;
use crate::ops::{linear_forward, softmax, vec_dot};
use crate::rotary::RotaryEmbedding;

#[derive(Debug, Clone)]
pub struct RelicAttention {
    pub q_proj: Vec<f32>,
    pub k_proj: Vec<f32>,
    pub v_proj: Vec<f32>,
    pub o_proj: Vec<f32>,
    pub q_norm: RMSNorm,
    pub k_norm: RMSNorm,
    pub hidden_size: usize,
    pub num_heads: usize,
    pub num_kv_heads: usize,
    pub head_dim: usize,
    pub num_kv_groups: usize,
    pub scaling: f32,
}

impl RelicAttention {
    pub fn new(config: &RelicConfig) -> Self {
        let h = config.hidden_size;
        let d = config.head_dim;
        let q_dim = config.num_attention_heads * d;
        let kv_dim = config.num_key_value_heads * d;

        Self {
            q_proj: vec![0.01; q_dim * h],
            k_proj: vec![0.01; kv_dim * h],
            v_proj: vec![0.01; kv_dim * h],
            o_proj: vec![0.01; h * q_dim],
            q_norm: RMSNorm::new(d, config.rms_norm_eps),
            k_norm: RMSNorm::new(d, config.rms_norm_eps),
            hidden_size: h,
            num_heads: config.num_attention_heads,
            num_kv_heads: config.num_key_value_heads,
            head_dim: d,
            num_kv_groups: config.num_attention_heads / config.num_key_value_heads,
            scaling: 1.0 / (d as f32).sqrt(),
        }
    }

    /// Computa la atencion de un solo token durante decodificacion autoregresiva usando KV-Cache.
    pub fn forward_decode_step(
        &self,
        out: &mut [f32],
        x_t: &[f32],
        pos: usize,
        rope: &RotaryEmbedding,
        cache: &mut LayerKvCache,
    ) {
        let q_dim = self.num_heads * self.head_dim;
        let kv_dim = self.num_kv_heads * self.head_dim;

        let mut q = vec![0.0f32; q_dim];
        let mut k = vec![0.0f32; kv_dim];
        let mut v = vec![0.0f32; kv_dim];

        linear_forward(&mut q, x_t, &self.q_proj, self.hidden_size, q_dim);
        linear_forward(&mut k, x_t, &self.k_proj, self.hidden_size, kv_dim);
        linear_forward(&mut v, x_t, &self.v_proj, self.hidden_size, kv_dim);

        for h in 0..self.num_heads {
            let slice = &mut q[h * self.head_dim..(h + 1) * self.head_dim];
            let temp = slice.to_vec();
            self.q_norm.forward(slice, &temp);
            rope.apply(slice, pos);
        }
        for kv_h in 0..self.num_kv_heads {
            let slice = &mut k[kv_h * self.head_dim..(kv_h + 1) * self.head_dim];
            let temp = slice.to_vec();
            self.k_norm.forward(slice, &temp);
            rope.apply(slice, pos);
        }

        cache.store(pos, &k, &v);

        let mut attn_out = vec![0.0f32; q_dim];
        for h in 0..self.num_heads {
            let kv_h = h / self.num_kv_groups;
            let q_vec = &q[h * self.head_dim..(h + 1) * self.head_dim];
            let mut scores = vec![0.0f32; pos + 1];

            for (t_k, score) in scores.iter_mut().enumerate().take(pos + 1) {
                let k_start = t_k * kv_dim + kv_h * self.head_dim;
                let k_vec = &cache.k_cache[k_start..k_start + self.head_dim];
                *score = vec_dot(q_vec, k_vec) * self.scaling;
            }
            softmax(&mut scores);

            let out_head = &mut attn_out[h * self.head_dim..(h + 1) * self.head_dim];
            for (t_k, &score) in scores.iter().enumerate() {
                let v_start = t_k * kv_dim + kv_h * self.head_dim;
                let v_vec = &cache.v_cache[v_start..v_start + self.head_dim];
                for (oh, &vh) in out_head.iter_mut().zip(v_vec.iter()) {
                    *oh += score * vh;
                }
            }
        }

        linear_forward(out, &attn_out, &self.o_proj, q_dim, self.hidden_size);
    }

    /// Computa atencion causal sobre una secuencia completa.
    pub fn forward_sequence(
        &self,
        out: &mut [f32],
        hidden_states: &[f32],
        seq_len: usize,
        rope: &RotaryEmbedding,
    ) {
        let q_dim = self.num_heads * self.head_dim;
        let kv_dim = self.num_kv_heads * self.head_dim;
        let mut all_q = vec![0.0f32; seq_len * q_dim];
        let mut all_k = vec![0.0f32; seq_len * kv_dim];
        let mut all_v = vec![0.0f32; seq_len * kv_dim];

        for t in 0..seq_len {
            let x_t = &hidden_states[t * self.hidden_size..(t + 1) * self.hidden_size];
            let q_t = &mut all_q[t * q_dim..(t + 1) * q_dim];
            let k_t = &mut all_k[t * kv_dim..(t + 1) * kv_dim];
            let v_t = &mut all_v[t * kv_dim..(t + 1) * kv_dim];

            linear_forward(q_t, x_t, &self.q_proj, self.hidden_size, q_dim);
            linear_forward(k_t, x_t, &self.k_proj, self.hidden_size, kv_dim);
            linear_forward(v_t, x_t, &self.v_proj, self.hidden_size, kv_dim);

            for h in 0..self.num_heads {
                let head_slice = &mut q_t[h * self.head_dim..(h + 1) * self.head_dim];
                let temp = head_slice.to_vec();
                self.q_norm.forward(head_slice, &temp);
                rope.apply(head_slice, t);
            }
            for kv_h in 0..self.num_kv_heads {
                let head_slice = &mut k_t[kv_h * self.head_dim..(kv_h + 1) * self.head_dim];
                let temp = head_slice.to_vec();
                self.k_norm.forward(head_slice, &temp);
                rope.apply(head_slice, t);
            }
        }

        let mut attn_out = vec![0.0f32; seq_len * q_dim];
        for h in 0..self.num_heads {
            let kv_h = h / self.num_kv_groups;
            for t_q in 0..seq_len {
                let q_vec = &all_q[t_q * q_dim + h * self.head_dim..t_q * q_dim + (h + 1) * self.head_dim];
                let mut scores = vec![0.0f32; t_q + 1];
                for t_k in 0..=t_q {
                    let k_vec = &all_k[t_k * kv_dim + kv_h * self.head_dim..t_k * kv_dim + (kv_h + 1) * self.head_dim];
                    scores[t_k] = vec_dot(q_vec, k_vec) * self.scaling;
                }
                softmax(&mut scores);
                let out_head = &mut attn_out[t_q * q_dim + h * self.head_dim..t_q * q_dim + (h + 1) * self.head_dim];
                for (t_k, &score) in scores.iter().enumerate() {
                    let v_vec = &all_v[t_k * kv_dim + kv_h * self.head_dim..t_k * kv_dim + (kv_h + 1) * self.head_dim];
                    for (oh, &vh) in out_head.iter_mut().zip(v_vec.iter()) {
                        *oh += score * vh;
                    }
                }
            }
        }

        for t in 0..seq_len {
            let src = &attn_out[t * q_dim..(t + 1) * q_dim];
            let dst = &mut out[t * self.hidden_size..(t + 1) * self.hidden_size];
            linear_forward(dst, src, &self.o_proj, q_dim, self.hidden_size);
        }
    }
}
