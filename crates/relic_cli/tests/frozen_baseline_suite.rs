//! # Suite de Verificación de Baseline Congelado (Mejora 1) — Menos de 180 líneas.

use relic_core::episodic_memory::EpisodicMemory;
use relic_core::reasoning_engine::ReasoningEngine;
use relic_core::rir::{ArithmeticStep, RirArithmeticOp, RirDocumentV1, ValidityStatus};
use relic_cli::dispatch::{parse_dispatch_intent, parse_inventory_problem, DispatchIntent};
use relic_cli::math::parse_math_expression;

#[test]
fn test_frozen_math_precedence_and_associativity() {
    let plan = parse_math_expression("8 - 3 * 2").expect("Debe compilar a árbol AST");
    assert_eq!(plan.execute().unwrap(), 2.0);
    let plan2 = parse_math_expression("20 - 4 / 2 + 3 * 2").unwrap();
    assert_eq!(plan2.execute().unwrap(), 24.0);
    let plan3 = parse_math_expression("(8 - 3) * 2").unwrap();
    assert_eq!(plan3.execute().unwrap(), 10.0);
}

#[test]
fn test_frozen_math_decimals_negatives_and_div_zero() {
    let plan_dec = parse_math_expression("12.5 + 2.5 * -2").unwrap();
    assert_eq!(plan_dec.execute().unwrap(), 7.5);
    let plan_div_zero = parse_math_expression("10 / 0");
    assert!(plan_div_zero.is_none() || plan_div_zero.unwrap().execute().is_err());
}

#[test]
fn test_frozen_inventory_composition() {
    let plan = parse_inventory_problem("Tenía 15 libros, perdí 5 y recibí 2 más").unwrap();
    assert_eq!(plan.initial, 15.0);
    assert_eq!(plan.unit, Some("libros".to_string()));
    assert_eq!(plan.execute().unwrap(), 12.0);
    assert_eq!(plan.format_result(12.0), "12 libros");
}

#[test]
fn test_inventory_linguistic_variations_and_transfers() {
    let p1 = parse_inventory_problem("Tenía 10 manzanas, comí 2 y le di 3 a María").unwrap();
    assert_eq!(p1.execute().unwrap(), 5.0);
    assert_eq!(p1.format_result(5.0), "5 manzanas");

    let p2 = parse_inventory_problem("Tenía 12 panes y me zampé 3").unwrap();
    assert_eq!(p2.execute().unwrap(), 9.0);

    let p3 = parse_inventory_problem("Tenía 20 euros, gasté 5, me encontré 10 y le doné 4 a un refugio").unwrap();
    assert_eq!(p3.execute().unwrap(), 21.0);

    let p4 = parse_inventory_problem("Tenía 10 manzanas y observé 2");
    assert!(p4.is_none(), "Un verbo no cuantitativo como 'observé' no debe generar operación");
}

fn setup_frozen_test_graph() -> EpisodicMemory {
    let mut mem = EpisodicMemory::new();
    for (u, v) in [
        ("Madrid", "Barcelona"), ("Barcelona", "París"), ("Madrid", "Valencia"),
        ("Valencia", "Lyon"), ("Lyon", "París"), ("Madrid", "Toledo"), ("A", "B"), ("B", "C"),
    ] {
        mem.insert_edge(u, v);
    }
    mem
}

#[test]
fn test_frozen_bfs_unconstrained_shortest_path() {
    let mut mem = setup_frozen_test_graph();
    let res = ReasoningEngine::solve_reachability_bfs(&mut mem, "Madrid", "París", 10);
    assert!(res.is_reachable);
    assert_eq!(res.path, vec!["Madrid", "Barcelona", "París"]);
    assert_eq!(res.path_length, 2);
}

#[test]
fn test_frozen_bfs_avoid_constraint_finds_alternative() {
    let mut mem = setup_frozen_test_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "París", 10, &["Barcelona"]);
    assert!(res.is_reachable);
    assert_eq!(res.path, vec!["Madrid", "Valencia", "Lyon", "París"]);
    assert!(!res.path.contains(&"Barcelona".to_string()));
}

#[test]
fn test_frozen_bfs_avoid_constraint_blocks_all_routes() {
    let mut mem = setup_frozen_test_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "París", 10, &["Barcelona", "Valencia"]);
    assert!(!res.is_reachable);
    assert!(res.path.is_empty());
}

#[test]
fn test_frozen_bfs_avoid_origin_or_target_fails_safely() {
    let mut mem = setup_frozen_test_graph();
    assert!(!ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "París", 10, &["Madrid"]).is_reachable);
    assert!(!ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "París", 10, &["París"]).is_reachable);
}

#[test]
fn test_frozen_epistemic_dispatch_rejections() {
    let intent_free = parse_dispatch_intent("Explícame la teoría de cuerdas en tres párrafos");
    assert!(matches!(intent_free, DispatchIntent::FreeLanguage { .. }));
    let intent_unrec = parse_dispatch_intent("Tenía 4 manzanas y hallé en el almacén 2");
    assert!(matches!(intent_unrec, DispatchIntent::Incomplete { .. }));
}

#[test]
fn test_temporal_graph_transitive_reasoning() {
    let q = "Si Pedro es más alto que Juan y Juan es más alto que Diego, ¿quién es el más alto?";
    if let DispatchIntent::RelationalReasoning(spec) = parse_dispatch_intent(q) {
        assert_eq!(spec.graph.find_maximal("altura_mayor"), Some("Pedro".to_string()));
        assert!(spec.graph.is_transitively_related("Pedro", "Diego", "altura_mayor"));
        assert!(!spec.graph.is_transitively_related("Diego", "Pedro", "altura_mayor"));
    } else {
        panic!("Debe ser clasificado como RelationalReasoning");
    }
}

#[test]
fn test_frozen_rir_document_semantic_validation() {
    let doc_valid = RirDocumentV1::new_inventory(
        Some("unidades".to_string()), 10.0, vec![ArithmeticStep { operator: RirArithmeticOp::Add, operand: 5.0 }],
    );
    assert!(doc_valid.validate_semantic().is_ok());
    let doc_rejected = RirDocumentV1::new_rejected(ValidityStatus::Unsupported, "Consulta no admitida", vec![]);
    assert!(doc_rejected.validate_semantic().is_err());
}
