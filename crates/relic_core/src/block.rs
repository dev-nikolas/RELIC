//! Bloque decodificador Transformer completo en Rust puro.
use crate::attention::RelicAttention;
use crate::config::RelicConfig;
use crate::kv_cache::LayerKvCache;
use crate::mlp::SwiGluMlp;
use crate::norm::RMSNorm;
use crate::ops::vec_add_assign;
use crate::rotary::RotaryEmbedding;

#[derive(Debug, Clone)]
pub struct RelicBlock {
    pub input_layernorm: RMSNorm,
    pub self_attn: RelicAttention,
    pub post_attention_layernorm: RMSNorm,
    pub mlp: SwiGluMlp,
    pub hidden_size: usize,
}

impl RelicBlock {
    pub fn new(config: &RelicConfig) -> Self {
        Self {
            input_layernorm: RMSNorm::new(config.hidden_size, config.rms_norm_eps),
            self_attn: RelicAttention::new(config),
            post_attention_layernorm: RMSNorm::new(config.hidden_size, config.rms_norm_eps),
            mlp: SwiGluMlp::new(config),
            hidden_size: config.hidden_size,
        }
    }

    /// Ejecuta el bloque para un solo token durante decodificacion autoregresiva.
    pub fn forward_decode_step(
        &self,
        x: &mut [f32],
        pos: usize,
        rope: &RotaryEmbedding,
        cache: &mut LayerKvCache,
    ) {
        debug_assert_eq!(x.len(), self.hidden_size);

        // 1. Atencion con conexion residual y pre-RMSNorm
        let mut normed = vec![0.0f32; self.hidden_size];
        self.input_layernorm.forward(&mut normed, x);

        let mut attn_out = vec![0.0f32; self.hidden_size];
        self.self_attn.forward_decode_step(&mut attn_out, &normed, pos, rope, cache);
        vec_add_assign(x, &attn_out);

        // 2. SwiGLU MLP con conexion residual y pre-RMSNorm
        self.post_attention_layernorm.forward(&mut normed, x);
        let mut mlp_out = vec![0.0f32; self.hidden_size];
        self.mlp.forward(&mut mlp_out, &normed);
        vec_add_assign(x, &mlp_out);
    }

    /// Ejecuta el bloque decodificador sobre una secuencia completa.
    pub fn forward_sequence(
        &self,
        hidden_states: &mut [f32],
        seq_len: usize,
        rope: &RotaryEmbedding,
    ) {
        let total_elements = seq_len * self.hidden_size;
        debug_assert_eq!(hidden_states.len(), total_elements);

        let mut normed_tokens = vec![0.0f32; total_elements];
        for t in 0..seq_len {
            let src = &hidden_states[t * self.hidden_size..(t + 1) * self.hidden_size];
            let dst = &mut normed_tokens[t * self.hidden_size..(t + 1) * self.hidden_size];
            self.input_layernorm.forward(dst, src);
        }

        let mut attn_out = vec![0.0f32; total_elements];
        self.self_attn.forward_sequence(&mut attn_out, &normed_tokens, seq_len, rope);
        vec_add_assign(hidden_states, &attn_out);

        for t in 0..seq_len {
            let x_t = &hidden_states[t * self.hidden_size..(t + 1) * self.hidden_size];
            let mut normed_t = vec![0.0f32; self.hidden_size];
            self.post_attention_layernorm.forward(&mut normed_t, x_t);

            let mut mlp_out_t = vec![0.0f32; self.hidden_size];
            self.mlp.forward(&mut mlp_out_t, &normed_t);

            let dst = &mut hidden_states[t * self.hidden_size..(t + 1) * self.hidden_size];
            vec_add_assign(dst, &mlp_out_t);
        }
    }
}
