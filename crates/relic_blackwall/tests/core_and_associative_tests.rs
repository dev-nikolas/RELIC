//! Suite de pruebas de núcleo aritmético, activación propagada, consolidación y diff (`core_and_associative_tests.rs`). Menos de 180 líneas.

use relic_blackwall::{BlackwallGate, RegressionCase, TransactionOutcome};
use relic_cli::DispatchIntent;
use relic_core::{ActivationStatus, ArithmeticOp};
use relic_lexicon::{to_dot_diff, to_mermaid_diff, LexiconPayload};

#[test]
fn test_1_human_everyday_arithmetic_with_real_rpn_trace() {
    let mut gate = BlackwallGate::with_genesis();

    let (intent_half, ans_half) = gate.evaluate_query("¿cuánto es la mitad de 50?");
    assert_eq!(ans_half.status, "VALID");
    assert!(ans_half.text.contains("50 / 2 = 25"));
    if let DispatchIntent::Arithmetic(plan) = intent_half {
        assert_eq!(plan.reductions.len(), 1);
        assert_eq!((plan.reductions[0].left, plan.reductions[0].op, plan.reductions[0].right, plan.reductions[0].result), (50.0, ArithmeticOp::Divide, 2.0, 25.0));
    } else {
        panic!("Se esperaba DispatchIntent::Arithmetic");
    }

    let (intent_triple, ans_triple) = gate.evaluate_query("calcula el triple de (10 + 5)");
    assert_eq!(ans_triple.status, "VALID");
    assert!(ans_triple.text.contains("10 + 5 = 15; 15 * 3 = 45"));
    if let DispatchIntent::Arithmetic(plan) = intent_triple {
        assert_eq!((plan.reductions.len(), plan.reductions[0].result, plan.reductions[1].result), (2, 15.0, 45.0));
    }

    let (intent_pct, ans_pct) = gate.evaluate_query("¿cuánto es 20% de 150?");
    assert_eq!(ans_pct.status, "VALID");
    assert!(ans_pct.text.contains("20 / 100 = 0.20; 0.20 * 150 = 30"));
    if let DispatchIntent::Arithmetic(plan) = intent_pct {
        assert_eq!((plan.execute().unwrap(), plan.reductions.len()), (30.0, 2));
    }

    let (_, ans_var_pct) = gate.evaluate_query("200 más el 15%");
    assert_eq!(ans_var_pct.status, "VALID");
    assert!(ans_var_pct.text.contains("15 / 100 = 0.15; 0.15 * 200 = 30; 200 + 30 = 230"));

    let (_, ans_rep_exact) = gate.evaluate_query("repartir 20 entre 4");
    assert!(ans_rep_exact.status == "VALID" && ans_rep_exact.text.contains("20 / 4 = 5") && ans_rep_exact.text.contains("reparto exacto: cociente 5, resto 0"));

    let (_, ans_rep_rem) = gate.evaluate_query("repartir 10 entre 4");
    assert!(ans_rep_rem.status == "VALID" && ans_rep_rem.text.contains("10 / 4 = 2.50") && ans_rep_rem.text.contains("reparto inexacto: cociente entero 2, resto 2"));

    let commit_jargon = gate.propose_mutation(LexiconPayload::op_alias("embolatan", ArithmeticOp::Subtract), 1_700_100_001);
    assert!(matches!(commit_jargon, TransactionOutcome::Committed { .. }));

    let (intent_combo, ans_combo) = gate.evaluate_query("Si gano el doble de 100 y me embolatan el 10% de 200");
    assert_eq!(ans_combo.status, "VALID");
    assert!(ans_combo.text.contains("100 * 2 = 200; 10 / 100 = 0.10; 0.10 * 200 = 20; 200 - 20 = 180"));
    if let DispatchIntent::Arithmetic(plan) = intent_combo {
        assert_eq!(plan.execute().unwrap(), 180.0);
        assert_eq!(plan.reductions.len(), 4);
    } else {
        panic!("Se esperaba DispatchIntent::Arithmetic");
    }
}

#[test]
fn test_2_spreading_activation_and_entropy_abstention() {
    let mut gate = BlackwallGate::with_genesis();
    let res_symmetric = gate.propagate_activation(&["Madrid"], 0.8, 1, 1.5);
    assert_eq!(res_symmetric.steps_executed, 1);
    assert!((res_symmetric.shannon_entropy_bits - 1.0).abs() < 1e-6);
    match res_symmetric.status {
        ActivationStatus::Ambiguous { ref tied_candidates, entropy_bits, ref reason } => {
            assert!(tied_candidates.contains(&"Barcelona".to_string()) && tied_candidates.contains(&"Toledo".to_string()));
            assert!((entropy_bits - 1.0).abs() < 1e-6 && reason.contains("Abstención por empate"));
        }
        other => panic!("Se esperaba Ambiguous, obtuvo: {other:?}"),
    }

    assert!(matches!(gate.reinforce_edge("Madrid", "Barcelona", 2.0, 1_700_100_010), TransactionOutcome::Committed { .. }));
    let res_reinforced = gate.propagate_activation(&["Madrid"], 0.8, 1, 1.5);
    match res_reinforced.status {
        ActivationStatus::Valid { ref top_concept, top_energy, probability } => {
            assert_eq!(top_concept, "Barcelona");
            assert!((top_energy - 2.4).abs() < 1e-6 && (probability - 0.75).abs() < 1e-6);
        }
        other => panic!("Se esperaba Valid, obtuvo: {other:?}"),
    }
    assert!(res_reinforced.shannon_entropy_bits < 0.82 && res_reinforced.shannon_entropy_bits > 0.80);
}

#[test]
fn test_3_memory_consolidation_sleep_with_blackwall_fidelity_guarantee() {
    let mut gate = BlackwallGate::with_genesis();
    assert!(matches!(gate.propose_mutation(LexiconPayload::graph_fact_weighted("rumor_temporal_1", "asociado_a", "ruido_A", 0.2), 1_700_200_001), TransactionOutcome::Committed { .. }));
    assert!(matches!(gate.propose_mutation(LexiconPayload::graph_fact_weighted("rumor_temporal_2", "asociado_a", "ruido_B", 0.15), 1_700_200_002), TransactionOutcome::Committed { .. }));
    assert!(gate.active_view().episodic.contains_node("rumor_temporal_1"));

    match gate.consolidate_memory(0.5, 1.0, 1_700_200_010) {
        TransactionOutcome::Consolidated { pruned_count, fidelity_score, .. } => {
            assert_eq!(pruned_count, 2);
            assert!((fidelity_score - 1.0).abs() < 1e-9);
        }
        other => panic!("Se esperaba Consolidated, obtuvo: {other:?}"),
    }
    assert!(!gate.active_view().episodic.contains_node("rumor_temporal_1"));
    assert_eq!(gate.log_entries().len(), 16);

    let v_before_bad_sleep = gate.current_version();
    match gate.consolidate_memory(2.0, 1.0, 1_700_200_020) {
        TransactionOutcome::RejectedByFidelity { fidelity_score, required_fidelity, ref degraded_cases } => {
            assert!(fidelity_score < 1.0 && (required_fidelity - 1.0).abs() < 1e-9);
            assert!(degraded_cases.contains(&"reg_graph_spatial_07".to_string()));
        }
        other => panic!("Debió abortar por fidelidad, obtuvo: {other:?}"),
    }
    assert_eq!(gate.current_version(), v_before_bad_sleep);

    gate.propose_mutation(LexiconPayload::graph_fact_weighted("Laboratorio", "conectado_a", " Bóveda", 0.8), 1_700_200_030);
    gate.add_regression_case(RegressionCase {
        id: "reg_custom_lab_vault".to_string(),
        query: "¿Es posible ir de Laboratorio a Bóveda?".to_string(),
        expected_substring: "Laboratorio -> Bóveda".to_string(),
        expected_route: "deterministic_graph".to_string(),
        expected_status: "VALID".to_string(),
    });
    match gate.consolidate_memory(0.9, 1.0, 1_700_200_040) {
        TransactionOutcome::RejectedByFidelity { fidelity_score, ref degraded_cases, .. } => {
            assert!(fidelity_score < 1.0 && degraded_cases == &vec!["reg_custom_lab_vault".to_string()]);
        }
        other => panic!("Debió bloquearse por caso custom, obtuvo: {other:?}"),
    }
}

#[test]
fn test_4_structured_version_diff_and_mermaid_dot_visualizers() {
    let mut gate = BlackwallGate::with_genesis();
    assert!(matches!(gate.propose_mutation(LexiconPayload::op_alias("coronar", ArithmeticOp::Add), 1_700_300_001), TransactionOutcome::Committed { version: 14, .. }));
    assert!(matches!(gate.propose_mutation(LexiconPayload::graph_fact("Quito", "conectado_a", "Guayaquil"), 1_700_300_002), TransactionOutcome::Committed { version: 15, .. }));
    assert!(matches!(gate.rollback_to(14, "Revertir ruta experimental Quito-Guayaquil", 1_700_300_003), TransactionOutcome::RolledBack { version: 16, target_version: 14, .. }));
    assert!(matches!(gate.propose_mutation(LexiconPayload::graph_fact_weighted("Bogotá", "conectado_a", "Medellín", 1.5), 1_700_300_004), TransactionOutcome::Committed { version: 17, .. }));

    let diff_13_16 = gate.diff_versions(13, 16).expect("Diff v13..v16 válido");
    assert_eq!((diff_13_16.v_from, diff_13_16.v_to, diff_13_16.added_aliases.len(), diff_13_16.reverted_entries.len()), (13, 16, 1, 1));
    assert_eq!(diff_13_16.added_aliases[0].0, "coronar");
    assert_eq!((diff_13_16.reverted_entries[0].version, diff_13_16.reverted_entries[0].reverted_by_version), (15, 16));
    assert!(diff_13_16.removed_facts.contains(&("Quito".to_string(), "conectado_a".to_string(), "Guayaquil".to_string())));

    let diff_13_17 = gate.diff_versions(13, 17).expect("Diff v13..v17 válido");
    assert_eq!(diff_13_17.added_facts.len(), 1);
    let mermaid = to_mermaid_diff(&diff_13_17);
    assert!(mermaid.starts_with("flowchart LR") && mermaid.contains("\"Bogotá\" == \"+ conectado_a (w=1.50)\" ==> \"Medellín\""));
    assert!(mermaid.contains("\"Quito\" -. \"REVERTED: conectado_a\" .-> \"Guayaquil\"") && mermaid.contains("SCAR v15 (rev by v16)"));

    let dot = to_dot_diff(&diff_13_17);
    assert!(dot.starts_with("digraph EngramDiff_v13_to_v17 {") && dot.contains("\"Bogotá\" -> \"Medellín\"") && dot.contains("REVERTED: conectado_a"));
}
