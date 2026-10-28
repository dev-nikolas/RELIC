//! Exportador de datasets JSONL con hash SHA-256 y control de fugas de n-gramas.
//! Menos de 180 líneas.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use relic_core::Sha256;
use super::types::{CurriculumLevel, CurriculumSample, DatasetSplit};
use super::engine::generate_sample;

pub fn extract_3grams(text: &str) -> HashSet<String> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut ngrams = HashSet::new();
    if tokens.len() < 3 { return ngrams; }
    for i in 0..=(tokens.len() - 3) {
        ngrams.insert(format!("{} {} {}", tokens[i], tokens[i+1], tokens[i+2]));
    }
    ngrams
}

pub fn generate_and_export_curriculum(
    seed: u64,
    samples_per_level_train: usize,
    samples_per_level_holdout: usize,
    out_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut train_samples: Vec<CurriculumSample> = Vec::new();
    let mut holdout_samples: Vec<CurriculumSample> = Vec::new();

    let levels = [
        CurriculumLevel::L01DirectArithmetic,
        CurriculumLevel::L02InventoryUnits,
        CurriculumLevel::L03EpistemicRejection,
    ];

    for &lvl in &levels {
        for i in 0..samples_per_level_train {
            train_samples.push(generate_sample(seed, lvl, DatasetSplit::Train, i));
        }
        for i in 0..samples_per_level_holdout {
            holdout_samples.push(generate_sample(seed, lvl, DatasetSplit::HoldoutTemplate, i));
        }
    }

    // Análisis de fuga de n-gramas entre Train y Holdout
    let mut train_3grams = HashSet::new();
    for s in &train_samples {
        for g in extract_3grams(&s.prompt) { train_3grams.insert(g); }
    }
    let mut leak_count = 0;
    let mut total_holdout_grams = 0;
    for s in &holdout_samples {
        for g in extract_3grams(&s.prompt) {
            total_holdout_grams += 1;
            if train_3grams.contains(&g) { leak_count += 1; }
        }
    }

    let leak_rate = if total_holdout_grams > 0 { 100.0 * leak_count as f64 / total_holdout_grams as f64 } else { 0.0 };
    println!("Control de Fugas: Solapamiento 3-gramas Train vs Hold-Out: {}/{} ({:.2}%)", leak_count, total_holdout_grams, leak_rate);

    let file = File::create(out_path)?;
    let mut writer = BufWriter::new(file);
    let mut hasher = Sha256::new();

    let all_samples: Vec<&CurriculumSample> = train_samples.iter().chain(holdout_samples.iter()).collect();
    for s in &all_samples {
        let json_line = serde_json::to_string(s)?;
        hasher.update(json_line.as_bytes());
        hasher.update(b"\n");
        writeln!(writer, "{}", json_line)?;
    }
    writer.flush()?;

    let hash_hex = hasher.finalize_hex();
    println!("Exportadas {} muestras a {}. Hash SHA-256: {}", all_samples.len(), out_path, hash_hex);
    Ok(hash_hex)
}
