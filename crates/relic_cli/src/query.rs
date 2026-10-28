//! Ejecución del despacho verificado, validación RIR v1, activación propagada y diff visual.
//! Menos de 180 líneas.

use std::path::Path;
use std::time::Instant;
use relic_core::{analyze_spectrum_donoho_stark, propagate_activation};
use relic_lexicon::{to_dot_diff, to_mermaid_diff, EngramStore};
use crate::answer::answer_intent;
use crate::dispatch::{parse_dispatch_intent_with_lexicon, DispatchIntent};
use crate::monitor::get_current_rss_kb;

pub fn load_engram_store(engram_path: Option<&str>) -> EngramStore {
    if let Some(p) = engram_path {
        let path = Path::new(p);
        if let Ok(store) = EngramStore::open_or_init_file(path) {
            return store;
        }
    }
    EngramStore::with_genesis()
}

pub fn handle_query(query: &str) {
    handle_query_with_engram(query, None);
}

pub fn handle_query_with_engram(query: &str, engram_path: Option<&str>) {
    println!("============================================================\n  RELIC — Capa de Despacho y Razonamiento Verificado\n============================================================");
    println!("Petición Recibida: \"{}\"", query);

    let t0 = Instant::now();
    let rss_before = get_current_rss_kb();
    let mut store = load_engram_store(engram_path);

    let t_parse_start = Instant::now();
    let intent = parse_dispatch_intent_with_lexicon(query, Some(store.active_view()));
    let t_parse = t_parse_start.elapsed();

    match &intent {
        DispatchIntent::Arithmetic(ref plan) => {
            println!("\n[1. RIR INTENCIÓN]: ARITHMETIC_OPERATION\n    Valor Inicial: {} {:?}\n    Operaciones:   {:?}\n    Reducciones RPN: {}", plan.initial, plan.unit, plan.steps, plan.reductions.len());
            run_verified(&intent, &mut store, t_parse, t0, rss_before);
        }
        DispatchIntent::GraphReachability(ref spec) => {
            println!("\n[1. RIR INTENCIÓN]: GRAPH_REACHABILITY");
            println!("    Estado Inicial:  {:?}", spec.initial);
            println!("    Estado Objetivo: {:?}", spec.target);
            println!("    Vía / Escala:    {:?}", spec.via);
            println!("    Evitar / Avoid:  {:?}", spec.avoid);
            run_verified(&intent, &mut store, t_parse, t0, rss_before);
        }
        _ => run_verified(&intent, &mut store, t_parse, t0, rss_before),
    }
}

fn run_verified(
    intent: &DispatchIntent,
    store: &mut EngramStore,
    t_parse: std::time::Duration,
    t0: Instant,
    rss_before: usize,
) {
    let t_engine_start = Instant::now();
    let answer = answer_intent(intent, &mut store.active_view_mut().episodic);
    let t_engine = t_engine_start.elapsed();

    println!(
        "\n[2. ESTADO]: {} ({})\n\n[3. RESPUESTA VERIFICADA]:\n  {}",
        answer.status, answer.route, answer.text
    );
    print_perf(t_parse, t_engine, t0.elapsed(), rss_before);
}

pub fn handle_activate(seed: &str, gamma: f64, steps: usize, max_entropy: f64, engram_path: Option<&str>) {
    let store = load_engram_store(engram_path);
    let res = propagate_activation(&store.active_view().episodic, &[seed], gamma, steps, max_entropy);
    println!("============================================================\n  RELIC — Activación Propagada (Spreading Activation)\n============================================================");
    println!("Semilla: {:?} | Gamma: {:.2} | Pasos: {} | Entropía H: {:.4} bits", res.seeds, res.decay_gamma, res.steps_executed, res.shannon_entropy_bits);
    println!("Estado:  {:?}", res.status);
    println!("Ranking: {:?}", res.ranking);
    println!("============================================================");
}

pub fn handle_diff(v_from: u64, v_to: u64, format: &str, engram_path: Option<&str>) {
    let store = load_engram_store(engram_path);
    match store.diff_versions(v_from, v_to) {
        Ok(diff) => {
            if format.eq_ignore_ascii_case("dot") {
                println!("{}", to_dot_diff(&diff));
            } else {
                println!("{}", to_mermaid_diff(&diff));
            }
        }
        Err(e) => eprintln!("[ERROR INTEGRIDAD DIFF]: {e}"),
    }
}

pub fn handle_spectral(csv_signal: &str, min_cycles: f64) {
    let signal: Vec<f64> = csv_signal
        .split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect();
    let outcome = analyze_spectrum_donoho_stark(&signal, 0.05, min_cycles, None);
    println!("============================================================\n  RELIC — Verificador Espectral (DFT + Donoho-Stark)\n============================================================");
    println!("{outcome:#?}");
    println!("============================================================");
}

pub fn print_perf(
    t_parse: std::time::Duration,
    t_engine: std::time::Duration,
    t_total: std::time::Duration,
    rss_before: usize,
) {
    let rss_after = get_current_rss_kb();
    let delta = rss_after.saturating_sub(rss_before);
    println!(
        "------------------------------------------------------------\nLatencia Parser: {:.1} µs | Motor: {:.1} µs | Total: {:.1} µs\nMemoria RSS: {:.2} MB (Delta: +{} KB)\n============================================================",
        t_parse.as_nanos() as f64 / 1000.0,
        t_engine.as_nanos() as f64 / 1000.0,
        t_total.as_nanos() as f64 / 1000.0,
        rss_after as f64 / 1024.0,
        delta
    );
}
