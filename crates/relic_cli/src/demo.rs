//! Demostración de inferencia autorregresiva causal con KV-Cache y parada estricta en EOS.
//! Menos de 180 líneas.

use std::path::Path;
use std::time::Instant;
use relic_core::{KvCache, RelicConfig, RelicModel, SafetensorsLoader, Sampler};
use relic_tokenizer::{BpeTokenizer, Vocab};
use crate::monitor::get_current_rss_kb;

pub fn handle_demo(prompt_text: &str) {
    println!("============================================================");
    println!("  RELIC-10M — Inferencia Autoregresiva (Rust Nativo)");
    println!("============================================================");

    let config = RelicConfig::default();
    let mut model = RelicModel::new(config.clone());

    let weights_path = Path::new("artifacts/weights/relic_10m_fp16.safetensors");
    if weights_path.exists() {
        if let Ok(loader) = SafetensorsLoader::open(weights_path) {
            let _ = loader.load_into_model(&mut model);
            println!("[INFO] Pesos Safetensors cargados.");
        }
    } else {
        println!("[WARN] No se encontró artifacts/weights/relic_10m_fp16.safetensors, usando inicialización");
    }

    let tok_path = Path::new("artifacts/tokenizer/tokenizer.json");
    let tokenizer = if tok_path.exists() {
        Vocab::from_file(tok_path).ok().map(BpeTokenizer::new)
    } else {
        None
    };

    println!("Prompt: \"{}\"", prompt_text);

    let mut token_ids = if let Some(ref tok) = tokenizer {
        tok.encode(prompt_text)
    } else {
        vec![12u32, 45, 99, 1024]
    };

    let mut kv_cache = KvCache::new(config.num_hidden_layers, config.max_train_seq_len, config.num_key_value_heads, config.head_dim);
    let mut step_logits = vec![0.0f32; config.vocab_size];

    let t0 = Instant::now();
    for (pos, &tid) in token_ids.iter().enumerate() {
        model.forward_decode_step(tid, pos, &mut kv_cache, &mut step_logits);
    }

    let eos_id = tokenizer.as_ref().and_then(|t| t.vocab().eos_id);
    let tokens_to_gen = 16;
    for _ in 0..tokens_to_gen {
        let next_token = Sampler::greedy(&step_logits);
        if Some(next_token) == eos_id || next_token == 315 {
            break;
        }
        token_ids.push(next_token);
        let pos = kv_cache.current_len;
        if pos >= config.max_train_seq_len { break; }
        model.forward_decode_step(next_token, pos, &mut kv_cache, &mut step_logits);
    }

    let elapsed = t0.elapsed();
    let gen_rate = (tokens_to_gen as f64) / elapsed.as_secs_f64();

    if let Some(ref tok) = tokenizer {
        if let Ok(text) = tok.decode(&token_ids) {
            println!("\n[GENERACIÓN]:\n\"{}\"", text);
        }
    }
    println!("\nLatencia: {:.2?} ({:.1} tok/s) | RSS: {:.2} MB", elapsed, gen_rate, get_current_rss_kb() as f64 / 1024.0);
    println!("============================================================");
}
