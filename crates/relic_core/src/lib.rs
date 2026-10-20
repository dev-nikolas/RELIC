//! # relic_core
//!
//! Motor neuronal e inferencia para RELIC-10M en Rust nativo.
//! Diseno modular, cero dependencias C++, multiplataforma.
//! Menos de 180 lineas.

pub mod attention;
pub mod block;
pub mod config;
pub mod kv_cache;
pub mod loader;
pub mod mlp;
pub mod model;
pub mod norm;
pub mod ops;
pub mod rotary;
pub mod sampler;

pub use attention::RelicAttention;
pub use block::RelicBlock;
pub use config::RelicConfig;
pub use kv_cache::{KvCache, LayerKvCache};
pub use loader::SafetensorsLoader;
pub use mlp::SwiGluMlp;
pub use model::RelicModel;
pub use norm::RMSNorm;
pub use rotary::RotaryEmbedding;
pub use sampler::Sampler;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_relic_model_forward_pass() {
        let mut cfg = RelicConfig::default();
        cfg.vocab_size = 64;
        cfg.hidden_size = 48;
        cfg.num_attention_heads = 2;
        cfg.num_key_value_heads = 1;
        cfg.head_dim = 24;
        cfg.intermediate_size = 96;
        cfg.num_hidden_layers = 2;

        let model = RelicModel::new(cfg.clone());
        let input_tokens = [1u32, 5u32, 12u32];
        let mut logits = vec![0.0f32; input_tokens.len() * cfg.vocab_size];

        model.forward(&input_tokens, &mut logits);

        assert_eq!(logits.len(), 3 * 64);
        for &val in &logits {
            assert!(!val.is_nan(), "Logit no debe ser NaN");
            assert!(!val.is_infinite(), "Logit debe ser finito");
        }
    }

    #[test]
    fn test_kv_cache_decode_step() {
        let mut cfg = RelicConfig::default();
        cfg.vocab_size = 64;
        cfg.hidden_size = 48;
        cfg.num_attention_heads = 2;
        cfg.num_key_value_heads = 1;
        cfg.head_dim = 24;
        cfg.intermediate_size = 96;
        cfg.num_hidden_layers = 2;

        let model = RelicModel::new(cfg.clone());
        let mut cache = KvCache::new(cfg.num_hidden_layers, 32, cfg.num_key_value_heads, cfg.head_dim);
        let mut logits = vec![0.0f32; cfg.vocab_size];

        model.forward_decode_step(5, 0, &mut cache, &mut logits);
        assert_eq!(cache.current_len, 1);
        assert!(!logits[0].is_nan());
    }
}
