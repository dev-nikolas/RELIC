//! # relic_cli
//!
//! Binario de ejecución local y CLI unificado para RELIC en Rust nativo.
//! Menos de 180 líneas.

use relic_cli::bench::handle_bench;
use relic_cli::demo::handle_demo;
use relic_cli::query::handle_query;

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
            "eval-holdout" => {
                let p = args.get(2).cloned().unwrap_or_else(|| "data/curriculum_dataset_v2_20k.jsonl".to_string());
                relic_cli::eval_holdout::handle_eval_holdout(&p);
            }
            _ => {
                eprintln!("Subcomando desconocido: {}", args[1]);
                eprintln!("Uso: relic_cli [query <texto> | bench | demo <prompt> | generate <seed> <n_train> <n_holdout> <out> | eval-holdout <path>]");
            }
        }
    } else {
        handle_demo("El pingüino encontró una señal en Bogotá");
    }
}
