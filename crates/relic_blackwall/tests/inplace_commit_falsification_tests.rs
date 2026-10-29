//! Pruebas obligatorias de validación in-place con compensación inversa para BlackwallGate.
//! Cubre: igualdad profunda tras rechazo, equivalencia con rebuild_active_view y secuencias mixtas largas.
//! Menos de 180 líneas.

use relic_blackwall::{BlackwallGate, TransactionOutcome};
use relic_core::ArithmeticOp;
use relic_lexicon::{EngramStore, LexiconPayload, LexiconView};

fn assert_view_deep_equal(a: &LexiconView, b: &LexiconView) {
    assert_eq!(a.current_version, b.current_version);
    assert_eq!(a.head_hash, b.head_hash);
    assert_eq!(a.active_facts, b.active_facts);
    assert_eq!(a.op_aliases, b.op_aliases);
    assert_eq!(a.op_alias_weights, b.op_alias_weights);
    assert_eq!(a.phrase_macros, b.phrase_macros);
    assert_eq!(a.entity_aliases, b.entity_aliases);
    assert_eq!(a.episodic.total_node_count(), b.episodic.total_node_count());
    assert_eq!(a.episodic.edge_count(), b.episodic.edge_count());
}

#[test]
fn test_deep_equality_after_oracle_and_regression_rejection() {
    let mut gate = BlackwallGate::with_genesis();
    gate.propose_mutation(LexiconPayload::graph_fact("A", "conecta", "B"), 100);
    let view_before = gate.active_view().clone();

    // 1. Rechazo por Oráculo (auto-bucle prohibido)
    let res_oracle = gate.propose_mutation(LexiconPayload::graph_fact("SelfNode", "bucle", "SelfNode"), 200);
    assert!(matches!(res_oracle, TransactionOutcome::RejectedByOracle { .. }));
    assert_view_deep_equal(gate.active_view(), &view_before);

    // 2. Rechazo por Oráculo (palabra reservada del núcleo)
    let res_res = gate.propose_mutation(LexiconPayload::op_alias("suma", ArithmeticOp::Multiply), 300);
    assert!(matches!(res_res, TransactionOutcome::RejectedByOracle { .. }));
    assert_view_deep_equal(gate.active_view(), &view_before);

    // 3. Rechazo por Regresión (colisión de alias que rompe test canónico)
    let res_reg = gate.propose_mutation(LexiconPayload::entity_alias("Bogotá", "Invalido"), 400);
    assert!(matches!(res_reg, TransactionOutcome::RejectedByRegression { .. }));
    assert_view_deep_equal(gate.active_view(), &view_before);
}

#[test]
fn test_equivalence_with_rebuild_active_view_after_acceptance() {
    let mut gate = BlackwallGate::with_genesis();
    for i in 0..20 {
        let u = format!("N_{i}");
        let v = format!("N_{}", i + 1);
        let res = gate.propose_mutation(LexiconPayload::graph_fact(u, "step", v), 1000 + i);
        assert!(matches!(res, TransactionOutcome::Committed { .. }));
    }
    // Refuerzo
    let res_reinf = gate.reinforce_edge("N_0", "N_1", 1.5, 2000);
    assert!(matches!(res_reinf, TransactionOutcome::Committed { .. }));

    // Reconstrucción desde el log completo
    let rebuilt = EngramStore::rebuild_active_view(gate.log_entries()).unwrap();
    assert_view_deep_equal(gate.active_view(), &rebuilt);
}

#[test]
fn test_long_mixed_sequence_acceptances_and_rejections() {
    let mut gate = BlackwallGate::with_genesis();
    let mut accepted_count = 0;
    let mut rejected_count = 0;

    for i in 0..100 {
        let is_valid = i % 2 == 0;
        let payload = if is_valid {
            LexiconPayload::graph_fact(format!("Src_{i}"), "rel", format!("Dst_{i}"))
        } else if i % 4 == 1 {
            // Oráculo: auto-bucle
            LexiconPayload::graph_fact("C_err", "rel", "C_err")
        } else {
            // Oráculo: token reservado
            LexiconPayload::op_alias("más", ArithmeticOp::Divide)
        };

        let view_pre = gate.active_view().clone();
        let out = gate.propose_mutation(payload, 5000 + i as u64);
        if is_valid {
            assert!(matches!(out, TransactionOutcome::Committed { .. }));
            accepted_count += 1;
        } else {
            assert!(matches!(out, TransactionOutcome::RejectedByOracle { .. } | TransactionOutcome::RejectedByRegression { .. }));
            rejected_count += 1;
            // Estado debe ser idéntico al previo
            assert_view_deep_equal(gate.active_view(), &view_pre);
        }
    }

    assert_eq!(accepted_count, 50);
    assert_eq!(rejected_count, 50);
    // El estado final debe ser idéntico a la reconstrucción completa
    let rebuilt = EngramStore::rebuild_active_view(gate.log_entries()).unwrap();
    assert_view_deep_equal(gate.active_view(), &rebuilt);
}

#[test]
fn test_panic_safety_with_undo_guard() {
    use relic_lexicon::InverseOp;
    struct PanicGuard<'a>(&'a mut BlackwallGate, Option<InverseOp>);
    impl<'a> Drop for PanicGuard<'a> {
        fn drop(&mut self) {
            if let Some(op) = self.1.take() {
                self.0.engram_store_mut().active_view_mut().apply_inverse_op(op);
            }
        }
    }

    let mut gate = BlackwallGate::with_genesis();
    gate.propose_mutation(LexiconPayload::graph_fact("Base_A", "rel", "Base_B"), 100);
    let view_before = gate.active_view().clone();

    // Simulamos un pánico durante la ejecución bajo catch_unwind
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let payload = LexiconPayload::graph_fact("Transient_X", "rel", "Transient_Y");
        let undo = gate.engram_store_mut().active_view_mut().apply_payload_recording_undo(&payload);
        let _guard = PanicGuard(&mut gate, Some(undo));
        panic!("Falla catastrófica simulada");
    }));
    assert!(res.is_err(), "Se esperaba que capturara el pánico");
    // El estado tras el pánico debe ser idéntico al previo
    assert_view_deep_equal(gate.active_view(), &view_before);
}
