//! Benchmark 1 — Estudio de Ablación: Léxico Clásico (Fase 4) vs. Interferencia Cuántica de Fase (Fase 5).
//! Menos de 180 líneas.

use std::f64::consts::PI;
use std::time::Instant;
use relic_core::{ArithmeticOp, CollapseDecision};
use relic_lexicon::{ClassicalPolysemyResult, EngramStore, EntryStatus, LexiconPayload};
use serde::{Deserialize, Serialize};
use crate::bench::compute_mean_and_p99;

/// Métricas comparativas del Benchmark 1 (Ablación Léxico Clásico vs. Interferencia de Fase Cuántica).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolysemyAblationMetrics {
    pub total_cases: usize,
    pub classical_resolved_accuracy_pct: f64,
    pub classical_coverage_pct: f64,
    pub classical_contradiction_error_rate_pct: f64,
    pub classical_f1: f64,
    pub classical_mean_latency_us: f64,
    pub classical_p99_latency_us: f64,
    pub quantum_resolved_accuracy_pct: f64,
    pub quantum_coverage_pct: f64,
    pub quantum_contradiction_error_rate_pct: f64,
    pub quantum_f1: f64,
    pub quantum_mean_latency_us: f64,
    pub quantum_p99_latency_us: f64,
}

struct PolysemyTestCase {
    word: &'static str,
    context: Vec<&'static str>,
    expected: Option<ArithmeticOp>,
}

type PolysemyVerbDef<'a> = (&'a str, ArithmeticOp, ArithmeticOp, &'a [&'a str], &'a [&'a str]);

/// Ejecuta el Benchmark 1: Estudio de Ablación Clásico (Fase 4) vs. Interferencia Cuántica de Fase (Fase 5).
pub fn run_bench1_polysemy_ablation() -> PolysemyAblationMetrics {
    let mut store = EngramStore::with_genesis();
    let definitions: Vec<PolysemyVerbDef<'_>> = vec![
        ("bajar", ArithmeticOp::Subtract, ArithmeticOp::Divide, &["descuento", "rebaja", "impuesto"], &["escala", "proporcion"]),
        ("tumbar", ArithmeticOp::Subtract, ArithmeticOp::Divide, &["comision", "multa", "cobro"], &["fraccion", "reparto"]),
        ("coronar", ArithmeticOp::Add, ArithmeticOp::Multiply, &["bono", "premio", "propina"], &["racha", "factor"]),
        ("quemar", ArithmeticOp::Subtract, ArithmeticOp::Multiply, &["gasto", "merma", "reserva"], &["potencia", "impulso"]),
        ("partir", ArithmeticOp::Divide, ArithmeticOp::Subtract, &["lote", "herencia", "grupo"], &["tajada", "retiro"]),
    ];

    let mut ts = 10_000u64;
    for (word, op_a, op_b, ctxs_a, ctxs_b) in definitions {
        store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::quantum_polysemous_alias(word, op_a, 1.0, 0.0, None)).unwrap();
        ts += 1;
        store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::quantum_polysemous_alias(word, op_b, 1.0, 0.0, None)).unwrap();
        ts += 1;
        for &ca in ctxs_a {
            store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::quantum_polysemous_alias(word, op_a, 1.0, 0.0, Some(ca.to_string()))).unwrap();
            ts += 1;
            store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::quantum_polysemous_alias(word, op_b, 1.0, PI, Some(ca.to_string()))).unwrap();
            ts += 1;
        }
        for &cb in ctxs_b {
            store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::quantum_polysemous_alias(word, op_b, 1.0, 0.0, Some(cb.to_string()))).unwrap();
            ts += 1;
            store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::quantum_polysemous_alias(word, op_a, 1.0, PI, Some(cb.to_string()))).unwrap();
            ts += 1;
        }
    }

    let cases = vec![
        PolysemyTestCase { word: "bajar", context: vec!["descuento", "tienda"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "bajar", context: vec!["rebaja", "efectivo"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "bajar", context: vec!["impuesto", "factura"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "bajar", context: vec!["escala", "mapa"], expected: Some(ArithmeticOp::Divide) },
        PolysemyTestCase { word: "bajar", context: vec!["proporcion", "plano"], expected: Some(ArithmeticOp::Divide) },
        PolysemyTestCase { word: "tumbar", context: vec!["comision", "banco"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "tumbar", context: vec!["multa", "transito"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "tumbar", context: vec!["cobro", "servicio"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "tumbar", context: vec!["fraccion", "total"], expected: Some(ArithmeticOp::Divide) },
        PolysemyTestCase { word: "tumbar", context: vec!["reparto", "socios"], expected: Some(ArithmeticOp::Divide) },
        PolysemyTestCase { word: "coronar", context: vec!["bono", "ventas"], expected: Some(ArithmeticOp::Add) },
        PolysemyTestCase { word: "coronar", context: vec!["premio", "torneo"], expected: Some(ArithmeticOp::Add) },
        PolysemyTestCase { word: "coronar", context: vec!["propina", "mesa"], expected: Some(ArithmeticOp::Add) },
        PolysemyTestCase { word: "coronar", context: vec!["racha", "puntos"], expected: Some(ArithmeticOp::Multiply) },
        PolysemyTestCase { word: "coronar", context: vec!["factor", "apuesta"], expected: Some(ArithmeticOp::Multiply) },
        PolysemyTestCase { word: "quemar", context: vec!["gasto", "viaje"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "quemar", context: vec!["merma", "bodega"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "quemar", context: vec!["potencia", "motor"], expected: Some(ArithmeticOp::Multiply) },
        PolysemyTestCase { word: "partir", context: vec!["lote", "terreno"], expected: Some(ArithmeticOp::Divide) },
        PolysemyTestCase { word: "partir", context: vec!["tajada", "pastel"], expected: Some(ArithmeticOp::Subtract) },
        PolysemyTestCase { word: "bajar", context: vec!["hoy"], expected: None },
        PolysemyTestCase { word: "tumbar", context: vec!["ahora"], expected: None },
        PolysemyTestCase { word: "coronar", context: vec!["luego"], expected: None },
        PolysemyTestCase { word: "partir", context: vec!["pronto"], expected: None },
    ];

    let total_cases = cases.len();
    let answerable_count = cases.iter().filter(|c| c.expected.is_some()).count();
    let (mut classical_latencies, mut quantum_latencies) = (Vec::with_capacity(total_cases), Vec::with_capacity(total_cases));
    let (mut classical_decided, mut classical_correct_decided, mut classical_contradiction_errors) = (0usize, 0usize, 0usize);
    let (mut quantum_decided, mut quantum_correct_decided, mut quantum_contradiction_errors) = (0usize, 0usize, 0usize);

    let view = store.active_view();
    for case in &cases {
        let t0_c = Instant::now();
        let res_c = view.resolve_polysemous_classical(case.word, &case.context);
        classical_latencies.push(t0_c.elapsed().as_nanos() as f64 / 1000.0);

        match (res_c, case.expected) {
            (ClassicalPolysemyResult::Resolved { target_op, .. }, Some(exp)) => {
                classical_decided += 1;
                if target_op == exp { classical_correct_decided += 1; } else { classical_contradiction_errors += 1; }
            }
            (ClassicalPolysemyResult::Resolved { .. }, None) => { classical_decided += 1; classical_contradiction_errors += 1; }
            (ClassicalPolysemyResult::Ambiguous { .. }, Some(_)) => { classical_contradiction_errors += 1; }
            _ => {}
        }

        let t0_q = Instant::now();
        let res_q = view.resolve_polysemous_quantum(case.word, &case.context, 3.0, 0.0).expect("Entrada polisémica registrada");
        quantum_latencies.push(t0_q.elapsed().as_nanos() as f64 / 1000.0);

        match (res_q, case.expected) {
            (CollapseDecision::Collapsed { target, .. }, Some(exp)) => {
                quantum_decided += 1;
                if target == exp { quantum_correct_decided += 1; } else { quantum_contradiction_errors += 1; }
            }
            (CollapseDecision::Collapsed { .. }, None) => { quantum_decided += 1; quantum_contradiction_errors += 1; }
            (CollapseDecision::Abstained { .. }, Some(_)) => { quantum_contradiction_errors += 1; }
            (CollapseDecision::Abstained { .. }, None) => {}
        }
    }

    let c_prec = if classical_decided > 0 { (classical_correct_decided as f64) / (classical_decided as f64) } else { 0.0 };
    let c_rec = (classical_correct_decided as f64) / (answerable_count as f64);
    let c_f1 = if (c_prec + c_rec) > 0.0 { 2.0 * c_prec * c_rec / (c_prec + c_rec) } else { 0.0 };

    let q_prec = if quantum_decided > 0 { (quantum_correct_decided as f64) / (quantum_decided as f64) } else { 0.0 };
    let q_rec = (quantum_correct_decided as f64) / (answerable_count as f64);
    let q_f1 = if (q_prec + q_rec) > 0.0 { 2.0 * q_prec * q_rec / (q_prec + q_rec) } else { 0.0 };

    let (c_mean, c_p99) = compute_mean_and_p99(&classical_latencies);
    let (q_mean, q_p99) = compute_mean_and_p99(&quantum_latencies);

    PolysemyAblationMetrics {
        total_cases,
        classical_resolved_accuracy_pct: c_prec * 100.0,
        classical_coverage_pct: (classical_decided as f64 / answerable_count as f64) * 100.0,
        classical_contradiction_error_rate_pct: (classical_contradiction_errors as f64 / total_cases as f64) * 100.0,
        classical_f1: c_f1,
        classical_mean_latency_us: c_mean,
        classical_p99_latency_us: c_p99,
        quantum_resolved_accuracy_pct: q_prec * 100.0,
        quantum_coverage_pct: (quantum_decided as f64 / answerable_count as f64) * 100.0,
        quantum_contradiction_error_rate_pct: (quantum_contradiction_errors as f64 / total_cases as f64) * 100.0,
        quantum_f1: q_f1,
        quantum_mean_latency_us: q_mean,
        quantum_p99_latency_us: q_p99,
    }
}

