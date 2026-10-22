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
            _ => {
                eprintln!("Subcomando desconocido: {}", args[1]);
                eprintln!("Uso: relic_cli [query <texto> | bench | demo <prompt>]");
            }
        }
    } else {
        handle_demo("El pingüino encontró una señal en Bogotá");
    }
}
