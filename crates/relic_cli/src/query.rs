//! Ejecución del despacho verificado, validación RIR v1 y reporte granular.
//! Menos de 180 líneas.

use std::time::Instant;
use relic_core::{
    ArithmeticOp, ArithmeticStep, EpisodicMemory, ReasoningEngine,
    RirArithmeticOp, RirDocumentV1, RirDomainKind,
};
use crate::dispatch::{parse_dispatch_intent, DispatchIntent};
use crate::knowledge::load_default_knowledge;
use crate::monitor::get_current_rss_kb;

pub fn handle_query(query: &str) {
    println!("============================================================");
    println!("  RELIC — Capa de Despacho y Razonamiento Verificado");
    println!("============================================================");
    println!("Petición Recibida: \"{}\"", query);

    let t0 = Instant::now();
    let rss_before = get_current_rss_kb();

    let t_parse_start = Instant::now();
    let intent = parse_dispatch_intent(query);
    let t_parse = t_parse_start.elapsed();

    match intent {
        DispatchIntent::Arithmetic(ref plan) => {
            let rir_steps: Vec<ArithmeticStep> = plan.steps.iter().map(|&(op, val)| {
                let rir_op = match op {
                    ArithmeticOp::Add => RirArithmeticOp::Add,
                    ArithmeticOp::Subtract => RirArithmeticOp::Subtract,
                    ArithmeticOp::Multiply => RirArithmeticOp::Multiply,
                    ArithmeticOp::Divide => RirArithmeticOp::Divide,
                };
                ArithmeticStep { operator: rir_op, operand: val }
            }).collect();

            let doc = RirDocumentV1::new_inventory(plan.unit.clone(), plan.initial, rir_steps);
            if let Err(e) = doc.validate_semantic() {
                println!("\n[1. RIR INTENCIÓN]: ARITHMETIC_OPERATION");
                println!("\n[2. ESTADO]: REJECTED (Fallo de Validación RIR: {})", e);
                println!("[RESULTADO]: ValidityStatus::INCOMPLETE");
                print_granular_performance(t_parse, std::time::Duration::from_nanos(0), t0.elapsed(), rss_before);
                return;
            }

            println!("\n[1. RIR INTENCIÓN]: ARITHMETIC_OPERATION");
            println!("    Valor Inicial: {} {:?}", plan.initial, plan.unit);
            println!("    Operaciones:   {:?}", plan.steps);

            let t_engine_start = Instant::now();
            let result = plan.execute();
            let t_engine = t_engine_start.elapsed();

            println!("\n[2. ESTADO]: VALID (Ejecución Aritmética Determinista)");
            println!("\n[3. RESPUESTA VERIFICADA]:");
            match result {
                Ok(val) => {
                    let formatted = plan.format_result(val);
                    if plan.unit.is_some() {
                        println!("  Ahora tienes {}.", formatted);
                    } else {
                        println!("  El resultado es {}.", formatted);
                    }
                }
                Err(e) => {
                    println!("  Error aritmético determinista: {}", e);
                }
            }

            print_granular_performance(t_parse, t_engine, t0.elapsed(), rss_before);
        }

        DispatchIntent::GraphReachability(ref spec) => {
            let orig = spec.initial.as_deref().unwrap_or("").to_string();
            let dest = spec.target.as_deref().unwrap_or("").to_string();

            let doc = RirDocumentV1::new_graph(
                RirDomainKind::SpatialGraph,
                orig.clone(),
                dest.clone(),
                spec.via.clone(),
                spec.avoid.clone(),
                spec.is_negated,
            );
            if let Err(e) = doc.validate_semantic() {
                println!("\n[1. RIR INTENCIÓN]: GRAPH_REACHABILITY");
                println!("\n[2. ESTADO]: REJECTED (Fallo de Validación RIR: {})", e);
                println!("[RESULTADO]: ValidityStatus::INCOMPLETE");
                print_granular_performance(t_parse, std::time::Duration::from_nanos(0), t0.elapsed(), rss_before);
                return;
            }

            println!("\n[1. RIR INTENCIÓN]: GRAPH_REACHABILITY");
            println!("    Estado Inicial:  {:?}", spec.initial);
            println!("    Estado Objetivo: {:?}", spec.target);
            println!("    Vía / Escala:    {:?}", spec.via);
            println!("    Evitar / Avoid:  {:?}", spec.avoid);

            let orig = spec.initial.as_ref().unwrap();
            let dest = spec.target.as_ref().unwrap();

            let mut memory = EpisodicMemory::new();
            load_default_knowledge(&mut memory);

            let t_engine_start = Instant::now();
            let (reachable, path, len) = if let Some(ref via_node) = spec.via {
                let r1 = ReasoningEngine::solve_reachability_bfs(&mut memory, orig, via_node, 8);
                let r2 = ReasoningEngine::solve_reachability_bfs(&mut memory, via_node, dest, 8);
                if r1.is_reachable && r2.is_reachable {
                    let mut full = r1.path;
                    full.extend(r2.path.into_iter().skip(1));
                    let total_len = full.len() - 1;
                    (true, full, total_len)
                } else {
                    (false, Vec::new(), 0)
                }
            } else {
                let res = ReasoningEngine::solve_reachability_bfs(&mut memory, orig, dest, 8);
                (res.is_reachable, res.path, res.path_length)
            };
            let t_engine = t_engine_start.elapsed();

            println!("\n[2. ESTADO]: VALID (Ejecución BFS)");
            println!("\n[3. RESPUESTA VERIFICADA]:");
            if reachable {
                let path_str = path.join(" -> ");
                if spec.is_negated {
                    println!("  Incorrecto: Sí existe conexión viable entre {} y {} (Ruta: {}, {} saltos).", orig, dest, path_str, len);
                } else {
                    let extra = if let Some(ref v) = spec.via { format!(" pasando por {}", v) } else { "".to_string() };
                    println!("  Sí, es posible alcanzar {} desde {}{} (Ruta: {}, {} saltos).", dest, orig, extra, path_str, len);
                }
            } else {
                if spec.is_negated {
                    println!("  Correcto: No existe ninguna ruta viable entre {} y {}.", orig, dest);
                } else {
                    println!("  No, no existe ninguna ruta viable desde {} hasta {}.", orig, dest);
                }
            }

            print_granular_performance(t_parse, t_engine, t0.elapsed(), rss_before);
        }

        DispatchIntent::Incomplete { ref reason, ref missing } => {
            println!("\n[1. RIR INTENCIÓN]: INCOMPLETE (Rechazo Epistémico)");
            println!("    Campo Faltante:  {}", missing);
            println!("\n[2. ESTADO]: INCOMPLETE");
            println!("\n[3. RESPUESTA VERIFICADA]:\n  Consulta incompleta: {}", reason);
            print_granular_performance(t_parse, std::time::Duration::from_nanos(0), t0.elapsed(), rss_before);
        }

        DispatchIntent::FreeLanguage(ref _text) => {
            println!("\n[1. RIR INTENCIÓN]: FREE_LANGUAGE (Rechazo seguro)");
            println!("    Consulta fuera del dominio soportado o no estructurada. Rechazando.");
            println!("[RESULTADO]: ValidityStatus::UNSUPPORTED");
        }
    }
}

pub fn print_granular_performance(
    t_parse: std::time::Duration,
    t_engine: std::time::Duration,
    t_total: std::time::Duration,
    rss_before: usize,
) {
    let rss_after = get_current_rss_kb();
    let delta = if rss_after >= rss_before { rss_after - rss_before } else { 0 };
    println!("------------------------------------------------------------");
    println!("Latencia del Parser:        {:.2?} ({:.1} µs)", t_parse, t_parse.as_nanos() as f64 / 1000.0);
    println!("Latencia del Motor:         {:.2?} ({:.1} µs)", t_engine, t_engine.as_nanos() as f64 / 1000.0);
    println!("Latencia Total End-to-End:  {:.2?} ({:.1} µs)", t_total, t_total.as_nanos() as f64 / 1000.0);
    println!("Memoria Física Activa RSS:  {:.2} MB (Delta: +{} KB)", rss_after as f64 / 1024.0, delta);
    println!("============================================================");
}
