//! # relic_server::generate
//!
//! Generación autorregresiva causal con emisión Server-Sent Events (SSE) token a token.
//! Menos de 180 líneas.

use std::time::Instant;
use relic_core::{KvCache, Sampler};
use crate::state::ServerState;
use crate::types::SseTokenChunk;

pub fn handle_generate_sse(prompt_text: &str, state: &ServerState) -> Vec<u8> {
    let t0 = Instant::now();
    let mut buffer = Vec::new();

    if let (Some(model), Some(tokenizer)) = (&state.model, &state.tokenizer) {
        let token_ids = tokenizer.encode(prompt_text);
        let mut kv_cache = KvCache::new(
            state.config.num_hidden_layers,
            state.config.max_train_seq_len,
            state.config.num_key_value_heads,
            state.config.head_dim,
        );
        let mut step_logits = vec![0.0f32; state.config.vocab_size];

        for (pos, &tid) in token_ids.iter().enumerate() {
            model.forward_decode_step(tid, pos, &mut kv_cache, &mut step_logits);
        }

        let eos_id = tokenizer.vocab().eos_id;
        for _ in 0..48 {
            let next_token = Sampler::greedy(&step_logits);
            if Some(next_token) == eos_id || next_token == 315 {
                break;
            }
            let token_str = tokenizer.decode(&[next_token]).unwrap_or_default();
            let chunk = SseTokenChunk {
                token: token_str,
                token_id: next_token,
                finished: false,
                route: None,
                latency_ms: None,
            };
            if let Ok(json_str) = serde_json::to_string(&chunk) {
                buffer.extend_from_slice(format!("data: {}\n\n", json_str).as_bytes());
            }

            let pos = kv_cache.current_len;
            if pos >= state.config.max_train_seq_len {
                break;
            }
            model.forward_decode_step(next_token, pos, &mut kv_cache, &mut step_logits);
        }

        let final_chunk = SseTokenChunk {
            token: String::new(),
            token_id: 0,
            finished: true,
            route: Some("neural_model".to_string()),
            latency_ms: Some(t0.elapsed().as_nanos() as f64 / 1_000_000.0),
        };
        if let Ok(json_str) = serde_json::to_string(&final_chunk) {
            buffer.extend_from_slice(format!("data: {}\n\n", json_str).as_bytes());
        }
    } else {
        let degraded_msg = "RELIC [Modo degradado]: Pesos Safetensors no cargados. Inferencia causal en espera.";
        let chunk = SseTokenChunk {
            token: degraded_msg.to_string(),
            token_id: 0,
            finished: false,
            route: None,
            latency_ms: None,
        };
        if let Ok(json_str) = serde_json::to_string(&chunk) {
            buffer.extend_from_slice(format!("data: {}\n\n", json_str).as_bytes());
        }

        let final_chunk = SseTokenChunk {
            token: String::new(),
            token_id: 0,
            finished: true,
            route: Some("degraded_mode".to_string()),
            latency_ms: Some(t0.elapsed().as_nanos() as f64 / 1_000_000.0),
        };
        if let Ok(json_str) = serde_json::to_string(&final_chunk) {
            buffer.extend_from_slice(format!("data: {}\n\n", json_str).as_bytes());
        }
    }

    buffer
}
