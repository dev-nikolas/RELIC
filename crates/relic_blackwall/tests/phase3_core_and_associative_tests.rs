//! Suite de pruebas obligatoria de Fase 3 (`phase3_core_and_associative_tests.rs`).
//! Verifica con `assert!` estrictos:
//! 1. Aritmética cotidiana humana con traza RPN exacta ("la mitad de", "el triple de", "20% de 150",
//!    combinaciones con paréntesis/jerga aprendida y reparto con resto).
//! 2. Memoria asociativa con Activación Propagada (`propagate_activation`), refuerzo y abstención por entropía/empate.
//! 3. Consolidación ("Sueño") con métrica de Fidelidad $F \in [0.0, 1.0]$ custodiada por el Blackwall.
//! 4. Diff visual estructurado entre versiones ($v_{13} \leftrightarrow v_{16}$) y exportadores Mermaid/DOT.

use relic_blackwall::{BlackwallGate, RegressionCase, TransactionOutcome};
use relic_cli::DispatchIntent;
use relic_core::{ActivationStatus, ArithmeticOp};
use relic_lexicon::{to_dot_diff, to_mermaid_diff, LexiconPayload};

#[test]
fn test_1_human_everyday_arithmetic_with_real_rpn_trace() {
    let mut gate = BlackwallGate::with_genesis();

    // 1. "la mitad de 50" -> reducción 50 / 2 = 25
    let (intent_half, ans_half) = gate.evaluate_query("¿cuánto es la mitad de 50?");
    assert_eq!(ans_half.status, "VALID");
    assert_eq!(ans_half.route, "deterministic_arithmetic");
    assert!(
        ans_half.text.contains("50 / 2 = 25"),
        "Debe incluir la reducción exacta '50 / 2 = 25', obtuvo: {}",
        ans_half.text
    );
    if let DispatchIntent::Arithmetic(plan) = intent_half {
        assert_eq!(plan.reductions.len(), 1);
        assert_eq!(plan.reductions[0].left, 50.0);
        assert_eq!(plan.reductions[0].op, ArithmeticOp::Divide);
        assert_eq!(plan.reductions[0].right, 2.0);
        assert_eq!(plan.reductions[0].result, 25.0);
    } else {
        panic!("Se esperaba DispatchIntent::Arithmetic");
    }

    // 2. "el triple de (10 + 5)" -> Paso 1: 10 + 5 = 15; Paso 2: 15 * 3 = 45
    let (intent_triple, ans_triple) = gate.evaluate_query("calcula el triple de (10 + 5)");
    assert_eq!(ans_triple.status, "VALID");
    assert!(
        ans_triple.text.contains("10 + 5 = 15; 15 * 3 = 45"),
        "Debe contener las dos reducciones RPN exactas, obtuvo: {}",
        ans_triple.text
    );
    if let DispatchIntent::Arithmetic(plan) = intent_triple {
        assert_eq!(plan.reductions.len(), 2);
        assert_eq!(plan.reductions[0].result, 15.0);
        assert_eq!(plan.reductions[1].result, 45.0);
    }

    // 3. "20% de 150" -> Paso 1: 20 / 100 = 0.20; Paso 2: 0.20 * 150 = 30
    let (intent_pct, ans_pct) = gate.evaluate_query("¿cuánto es 20% de 150?");
    assert_eq!(ans_pct.status, "VALID");
    assert!(
        ans_pct.text.contains("20 / 100 = 0.20; 0.20 * 150 = 30"),
        "Debe exponer el paso porcentual explícito en la traza, obtuvo: {}",
        ans_pct.text
    );
    if let DispatchIntent::Arithmetic(plan) = intent_pct {
        assert_eq!(plan.execute().unwrap(), 30.0);
        assert_eq!(plan.reductions.len(), 2);
    }

    // 4. Variación porcentual: "200 más el 15%" -> 200 + 30 = 230
    let (_, ans_var_pct) = gate.evaluate_query("200 más el 15%");
    assert_eq!(ans_var_pct.status, "VALID");
    assert!(
        ans_var_pct.text.contains("15 / 100 = 0.15; 0.15 * 200 = 30; 200 + 30 = 230"),
        "Variación porcentual debe mostrar todos los pasos RPN, obtuvo: {}",
        ans_var_pct.text
    );

    // 5. Reparto exacto e inexacto con detección de resto
    let (_, ans_rep_exact) = gate.evaluate_query("repartir 20 entre 4");
    assert_eq!(ans_rep_exact.status, "VALID");
    assert!(ans_rep_exact.text.contains("20 / 4 = 5"));
    assert!(ans_rep_exact.text.contains("reparto exacto: cociente 5, resto 0"));

    let (_, ans_rep_rem) = gate.evaluate_query("repartir 10 entre 4");
    assert_eq!(ans_rep_rem.status, "VALID");
    assert!(ans_rep_rem.text.contains("10 / 4 = 2.50"));
    assert!(ans_rep_rem.text.contains("reparto inexacto: cociente entero 2, resto 2"));

    // 6. Combinación con jerga aprendida en el Blackwall:
    // "Si gano el doble de 100 y me embolatan el 10% de 200" -> 180
    let commit_jargon = gate.propose_mutation(
        LexiconPayload::op_alias("embolatan", ArithmeticOp::Subtract),
        1_700_100_001,
    );
    assert!(matches!(commit_jargon, TransactionOutcome::Committed { .. }));

    let (intent_combo, ans_combo) =
        gate.evaluate_query("Si gano el doble de 100 y me embolatan el 10% de 200");
    assert_eq!(ans_combo.status, "VALID");
    assert_eq!(ans_combo.route, "deterministic_arithmetic");
    assert!(
        ans_combo.text.contains("100 * 2 = 200; 10 / 100 = 0.10; 0.10 * 200 = 20; 200 - 20 = 180"),
        "La traza combinada debe reflejar cada reducción exacta hasta 180, obtuvo: {}",
        ans_combo.text
    );
    if let DispatchIntent::Arithmetic(plan) = intent_combo {
        assert_eq!(plan.execute().unwrap(), 180.0);
        assert_eq!(plan.reductions.len(), 4);
        assert_eq!(plan.reductions[0].result, 200.0);
        assert_eq!(plan.reductions[1].result, 0.10);
        assert_eq!(plan.reductions[2].result, 20.0);
        assert_eq!(plan.reductions[3].result, 180.0);
    } else {
        panic!("Se esperaba DispatchIntent::Arithmetic");
    }
}

#[test]
fn test_2_spreading_activation_and_entropy_abstention() {
    let mut gate = BlackwallGate::with_genesis();

    // En la semilla génesis, "Madrid" tiene dos vecinos directos con igual peso inicial (1.0):
    // Madrid -> Barcelona (w=1.0) y Madrid -> Toledo (w=1.0).
    // Con 1 paso (K=1), ambos reciben exactamente gamma * 1.0 = 0.8 de energía (empate 50% / 50%, H = 1.0 bit).
    let res_symmetric = gate.propagate_activation(&["Madrid"], 0.8, 1, 1.5);
    assert_eq!(res_symmetric.steps_executed, 1);
    assert!((res_symmetric.shannon_entropy_bits - 1.0).abs() < 1e-6);
    match res_symmetric.status {
        ActivationStatus::Ambiguous {
            ref tied_candidates,
            entropy_bits,
            ref reason,
        } => {
            assert!(tied_candidates.contains(&"Barcelona".to_string()));
            assert!(tied_candidates.contains(&"Toledo".to_string()));
            assert!((entropy_bits - 1.0).abs() < 1e-6);
            assert!(reason.contains("Abstención por empate"));
        }
        other => panic!("Ante dos rutas simétricas de igual peso debe abstenerse como Ambiguous, obtuvo: {other:?}"),
    }

    // Ahora reforzamos por uso la arista "Madrid -> Barcelona" (+2.0 -> nuevo peso = 3.0) a través del Blackwall
    let reinf = gate.reinforce_edge("Madrid", "Barcelona", 2.0, 1_700_100_010);
    assert!(matches!(reinf, TransactionOutcome::Committed { .. }));

    // Re-ejecutamos propagación desde "Madrid": ahora "Barcelona" domina claramente (energía 2.4 vs 0.8 de Toledo)
    let res_reinforced = gate.propagate_activation(&["Madrid"], 0.8, 1, 1.5);
    match res_reinforced.status {
        ActivationStatus::Valid {
            ref top_concept,
            top_energy,
            probability,
        } => {
            assert_eq!(top_concept, "Barcelona");
            assert!((top_energy - 2.4).abs() < 1e-6);
            assert!((probability - 0.75).abs() < 1e-6);
        }
        other => panic!("El camino reforzado debía ganar con ActivationStatus::Valid, obtuvo: {other:?}"),
    }
    // Entropía de (0.75, 0.25) es ~0.811278 bits (< 1.0 bit del caso simétrico)
    assert!(res_reinforced.shannon_entropy_bits < 0.82 && res_reinforced.shannon_entropy_bits > 0.80);
}

#[test]
fn test_3_memory_consolidation_sleep_with_blackwall_fidelity_guarantee() {
    let mut gate = BlackwallGate::with_genesis();

    // 1. Insertar dos hechos episódicos ruidosos/efímeros de bajo peso (weight = 0.2 < 0.5)
    let noise_1 = gate.propose_mutation(
        LexiconPayload::graph_fact_weighted("rumor_temporal_1", "asociado_a", "ruido_A", 0.2),
        1_700_200_001,
    );
    let noise_2 = gate.propose_mutation(
        LexiconPayload::graph_fact_weighted("rumor_temporal_2", "asociado_a", "ruido_B", 0.15),
        1_700_200_002,
    );
    assert!(matches!(noise_1, TransactionOutcome::Committed { .. }));
    assert!(matches!(noise_2, TransactionOutcome::Committed { .. }));
    assert!(gate.active_view().episodic.contains_node("rumor_temporal_1"));

    // 2. Ejecutar consolidación ("Sueño") con umbral min_weight = 0.5 y fidelidad exigida = 1.0:
    // Se podan las 2 aristas ruidosas; como ninguna consulta protegida depende de ellas, F = 1.0 y hace Commit.
    let sleep_ok = gate.consolidate_memory(0.5, 1.0, 1_700_200_010);
    match sleep_ok {
        TransactionOutcome::Consolidated {
            pruned_count,
            fidelity_score,
            ..
        } => {
            assert_eq!(pruned_count, 2);
            assert!((fidelity_score - 1.0).abs() < 1e-9);
        }
        other => panic!("La consolidación de ruido efímero debía aprobarse con F=1.0, obtuvo: {other:?}"),
    }
    // Los nodos ruidosos ya no están en la memoria proyectada activa, pero el log original es inmutable
    assert!(!gate.active_view().episodic.contains_node("rumor_temporal_1"));
    assert_eq!(gate.log_entries().len(), 16);

    // 3. Intentar consolidar con un umbral agresivo (min_weight = 2.0) que podaría aristas esenciales
    // de la suite de regresión (peso = 1.0, ej. Bogotá -> Lima, Marta -> estación):
    // La fidelidad cae (F < 1.0) y el Blackwall aborta automáticamente la consolidación.
    let v_before_bad_sleep = gate.current_version();
    let sleep_blocked = gate.consolidate_memory(2.0, 1.0, 1_700_200_020);
    match sleep_blocked {
        TransactionOutcome::RejectedByFidelity {
            fidelity_score,
            required_fidelity,
            ref degraded_cases,
        } => {
            assert!(fidelity_score < 1.0, "La fidelidad debía caer por debajo de 1.0: {fidelity_score}");
            assert!((required_fidelity - 1.0).abs() < 1e-9);
            assert!(degraded_cases.contains(&"reg_graph_spatial_07".to_string()));
        }
        other => panic!("El Blackwall debía abortar la consolidación destructiva, obtuvo: {other:?}"),
    }
    assert_eq!(gate.current_version(), v_before_bad_sleep);

    // Verificar también con un caso custom protegido
    gate.propose_mutation(
        LexiconPayload::graph_fact_weighted("Laboratorio", "conectado_a", " Bóveda", 0.8),
        1_700_200_030,
    );
    gate.add_regression_case(RegressionCase {
        id: "reg_custom_lab_vault".to_string(),
        query: "¿Es posible ir de Laboratorio a Bóveda?".to_string(),
        expected_substring: "Laboratorio -> Bóveda".to_string(),
        expected_route: "deterministic_graph".to_string(),
        expected_status: "VALID".to_string(),
    });
    let sleep_custom_blocked = gate.consolidate_memory(0.9, 1.0, 1_700_200_040);
    match sleep_custom_blocked {
        TransactionOutcome::RejectedByFidelity {
            fidelity_score,
            ref degraded_cases,
            ..
        } => {
            assert!(fidelity_score < 1.0);
            assert_eq!(degraded_cases, &vec!["reg_custom_lab_vault".to_string()]);
        }
        other => panic!("Debió bloquearse por degradar el caso protegido custom, obtuvo: {other:?}"),
    }
}

#[test]
fn test_4_structured_version_diff_and_mermaid_dot_visualizers() {
    let mut gate = BlackwallGate::with_genesis();
    assert_eq!(gate.current_version(), 13);

    // v14: aprender alias de operación "coronar" -> Add
    let c14 = gate.propose_mutation(
        LexiconPayload::op_alias("coronar", ArithmeticOp::Add),
        1_700_300_001,
    );
    assert!(matches!(c14, TransactionOutcome::Committed { version: 14, .. }));

    // v15: añadir hecho experimental "Quito -> Guayaquil"
    let c15 = gate.propose_mutation(
        LexiconPayload::graph_fact("Quito", "conectado_a", "Guayaquil"),
        1_700_300_002,
    );
    assert!(matches!(c15, TransactionOutcome::Committed { version: 15, .. }));

    // v16: rollback hacia v14 (revierte "Quito -> Guayaquil" dejando cicatriz en v16)
    let c16 = gate.rollback_to(14, "Revertir ruta experimental Quito-Guayaquil", 1_700_300_003);
    assert!(matches!(c16, TransactionOutcome::RolledBack { version: 16, target_version: 14, .. }));

    // v17: añadir hecho definitivo "Bogotá -> Medellín"
    let c17 = gate.propose_mutation(
        LexiconPayload::graph_fact_weighted("Bogotá", "conectado_a", "Medellín", 1.5),
        1_700_300_004,
    );
    assert!(matches!(c17, TransactionOutcome::Committed { version: 17, .. }));

    // Inspeccionar Diff entre v13 y v16 (y también v13 -> v17)
    let diff_13_16 = gate.diff_versions(13, 16).expect("Diff v13..v16 válido");
    assert_eq!(diff_13_16.v_from, 13);
    assert_eq!(diff_13_16.v_to, 16);
    assert_eq!(diff_13_16.added_aliases.len(), 1);
    assert_eq!(diff_13_16.added_aliases[0].0, "coronar");
    assert_eq!(diff_13_16.reverted_entries.len(), 1);
    assert_eq!(diff_13_16.reverted_entries[0].version, 15);
    assert_eq!(diff_13_16.reverted_entries[0].reverted_by_version, 16);
    assert!(diff_13_16.removed_facts.contains(&(
        "Quito".to_string(),
        "conectado_a".to_string(),
        "Guayaquil".to_string()
    )));

    let diff_13_17 = gate.diff_versions(13, 17).expect("Diff v13..v17 válido");
    assert_eq!(diff_13_17.added_facts.len(), 1);
    assert_eq!(diff_13_17.added_facts[0].0, "Bogotá");
    assert_eq!(diff_13_17.added_facts[0].2, "Medellín");

    let mermaid = to_mermaid_diff(&diff_13_17);
    assert!(mermaid.starts_with("flowchart LR"));
    assert!(mermaid.contains("\"Bogotá\" == \"+ conectado_a (w=1.50)\" ==> \"Medellín\""));
    assert!(mermaid.contains("\"Quito\" -. \"REVERTED: conectado_a\" .-> \"Guayaquil\""));
    assert!(mermaid.contains("SCAR v15 (rev by v16)"));
    assert!(mermaid.contains("stroke:#16a34a"));
    assert!(mermaid.contains("stroke:#dc2626"));

    let dot = to_dot_diff(&diff_13_17);
    assert!(dot.starts_with("digraph EngramDiff_v13_to_v17 {"));
    assert!(dot.contains("\"Bogotá\" -> \"Medellín\" [label=\"+ conectado_a (w=1.50)\", color=\"#16a34a\""));
    assert!(dot.contains("\"Quito\" -> \"Guayaquil\" [label=\"REVERTED: conectado_a\", color=\"#dc2626\", style=\"dashed\"]"));
}

