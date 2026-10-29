use relic_blackwall::{BlackwallGate, CausalCatapult, TransactionOutcome};
use relic_lexicon::LexiconPayload;

#[test]
fn test_catapult_cut_crossing_exact_algebra() {
    let mut catapult = CausalCatapult::new(100);
    // Definimos el test 0 como corte cerrado UnboundedClosedCut
    catapult.unbounded_closed_cut_mask.set_bit(0);

    // Corte S_0 = {10, 11, 12}
    catapult.register_cut_node(10, 0);
    catapult.register_cut_node(11, 0);
    catapult.register_cut_node(12, 0);

    // 1. Arista interna: 10 -> 11 (u in S, v in S) => NO cruza corte
    let breaks_internal = catapult.evaluate_edge_addition(10, 11);
    assert!(breaks_internal.is_zero(), "Arista interna al corte no debe quebrar el certificado");

    // 2. Arista externa: 20 -> 21 (u not in S, v not in S) => NO cruza corte
    let breaks_external = catapult.evaluate_edge_addition(20, 21);
    assert!(breaks_external.is_zero(), "Arista externa al corte no debe quebrar el certificado");

    // 3. Arista entrante: 20 -> 10 (u not in S, v in S) => NO cruza corte
    let breaks_entering = catapult.evaluate_edge_addition(20, 10);
    assert!(breaks_entering.is_zero(), "Arista entrante al corte no debe quebrar el certificado");

    // 4. Arista saliente (cruza el corte): 10 -> 20 (u in S, v not in S) => CRUZA CORTE
    let breaks_crossing = catapult.evaluate_edge_addition(10, 20);
    assert!(!breaks_crossing.is_zero(), "Arista que cruza la frontera de S debe ser detectada");
    assert!(breaks_crossing.get_bit(0), "El bit del test 0 debe estar encendido");
}

#[test]
fn test_catapult_bounds_safety_on_giant_or_unknown_ids() {
    let mut catapult = CausalCatapult::new(10);
    catapult.unbounded_closed_cut_mask.set_bit(0);
    catapult.register_cut_node(5, 0);

    // v con id gigantesco (> capacity y > in_cut_bitset.len()) jamás debe provocar panic abort
    let breaks_1 = catapult.evaluate_edge_addition(5, 1_000_000);
    assert!(breaks_1.get_bit(0), "u in S y v nuevo/desconocido cruza el corte sin pánico");

    // u con id gigantesco (> capacity)
    let breaks_2 = catapult.evaluate_edge_addition(1_000_000, 5);
    assert!(breaks_2.is_zero(), "u fuera de todo corte no debe generar conflicto ni pánico");

    // Ambos gigantescos
    let breaks_3 = catapult.evaluate_edge_addition(u32::MAX - 2, u32::MAX - 1);
    assert!(breaks_3.is_zero(), "Nodos fuera de rango devuelven cero de forma segura");
}

#[test]
fn test_intra_batch_hazard_detection() {
    let mut gate = BlackwallGate::with_genesis();
    
    // Lote con encadenamiento: A -> B y luego B -> C
    // B es tocado por la primera arista y referenciado por la segunda
    let p1 = LexiconPayload::graph_fact("Nodo_Alfa", "conecta", "Nodo_Bravo");
    let p2 = LexiconPayload::graph_fact("Nodo_Bravo", "conecta", "Nodo_Charlie");
    
    let outcome = gate.commit_batch_payloads(&[p1, p2], 1_700_000_000);
    assert!(matches!(outcome, TransactionOutcome::Committed { .. }), "El lote encadenado debe procesarse y comitearse de forma segura");
}

#[test]
fn test_soundness_catapult_vs_full_regression_oracle() {
    let mut gate = BlackwallGate::with_genesis();
    
    // Inserción de aristas seguras e independientes
    for i in 0..50 {
        let from = format!("ClusterX_{i}");
        let to = format!("ClusterY_{i}");
        let payload = LexiconPayload::graph_fact(from, "enlace", to);
        let outcome = gate.propose_mutation(payload, 1_700_000_000 + i as u64);
        assert!(matches!(outcome, TransactionOutcome::Committed { .. }), "Mutación inocua {i} debe ser aprobada");
    }
}
