//! Suite de Pruebas de Sistemas Complejos: Dinámica de Sincronización de Kuramoto,
//! Auto-Calibración de Fase (Cero Ángulos Hardcodeados), Destilación en la Sombra
//! e Inmunidad Transaccional ante Cápsulas `.engram` Corruptas o Troyanas.
use std::f64::consts::PI;
use relic_blackwall::{BlackwallGate, TransactionOutcome};
use relic_core::{
    ArithmeticOp, CollapseDecision, KuramotoNetworkSpec, KuramotoOscillator, SynchronizationOutcome,
};
use relic_lexicon::{
    auto_calibrate_phases, ContextObservation, EngramStore, EntryStatus, LexiconPayload,
};

#[test]
fn test_1_kuramoto_spontaneous_synchronization_vs_incoherent_abstention() {
    let gate = BlackwallGate::with_genesis();

    // Caso A: Pistas de contexto consistentes (K_ij > 0) -> convergencia espontánea a r > 0.95
    let oscillators_coherent = vec![
        KuramotoOscillator::new(" Subtract_bajar_escalones ", -1.2, 0.0),
        KuramotoOscillator::new(" ctx_escalones ", -0.4, 0.0),
        KuramotoOscillator::new(" ctx_sotano ", 0.5, 0.0),
        KuramotoOscillator::new(" ctx_descenso ", 1.1, 0.0),
    ];
    let k_pos = vec![vec![2.5; 4]; 4];
    let spec_coherent = KuramotoNetworkSpec {
        oscillators: oscillators_coherent,
        coupling_matrix: k_pos,
        steps: 150,
        dt: 0.05,
        required_r: 0.95,
        alignment_cos_threshold: 0.85,
    };
    let out_coh = gate.synchronize_kuramoto(&spec_coherent);
    match out_coh {
        SynchronizationOutcome::Synchronized { order_parameter_r, synchronized_cluster, trace, .. } => {
            assert!(order_parameter_r > 0.95, "Se esperaba r > 0.95, obtuvo {order_parameter_r}");
            assert!(trace.initial_r < 0.75, "Debió partir de fases dispersas");
            assert_eq!(synchronized_cluster.len(), 4);
        }
        other => panic!("Esperaba Synchronized con r > 0.95, obtuvo: {other:?}"),
    }

    // Caso B: Pistas mutuamente contradictorias / frustradas (K_ij < 0) -> r < r* -> IncoherentAbstention
    let oscillators_frustrated = vec![
        KuramotoOscillator::new("op_add", 0.0, 0.0),
        KuramotoOscillator::new("op_sub", PI, 0.0),
        KuramotoOscillator::new("ctx_conflict_1", PI / 2.0, 0.0),
        KuramotoOscillator::new("ctx_conflict_2", -PI / 2.0, 0.0),
    ];
    let k_neg = vec![vec![-2.0; 4]; 4];
    let spec_frustrated = KuramotoNetworkSpec {
        oscillators: oscillators_frustrated,
        coupling_matrix: k_neg,
        steps: 150,
        dt: 0.05,
        required_r: 0.80,
        alignment_cos_threshold: 0.85,
    };
    let out_inc = gate.synchronize_kuramoto(&spec_frustrated);
    match out_inc {
        SynchronizationOutcome::IncoherentAbstention { order_parameter_r, required_r, .. } => {
            assert!(order_parameter_r < required_r, "El parámetro r ({order_parameter_r}) debió caer bajo r* ({required_r})");
            assert!(order_parameter_r < 0.25);
        }
        other => panic!("Esperaba IncoherentAbstention por frustración de fase, obtuvo: {other:?}"),
    }
}

#[test]
fn test_2_auto_phase_calibration_zero_hardcoded_angles() {
    let mut gate = BlackwallGate::with_genesis();

    // Alimentamos el calibrador únicamente con ejemplos crudos de uso en 2 contextos opuestos (cero ángulos manuales)
    let raw_observations = vec![
        ContextObservation::new("descargar", "batería", ArithmeticOp::Subtract),
        ContextObservation::new("descargar", "batería", ArithmeticOp::Subtract),
        ContextObservation::new("descargar", "archivos", ArithmeticOp::Add),
        ContextObservation::new("descargar", "archivos", ArithmeticOp::Add),
    ];

    let reports = auto_calibrate_phases(&raw_observations);
    assert_eq!(reports.len(), 1);
    let rep = &reports[0];
    assert_eq!(rep.ambiguous_word, "descargar");

    // Verificamos que sintetizó constructiva (0.0) y destructiva (PI) automáticamente
    let has_destructive_pi = rep.synthesized_payloads.iter().any(|p| {
        matches!(p, LexiconPayload::QuantumPolysemousAlias { phase_theta, context_word: Some(_), .. } if (*phase_theta - PI).abs() < 1e-9)
    });
    assert!(has_destructive_pi, "El auto-calibrador debe sintetizar antifase PI automáticamente");

    let outcome = gate.auto_calibrate_and_commit(&raw_observations, 2000);
    assert!(matches!(outcome, TransactionOutcome::Committed { .. }));

    // Resolución cuántica en ambos contextos con 100% de exactitud
    let dec_sub = gate.resolve_polysemous_quantum("descargar", &["batería"], 3.0, 0.0).unwrap();
    assert!(matches!(dec_sub, CollapseDecision::Collapsed { target: ArithmeticOp::Subtract, born_probability, .. } if born_probability > 0.99));

    let dec_add = gate.resolve_polysemous_quantum("descargar", &["archivos"], 3.0, 0.0).unwrap();
    assert!(matches!(dec_add, CollapseDecision::Collapsed { target: ArithmeticOp::Add, born_probability, .. } if born_probability > 0.99));

    // Y en consultas reales de inventario completas
    let (_, ans1) = gate.evaluate_query("Tengo 80 unidades y luego descargar 20 de batería");
    assert_eq!(ans1.status, "VALID");
    assert!(ans1.text.contains("80 unidades - 20 unidades") && ans1.text.contains("60 unidades"), "Obtuvo: {}", ans1.text);

    let (_, ans2) = gate.evaluate_query("Tengo 80 unidades y luego descargar 20 de archivos");
    assert_eq!(ans2.status, "VALID");
    assert!(ans2.text.contains("80 unidades + 20 unidades") && ans2.text.contains("100 unidades"), "Obtuvo: {}", ans2.text);
}

#[test]
fn test_3_shadow_distillation_cycle_from_incomplete_to_learned_and_protected() {
    let mut gate = BlackwallGate::with_genesis();
    let slang_query = "Tengo 50 monedas y me desfalcan 15";

    // 1. Antes de asimilar: falla con INCOMPLETE
    let (_, ans_before) = gate.evaluate_query(slang_query);
    assert_eq!(ans_before.status, "INCOMPLETE");

    // 2. Se invoca assimilate_verified_example
    let report = gate.assimilate_verified_example(slang_query, ArithmeticOp::Subtract, 3000);
    assert_eq!(report.extracted_token, "desfalcan");
    assert!(matches!(report.outcome, TransactionOutcome::Committed { .. }));
    assert!(report.registered_regression_id.is_some());

    // 3. En la siguiente llamada resuelve la consulta con traza RPN exacta
    let (_, ans_after) = gate.evaluate_query(slang_query);
    assert_eq!(ans_after.status, "VALID");
    assert!(ans_after.text.contains("50 monedas - 15 monedas") && ans_after.text.contains("35 monedas"), "Obtuvo: {}", ans_after.text);

    // 4. Intento posterior de corromper "desfalcan" -> Add es bloqueado por el test de regresión auto-generado
    let poison = gate.propose_mutation(LexiconPayload::op_alias("desfalcan", ArithmeticOp::Add), 3100);
    assert!(matches!(poison, TransactionOutcome::RejectedByRegression { .. }));
}

#[test]
fn test_4_engram_capsule_immunity_against_corrupted_or_trojan_capsules() {
    let mut local_gate = BlackwallGate::with_genesis();
    // El usuario local asimila y protege su término "liquidaron" -> Subtract
    let r_local = local_gate.assimilate_verified_example("Tengo 100 créditos y me liquidaron 30", ArithmeticOp::Subtract, 4000);
    assert!(matches!(r_local.outcome, TransactionOutcome::Committed { .. }));
    let safe_version = local_gate.current_version();
    let safe_hash = local_gate.head_hash().to_string();

    // A. Cápsula externa legítima que aporta vocabulario nuevo ("abonaron" -> Add)
    let mut peer_store = EngramStore::with_genesis();
    peer_store.append_entry(4100, EntryStatus::LexicalAlias, LexiconPayload::op_alias("abonaron", ArithmeticOp::Add)).unwrap();
    let legit_capsule = peer_store.export_capsule("peer_finance", 14, 14).unwrap();

    let import_ok = local_gate.import_engram_capsule(&legit_capsule, 4200);
    assert!(matches!(import_ok, TransactionOutcome::Committed { .. }));
    let (_, ans_peer) = local_gate.evaluate_query("Tengo 40 créditos y me abonaron 25");
    assert_eq!(ans_peer.status, "VALID");
    assert!(ans_peer.text.contains("40 créditos + 25 créditos") && ans_peer.text.contains("65 créditos"));

    // B. Cápsula corrupta criptográficamente (hash alterado) -> RejectedByIntegrity
    let mut corrupted_capsule = legit_capsule.clone();
    corrupted_capsule.merkle_seal_sha256 = "0000badhash".to_string();
    let import_corrupt = local_gate.import_engram_capsule(&corrupted_capsule, 4300);
    assert!(matches!(import_corrupt, TransactionOutcome::RejectedByIntegrity { .. }));

    // C. Cápsula troyana (intenta redefinir "liquidaron" -> Multiply rompiendo el caso protegido del usuario local)
    let mut trojan_store = EngramStore::with_genesis();
    trojan_store.append_entry(4400, EntryStatus::LexicalAlias, LexiconPayload::op_alias("liquidaron", ArithmeticOp::Multiply)).unwrap();
    let trojan_capsule = trojan_store.export_capsule("trojan_source", 14, 14).unwrap();

    let v_before_trojan = local_gate.current_version();
    let h_before_trojan = local_gate.head_hash().to_string();
    let import_trojan = local_gate.import_engram_capsule(&trojan_capsule, 4500);
    assert!(matches!(import_trojan, TransactionOutcome::RejectedByRegression { .. }), "Esperaba RejectedByRegression, obtuvo: {import_trojan:?}");
    assert_eq!(local_gate.current_version(), v_before_trojan);
    assert_eq!(local_gate.head_hash(), h_before_trojan);
    assert!(local_gate.current_version() > safe_version && safe_hash != local_gate.head_hash());
}

