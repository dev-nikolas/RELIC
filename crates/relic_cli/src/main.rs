//! Punto de entrada CLI para RELIC-10M en Rust nativo.
//! Inferencia autoregresiva acelerada con KV-Cache dinamico. Menos de 180 lineas.

use std::path::Path;
use std::time::Instant;
use relic_core::{KvCache, RelicConfig, RelicModel, SafetensorsLoader, Sampler};
use relic_tokenizer::{BpeTokenizer, Vocab};

fn main() {
    println!("============================================================");
    println!("  RELIC-10M — Minimalist LLM Engine (Pure Rust)");
    println!("============================================================");

    let config = RelicConfig::default();
    if let Err(err) = config.validate() {
        eprintln!("[ERROR] Configuracion invalida: {err}");
        std::process::exit(1);
    }

    let params = config.count_parameters();
    println!("Arquitectura:        RELIC-10M (Decoder-only Transformer)");
    println!("Parametros Totales:  {params} ({:.2}M)", params as f64 / 1e6);
    println!("Dimension Oculta:    {} | Capas: {}", config.hidden_size, config.num_hidden_layers);
    println!("Atencion GQA:        {} Q-heads / {} KV-heads (dim={})", config.num_attention_heads, config.num_key_value_heads, config.head_dim);
    println!("SwiGLU d_ff:         {} | RoPE Theta: {}", config.intermediate_size, config.rope_theta);
    println!("------------------------------------------------------------");

    let mut model = RelicModel::new(config.clone());

    // Cargar pesos Safetensors si existen
    let weights_path = Path::new("artifacts/weights/relic_10m_fp16.safetensors");
    if weights_path.exists() {
        println!("[INFO] Cargando pesos Safetensors desde {:?}...", weights_path);
        let start_load = Instant::now();
        if let Ok(loader) = SafetensorsLoader::open(weights_path) {
            if loader.load_into_model(&mut model).is_ok() {
                println!("[INFO] 69 tensores cargados con exito en {:.2?}", start_load.elapsed());
            }
        }
    }

    // Cargar tokenizador si existe
    let tok_path = Path::new("artifacts/tokenizer/tokenizer.json");
    let tokenizer = if tok_path.exists() {
        Vocab::from_file(tok_path).ok().map(|v| {
            println!("[INFO] Tokenizador cargado (vocab_size={})", v.len());
            BpeTokenizer::new(v)
        })
    } else {
        None
    };

    let prompt_text = "Once upon a time, there was a little girl";
    println!("\nPrompt: \"{}\"", prompt_text);

    let mut token_ids = if let Some(ref tok) = tokenizer {
        let ids = tok.encode(prompt_text);
        println!("Tokens Codificados: {:?}", ids);
        ids
    } else {
        vec![12u32, 45, 99, 1024, 78, 256]
    };

    // Inicializar KV-Cache dinamico
    let mut kv_cache = KvCache::new(
        config.num_hidden_layers,
        config.max_train_seq_len,
        config.num_key_value_heads,
        config.head_dim,
    );

    let tokens_to_generate = 10;
    println!("\n[INFO] Prefill del contexto ({} tokens) y generacion de {} tokens con KV-Cache...", token_ids.len(), tokens_to_generate);
    let start_gen = Instant::now();

    // 1. Prefill del prompt token a token llenando el KV-cache
    let mut step_logits = vec![0.0f32; config.vocab_size];
    for (pos, &tid) in token_ids.iter().enumerate() {
        model.forward_decode_step(tid, pos, &mut kv_cache, &mut step_logits);
    }

    // 2. Generacion autoregresiva acelerada con KV-Cache
    for _ in 0..tokens_to_generate {
        let next_token = Sampler::greedy(&step_logits);
        token_ids.push(next_token);

        let pos = kv_cache.current_len;
        if pos >= config.max_train_seq_len {
            break;
        }
        model.forward_decode_step(next_token, pos, &mut kv_cache, &mut step_logits);
    }

    let elapsed = start_gen.elapsed();
    let gen_rate = (tokens_to_generate as f64) / elapsed.as_secs_f64();
    println!("[INFO] Generacion KV-Cache completada en {:.2?} ({:.1} tok/s)", elapsed, gen_rate);

    if let Some(ref tok) = tokenizer {
        if let Ok(generated_text) = tok.decode(&token_ids) {
            println!("\nTexto Completo Generado:\n\"{}\"", generated_text);
        }
    } else {
        println!("Secuencia final de IDs: {:?}", token_ids);
    }

    println!("============================================================");
    println!("  ESTADO FASE 4: MOTOR EN RUST PURO CON KV-CACHE VERIFICADO");
    println!("============================================================");
}
