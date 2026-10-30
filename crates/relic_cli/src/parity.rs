//! # relic_cli::parity
//!
//! Subcomando para verificación estricta de paridad numérica PyTorch vs Rust.
use std::path::Path;
use relic_core::{RelicConfig, RelicModel, SafetensorsLoader};

pub fn handle_parity(tokens_csv: &str) {
    let tokens: Vec<u32> = tokens_csv
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .collect();

    if tokens.is_empty() {
        eprintln!("[ERROR] Se requiere al menos un token ID.");
        std::process::exit(1);
    }

    let config = RelicConfig::default();
    let mut model = RelicModel::new(config.clone());

    let weights_path = Path::new("artifacts/weights/relic_10m_fp16.safetensors");
    if weights_path.exists() {
        if let Ok(loader) = SafetensorsLoader::open(weights_path) {
            let _ = loader.load_into_model(&mut model);
        }
    }

    let mut logits = vec![0.0f32; tokens.len() * config.vocab_size];
    model.forward(&tokens, &mut logits);

    // Extraer los logits del último token
    let offset = (tokens.len() - 1) * config.vocab_size;
    let last_logits = &logits[offset..offset + config.vocab_size];

    let json_output = serde_json::to_string(last_logits).unwrap_or_else(|_| "[]".to_string());
    println!("{}", json_output);
}
