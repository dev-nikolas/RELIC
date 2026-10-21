//! Benchmark formal y reproducible en Rust puro para EpisodicMemory y ReasoningEngine.
//!
//! Carga el dataset real de GRAPH (val_id y test_ood), ejecuta las consultas
//! a través de la memoria episódica y mide con precisión de nanosegundos la latencia real.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;
use serde::Deserialize;
use relic_core::{AtomicOperator, EpisodicMemory, ReasoningEngine, RuleCatalog};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GraphSample {
    id: String,
    num_nodes: usize,
    path_len: usize,
    prompt: String,
    target: String,
}

fn main() {
    println!("============================================================");
    println!("  BENCHMARK OFICIAL DE MEMORIA EPISÓDICA EN RUST PURO");
    println!("============================================================");

    let val_id_path = Path::new("data/benchmarks/graph/val_id.json");
    let test_ood_path = Path::new("data/benchmarks/graph/test_ood.json");

    if !val_id_path.exists() || !test_ood_path.exists() {
        eprintln!("[ERROR] No se encontraron los datasets en data/benchmarks/graph/");
        return;
    }

    let file_id = File::open(val_id_path).expect("Abrir val_id.json");
    let samples_id: Vec<GraphSample> = serde_json::from_reader(BufReader::new(file_id)).expect("Parsear val_id");

    let file_ood = File::open(test_ood_path).expect("Abrir test_ood.json");
    let samples_ood: Vec<GraphSample> = serde_json::from_reader(BufReader::new(file_ood)).expect("Parsear test_ood");

    let total_samples = samples_id.len() + samples_ood.len();
    println!("Muestras cargadas: {} (ID: {}, OOD: {})", total_samples, samples_id.len(), samples_ood.len());

    let mut memory = EpisodicMemory::new();
    let mut total_correct = 0;
    let mut total_queries = 0;

    // Calentamiento
    for sample in samples_id.iter().take(10) {
        memory.clear();
        memory.load_from_text(&sample.prompt);
    }

    // Benchmark de latencia
    let start_time = Instant::now();

    for sample in samples_id.iter().chain(samples_ood.iter()) {
        memory.clear();
        memory.load_from_text(&sample.prompt);

        // Extraer start y end
        if let Some(idx) = sample.prompt.find(". ¿Camino ") {
            let query_part = &sample.prompt[idx + 10..];
            if let Some(end_idx) = query_part.find("? ->") {
                let clean_query = &query_part[..end_idx];
                if let Some((start_node, end_node)) = clean_query.split_once(" a ") {
                    let res = ReasoningEngine::solve_reachability_bfs(
                        &mut memory,
                        start_node.trim(),
                        end_node.trim(),
                        8,
                    );
                    total_queries += res.queries_used;
                    let pred = if res.is_reachable { "SI" } else { "NO" };
                    if pred == sample.target {
                        total_correct += 1;
                    }
                }
            }
        }
    }

    let elapsed = start_time.elapsed();
    let avg_latency_us = (elapsed.as_nanos() as f64 / total_samples as f64) / 1000.0;
    let accuracy = (total_correct as f64 / total_samples as f64) * 100.0;

    println!("------------------------------------------------------------");
    println!("Precisión Global Verificable (BFS): {:.2}%", accuracy);
    println!("Tiempo Total:                       {:.2?}", elapsed);
    println!("Latencia Promedio por Consulta:     {:.2} µs ({:.0} ns)", avg_latency_us, avg_latency_us * 1000.0);
    println!("Throughput de Consultas:            {:.0} queries/segundo", 1_000_000.0 / avg_latency_us);
    println!("Consultas Totales a Memoria:        {}", total_queries);
    println!("------------------------------------------------------------");

    // Test de catálogo de reglas
    let catalog = RuleCatalog::default();
    let ops = [AtomicOperator::Reverse, AtomicOperator::SwapCase, AtomicOperator::DuplicateVowels];
    let start_rules = Instant::now();
    let iters = 10_000;
    for _ in 0..iters {
        let _ = catalog.compose("RelicCognitiveUnit2026", &ops);
    }
    let elapsed_rules = start_rules.elapsed();
    let rules_us = (elapsed_rules.as_nanos() as f64 / iters as f64) / 1000.0;
    println!("Latencia Reglas (3 composiciones):  {:.2} µs / pipeline", rules_us);
    println!("Throughput Reglas:                  {:.0} ops/segundo", (iters as f64 / elapsed_rules.as_secs_f64()) * 3.0);
    println!("============================================================");
}
