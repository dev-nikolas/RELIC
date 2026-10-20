//! Arquitectura neuronal completa RELIC-10M con soporte de prefill y decode con KV-Cache.
//! Menos de 180 lineas.

use crate::block::RelicBlock;
use crate::config::RelicConfig;
use crate::kv_cache::KvCache;
use crate::norm::RMSNorm;
use crate::ops::linear_forward;
use crate::rotary::RotaryEmbedding;

#[derive(Debug, Clone)]
pub struct RelicModel {
    pub config: RelicConfig,
    pub embed_tokens: Vec<f32>,
    pub layers: Vec<RelicBlock>,
    pub norm: RMSNorm,
    pub lm_head: Vec<f32>,
    pub rope: RotaryEmbedding,
}

impl RelicModel {
    pub fn new(config: RelicConfig) -> Self {
        let v = config.vocab_size;
        let h = config.hidden_size;
        let layers = (0..config.num_hidden_layers)
            .map(|_| RelicBlock::new(&config))
            .collect();

        let rope = RotaryEmbedding::new(
            config.head_dim,
            config.max_position_embeddings,
            config.rope_theta,
        );

        Self {
            embed_tokens: vec![0.01; v * h],
            layers,
            norm: RMSNorm::new(h, config.rms_norm_eps),
            lm_head: vec![0.01; v * h],
            rope,
            config,
        }
    }

    /// Decodifica un solo token autoregresivo con KV-Cache en tiempo O(1) proyecciones.
    pub fn forward_decode_step(
        &self,
        token_id: u32,
        pos: usize,
        cache: &mut KvCache,
        logits_out: &mut [f32],
    ) {
        let h = self.config.hidden_size;
        let v = self.config.vocab_size;
        debug_assert_eq!(logits_out.len(), v);

        // 1. Embedding lookup del token
        let tid = (token_id as usize).min(v - 1);
        let mut x = self.embed_tokens[tid * h..(tid + 1) * h].to_vec();

        // 2. Paso a traves de los bloques decodificadores con KV-Cache
        for (i, layer) in self.layers.iter().enumerate() {
            layer.forward_decode_step(&mut x, pos, &self.rope, &mut cache.layers[i]);
        }

        // 3. Normalizacion RMSNorm final y proyeccion LM Head
        let mut normed = vec![0.0f32; h];
        self.norm.forward(&mut normed, &x);
        linear_forward(logits_out, &normed, &self.lm_head, h, v);

        cache.advance();
    }

    /// Forward pass causal completo sobre secuencia completa.
    pub fn forward(&self, input_ids: &[u32], logits_out: &mut [f32]) {
        let seq_len = input_ids.len();
        let h = self.config.hidden_size;
        let v = self.config.vocab_size;
        debug_assert_eq!(logits_out.len(), seq_len * v);

        let mut hidden_states = vec![0.0f32; seq_len * h];
        for (t, &id) in input_ids.iter().enumerate() {
            let token_id = (id as usize).min(v - 1);
            let embed_row = &self.embed_tokens[token_id * h..(token_id + 1) * h];
            let dst = &mut hidden_states[t * h..(t + 1) * h];
            dst.copy_from_slice(embed_row);
        }

        for layer in &self.layers {
            layer.forward_sequence(&mut hidden_states, seq_len, &self.rope);
        }

        for t in 0..seq_len {
            let h_t = &hidden_states[t * h..(t + 1) * h];
            let mut normed_t = vec![0.0f32; h];
            self.norm.forward(&mut normed_t, h_t);

            let logits_t = &mut logits_out[t * v..(t + 1) * v];
            linear_forward(logits_t, &normed_t, &self.lm_head, h, v);
        }
    }
}
