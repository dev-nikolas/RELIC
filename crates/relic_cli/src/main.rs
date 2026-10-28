//! # relic_cli
//!
//! Binario de ejecución local y CLI unificado para RELIC en Rust nativo.
//! Menos de 180 líneas.

use relic_cli::bench::handle_bench;
use relic_cli::demo::handle_demo;
use relic_cli::query::{handle_activate, handle_diff, handle_query, handle_spectral};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "query" => {
                let query = if args.len() > 2 {
                    args[2..].join(" ")
                } else {
                    "¿Depende el crate relic_cli del paquete memmap2?".to_string()
                };
                handle_query(&query);
            }
            "activate" => {
                let seed = args.get(2).map(|s| s.as_str()).unwrap_or("Madrid");
                let gamma = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.8);
                let steps = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2);
                let max_h = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(1.5);
                let path = args.get(6).map(|s| s.as_str());
                handle_activate(seed, gamma, steps, max_h, path);
            }
            "diff" => {
                let v_from = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
                let v_to = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(13);
                let fmt = args.get(4).map(|s| s.as_str()).unwrap_or("mermaid");
                let path = args.get(5).map(|s| s.as_str());
                handle_diff(v_from, v_to, fmt, path);
            }
            "spectral" => {
                let sig = args
                    .get(2)
                    .map(|s| s.as_str())
                    .unwrap_or("1,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0");
                let min_c = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3.0);
                handle_spectral(sig, min_c);
            }
            "bench" => handle_bench(),
            "parity" => {
                let tokens = if args.len() > 2 { args[2].clone() } else { "12,45,99".to_string() };
                relic_cli::parity::handle_parity(&tokens);
            }
            "demo" => {
                let prompt = if args.len() > 2 {
                    args[2..].join(" ")
                } else {
                    "El pingüino encontró una señal en Bogotá".to_string()
                };
                handle_demo(&prompt);
            }
            #[cfg(feature = "offline-tools")]
            "generate" => {
                let seed = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(42);
                let count_train = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
                let count_holdout = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(25);
                let out_path = args.get(5).cloned().unwrap_or_else(|| "../../data/curriculum_dataset_v1.jsonl".to_string());
                match relic_cli::generator::generate_and_export_curriculum(seed, count_train, count_holdout, &out_path) {
                    Ok(hash) => println!("[OK] Dataset generado con éxito. Hash: {}", hash),
                    Err(e) => eprintln!("[ERROR] Falló generación de dataset: {}", e),
                }
            }
            #[cfg(feature = "offline-tools")]
            "eval-holdout" => {
                let p = args.get(2).cloned().unwrap_or_else(|| "data/curriculum_dataset_v2_20k.jsonl".to_string());
                relic_cli::eval_holdout::handle_eval_holdout(&p);
            }
            _ => {
                eprintln!("Subcomando desconocido: {}", args[1]);
                eprintln!("Uso: relic_cli [query <texto> | activate <seed> [gamma] [steps] [max_h] [jsonl] | diff <v_from> <v_to> [mermaid|dot] [jsonl] | bench | demo <prompt> | generate | eval-holdout]");
            }
        }
    } else {
        handle_demo("El pingüino encontró una señal en Bogotá");
    }
}
