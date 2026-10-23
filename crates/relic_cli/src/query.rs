//! Ejecución del despacho verificado, validación RIR v1 y reporte granular. Menos de 180 líneas.

use std::time::Instant;
use relic_core::EpisodicMemory;
use crate::answer::answer_intent;
use crate::dispatch::{parse_dispatch_intent, DispatchIntent};
use crate::knowledge::load_default_knowledge;
use crate::monitor::get_current_rss_kb;

pub fn handle_query(query: &str) {
    println!("============================================================\n  RELIC — Capa de Despacho y Razonamiento Verificado\n============================================================");
    println!("Petición Recibida: \"{}\"", query);

    let t0 = Instant::now();
    let rss_before = get_current_rss_kb();
    let t_parse_start = Instant::now();
    let intent = parse_dispatch_intent(query);
    let t_parse = t_parse_start.elapsed();

    match &intent {
        DispatchIntent::Arithmetic(ref plan) => {
            println!("\n[1. RIR INTENCIÓN]: ARITHMETIC_OPERATION\n    Valor Inicial: {} {:?}\n    Operaciones:   {:?}", plan.initial, plan.unit, plan.steps);
            run_verified(&intent, t_parse, t0, rss_before);
        }

        DispatchIntent::GraphReachability(ref spec) => {
            println!("\n[1. RIR INTENCIÓN]: GRAPH_REACHABILITY");
            println!("    Estado Inicial:  {:?}", spec.initial);
            println!("    Estado Objetivo: {:?}", spec.target);
            println!("    Vía / Escala:    {:?}", spec.via);
            println!("    Evitar / Avoid:  {:?}", spec.avoid);
            run_verified(&intent, t_parse, t0, rss_before);
        }

        _ => run_verified(&intent, t_parse, t0, rss_before),
    }
}

fn run_verified(intent: &DispatchIntent, t_parse: std::time::Duration, t0: Instant, rss_before: usize) {
    let t_engine_start = Instant::now();
    let mut memory = EpisodicMemory::new();
    load_default_knowledge(&mut memory);
    let answer = answer_intent(intent, &mut memory);
    let t_engine = t_engine_start.elapsed();

    println!("\n[2. ESTADO]: {} ({})\n\n[3. RESPUESTA VERIFICADA]:\n  {}", answer.status, answer.route, answer.text);
    print_perf(t_parse, t_engine, t0.elapsed(), rss_before);
}

pub fn print_perf(t_parse: std::time::Duration, t_engine: std::time::Duration, t_total: std::time::Duration, rss_before: usize) {
    let rss_after = get_current_rss_kb();
    let delta = if rss_after >= rss_before { rss_after - rss_before } else { 0 };
    println!("------------------------------------------------------------\nLatencia Parser: {:.1} µs | Motor: {:.1} µs | Total: {:.1} µs\nMemoria RSS: {:.2} MB (Delta: +{} KB)\n============================================================", t_parse.as_nanos() as f64 / 1000.0, t_engine.as_nanos() as f64 / 1000.0, t_total.as_nanos() as f64 / 1000.0, rss_after as f64 / 1024.0, delta);
}
