//! Arquitectura decodificadora zero-copy RELIC-10M sobre páginas mmap F16.
//! Inferencia directa sin alojar los tensores en el heap anónimo privado.
//! Menos de 180 líneas.

use crate::config::RelicConfig;
use crate::kv_cache::KvCache;
use crate::loader::{half_to_f32, SafetensorsLoader};
use crate::norm::RMSNorm;
use crate::ops::{linear_forward_f16, softmax, swiglu_forward, vec_add_assign, vec_dot};
use crate::rotary::RotaryEmbedding;

pub struct RelicLayerMmap<'a> {
    pub input_layernorm: RMSNorm,
    pub post_attention_layernorm: RMSNorm,
    pub q_norm: RMSNorm,
    pub k_norm: RMSNorm,
    pub q_proj: &'a [u8],
    pub k_proj: &'a [u8],
    pub v_proj: &'a [u8],
    pub o_proj: &'a [u8],
    pub gate_proj: &'a [u8],
    pub up_proj: &'a [u8],
    pub down_proj: &'a [u8],
}

pub struct RelicModelMmap<'a> {
    pub config: RelicConfig,
    pub embed_tokens: &'a [u8],
    pub norm: RMSNorm,
    pub lm_head: &'a [u8],
    pub layers: Vec<RelicLayerMmap<'a>>,
    pub rope: RotaryEmbedding,
}

impl<'a> RelicModelMmap<'a> {
    pub fn from_loader(loader: &'a SafetensorsLoader, config: RelicConfig) -> Result<Self, String> {
        let embed_tokens = loader.get_tensor_f16("model.embed_tokens.weight")?;
        let lm_head = loader.get_tensor_f16("lm_head.weight")?;

        let mut norm = RMSNorm::new(config.hidden_size, config.rms_norm_eps);
        norm.weight = loader.get_tensor_f32("model.norm.weight")?;

        let rope = RotaryEmbedding::new(
            config.head_dim,
            config.max_position_embeddings,
            config.rope_theta,
        );

        let mut layers = Vec::with_capacity(config.num_hidden_layers);
        for i in 0..config.num_hidden_layers {
            let p = format!("model.layers.{i}");
            let mut in_ln = RMSNorm::new(config.hidden_size, config.rms_norm_eps);
            in_ln.weight = loader.get_tensor_f32(&format!("{p}.input_layernorm.weight"))?;

            let mut post_ln = RMSNorm::new(config.hidden_size, config.rms_norm_eps);
            post_ln.weight = loader.get_tensor_f32(&format!("{p}.post_attention_layernorm.weight"))?;

            let mut q_norm = RMSNorm::new(config.head_dim, config.rms_norm_eps);
            q_norm.weight = loader.get_tensor_f32(&format!("{p}.self_attn.q_norm.weight"))?;

            let mut k_norm = RMSNorm::new(config.head_dim, config.rms_norm_eps);
            k_norm.weight = loader.get_tensor_f32(&format!("{p}.self_attn.k_norm.weight"))?;

            layers.push(RelicLayerMmap {
                input_layernorm: in_ln,
                post_attention_layernorm: post_ln,
                q_norm,
                k_norm,
                q_proj: loader.get_tensor_f16(&format!("{p}.self_attn.q_proj.weight"))?,
                k_proj: loader.get_tensor_f16(&format!("{p}.self_attn.k_proj.weight"))?,
                v_proj: loader.get_tensor_f16(&format!("{p}.self_attn.v_proj.weight"))?,
                o_proj: loader.get_tensor_f16(&format!("{p}.self_attn.o_proj.weight"))?,
                gate_proj: loader.get_tensor_f16(&format!("{p}.mlp.gate_proj.weight"))?,
                up_proj: loader.get_tensor_f16(&format!("{p}.mlp.up_proj.weight"))?,
                down_proj: loader.get_tensor_f16(&format!("{p}.mlp.down_proj.weight"))?,
            });
        }

        Ok(Self { config, embed_tokens, norm, lm_head, layers, rope })
    }

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

        // 1. Embedding lookup directo de páginas F16
        let tid = (token_id as usize).min(v - 1);
        let row_bytes = &self.embed_tokens[tid * h * 2..(tid + 1) * h * 2];
        let (chunks, _) = row_bytes.as_chunks::<2>();
        let mut x = Vec::with_capacity(h);
        for chunk in chunks {
            x.push(half_to_f32(u16::from_le_bytes(*chunk)));
        }

        // 2. Capas del Transformer
        let q_dim = self.config.num_attention_heads * self.config.head_dim;
        let kv_dim = self.config.num_key_value_heads * self.config.head_dim;
        let num_kv_groups = self.config.num_attention_heads / self.config.num_key_value_heads;
        let scale = 1.0 / (self.config.head_dim as f32).sqrt();

        for (i, layer) in self.layers.iter().enumerate() {
            let mut normed = vec![0.0f32; h];
            layer.input_layernorm.forward(&mut normed, &x);

            let mut q = vec![0.0f32; q_dim];
            let mut k = vec![0.0f32; kv_dim];
            let mut v = vec![0.0f32; kv_dim];
            linear_forward_f16(&mut q, &normed, layer.q_proj, h, q_dim);
            linear_forward_f16(&mut k, &normed, layer.k_proj, h, kv_dim);
            linear_forward_f16(&mut v, &normed, layer.v_proj, h, kv_dim);

            for head in 0..self.config.num_attention_heads {
                let sl = &mut q[head * self.config.head_dim..(head + 1) * self.config.head_dim];
                let tmp = sl.to_vec();
                layer.q_norm.forward(sl, &tmp);
                self.rope.apply(sl, pos);
            }
            for kv_h in 0..self.config.num_key_value_heads {
                let sl = &mut k[kv_h * self.config.head_dim..(kv_h + 1) * self.config.head_dim];
                let tmp = sl.to_vec();
                layer.k_norm.forward(sl, &tmp);
                self.rope.apply(sl, pos);
            }

            cache.layers[i].store(pos, &k, &v);

            let mut attn_out = vec![0.0f32; q_dim];
            for head in 0..self.config.num_attention_heads {
                let kv_h = head / num_kv_groups;
                let q_vec = &q[head * self.config.head_dim..(head + 1) * self.config.head_dim];
                let mut scores = vec![0.0f32; pos + 1];
                for (t_k, score) in scores.iter_mut().enumerate().take(pos + 1) {
                    let k_start = t_k * kv_dim + kv_h * self.config.head_dim;
                    let k_vec = &cache.layers[i].k_cache[k_start..k_start + self.config.head_dim];
                    *score = vec_dot(q_vec, k_vec) * scale;
                }
                softmax(&mut scores);
                let out_head = &mut attn_out[head * self.config.head_dim..(head + 1) * self.config.head_dim];
                for (t_k, &sc) in scores.iter().enumerate() {
                    let v_start = t_k * kv_dim + kv_h * self.config.head_dim;
                    let v_vec = &cache.layers[i].v_cache[v_start..v_start + self.config.head_dim];
                    for (oh, &vh) in out_head.iter_mut().zip(v_vec.iter()) { *oh += sc * vh; }
                }
            }

            let mut proj_out = vec![0.0f32; h];
            linear_forward_f16(&mut proj_out, &attn_out, layer.o_proj, q_dim, h);
            vec_add_assign(&mut x, &proj_out);

            layer.post_attention_layernorm.forward(&mut normed, &x);
            let inter_dim = self.config.intermediate_size;
            let mut gate = vec![0.0f32; inter_dim];
            let mut up = vec![0.0f32; inter_dim];
            linear_forward_f16(&mut gate, &normed, layer.gate_proj, h, inter_dim);
            linear_forward_f16(&mut up, &normed, layer.up_proj, h, inter_dim);
            let mut swiglu = vec![0.0f32; inter_dim];
            swiglu_forward(&mut swiglu, &gate, &up);
            let mut mlp_out = vec![0.0f32; h];
            linear_forward_f16(&mut mlp_out, &swiglu, layer.down_proj, inter_dim, h);
            vec_add_assign(&mut x, &mlp_out);
        }

        let mut final_normed = vec![0.0f32; h];
        self.norm.forward(&mut final_normed, &x);
        linear_forward_f16(logits_out, &final_normed, self.lm_head, h, v);
        cache.advance();
    }
}

