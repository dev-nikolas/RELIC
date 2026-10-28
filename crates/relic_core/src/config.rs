//! Configuracion arquitectonica formal de RELIC-10M.
//! Valida todas las invariantes estructurales. Menos de 180 lineas.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicConfig {
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub num_key_value_heads: usize,
    pub head_dim: usize,
    pub max_position_embeddings: usize,
    pub max_train_seq_len: usize,
    pub rope_theta: f32,
    pub rms_norm_eps: f32,
}

impl Default for RelicConfig {
    fn default() -> Self {
        Self {
            vocab_size: 8192,
            hidden_size: 288,
            intermediate_size: 768,
            num_hidden_layers: 6,
            num_attention_heads: 6,
            num_key_value_heads: 2,
            head_dim: 48,
            max_position_embeddings: 512,
            max_train_seq_len: 256,
            rope_theta: 10000.0,
            rms_norm_eps: 1e-6,
        }
    }
}

impl RelicConfig {
    /// Valida invariantes fundamentales de la red neuronal.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.hidden_size != self.num_attention_heads * self.head_dim {
            return Err("hidden_size debe ser igual a num_attention_heads * head_dim");
        }
        if !self.num_attention_heads.is_multiple_of(self.num_key_value_heads) {
            return Err("num_attention_heads debe ser divisible por num_key_value_heads");
        }
        if !self.head_dim.is_multiple_of(2) {
            return Err("head_dim debe ser par para permitir RoPE");
        }
        Ok(())
    }

    /// Calcula analiticamente el conteo exacto de parametros del modelo con pesos desatados.
    pub fn count_parameters(&self) -> usize {
        let embed = self.vocab_size * self.hidden_size;
        let w_q = self.hidden_size * (self.num_attention_heads * self.head_dim);
        let w_k = self.hidden_size * (self.num_key_value_heads * self.head_dim);
        let w_v = self.hidden_size * (self.num_key_value_heads * self.head_dim);
        let w_o = (self.num_attention_heads * self.head_dim) * self.hidden_size;
        let qk_norm = 2 * self.head_dim;
        let attn_layer = w_q + w_k + w_v + w_o + qk_norm;

        let gate = self.hidden_size * self.intermediate_size;
        let up = self.hidden_size * self.intermediate_size;
        let down = self.intermediate_size * self.hidden_size;
        let mlp_layer = gate + up + down;

        let rms_norms = 2 * self.hidden_size;
        let layer_total = attn_layer + mlp_layer + rms_norms;

        let final_norm = self.hidden_size;
        let lm_head = self.hidden_size * self.vocab_size;

        embed + (self.num_hidden_layers * layer_total) + final_norm + lm_head
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_parameter_count() {
        let cfg = RelicConfig::default();
        assert_eq!(cfg.validate(), Ok(()));
        assert_eq!(cfg.count_parameters(), 10_031_328);
    }
}
