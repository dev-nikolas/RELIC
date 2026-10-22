//! Benchmark oficial de memoria y razonamiento determinista BFS en Rust.
//! Menos de 180 líneas.

use std::path::Path;
use std::time::Instant;
use relic_core::{EpisodicMemory, ReasoningEngine};
use crate::monitor::get_current_rss_kb;

pub fn handle_bench() {
    println!("============================================================");
    println!("  RELIC — Benchmark Oficial de Memoria y Razonamiento");
    println!("============================================================");

    let val_id_path = Path::new("data/benchmarks/graph/val_id.json");
    if !val_id_path.exists() {
        eprintln!("[ERROR] No se encontró data/benchmarks/graph/val_id.json");
        return;
    }

    let file = std::fs::File::open(val_id_path).expect("Abrir archivo");
    let samples: Vec<serde_json::Value> = serde_json::from_reader(std::io::BufReader::new(file)).expect("Parsear JSON");
    println!("Muestras cargadas: {}", samples.len());

    let mut memory = EpisodicMemory::new();
    let start = Instant::now();
    let mut total_correct = 0;

    for s in &samples {
        memory.clear();
        let prompt = s["prompt"].as_str().unwrap();
        let target = s["target"].as_str().unwrap();

        memory.load_from_text(prompt);
        if let Some(idx) = prompt.find(". ¿Camino ") {
            let q = &prompt[idx + 10..];
            if let Some(end_idx) = q.find("? ->") {
                let clean_q = &q[..end_idx];
                if let Some((u, v)) = clean_q.split_once(" a ") {
                    let res = ReasoningEngine::solve_reachability_bfs(&mut memory, u.trim(), v.trim(), 8);
                    let pred = if res.is_reachable { "SI" } else { "NO" };
                    if pred == target {
                        total_correct += 1;
                    }
                }
            }
        }
    }

    let elapsed = start.elapsed();
    let us_per_query = (elapsed.as_nanos() as f64 / samples.len() as f64) / 1000.0;
    println!("Exactitud Determinista (BFS): {:.2}%", (total_correct as f64 / samples.len() as f64) * 100.0);
    println!("Latencia Promedio por Muestra: {:.2} µs", us_per_query);
    println!("Throughput:                   {:.0} muestras/segundo", 1_000_000.0 / us_per_query);
    println!("Memoria Física Activa RSS:    {:.2} MB", get_current_rss_kb() as f64 / 1024.0);
    println!("============================================================");
}
