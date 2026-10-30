
use relic_blackwall::{BlackwallGate, RegressionCase, TransactionOutcome};
use relic_core::ArithmeticOp;
use relic_lexicon::{EngramStore, EntryStatus, IntegrityError, LexiconPayload};

#[test]
fn test_1_live_lexical_learning_full_cycle() {
    let mut gate = BlackwallGate::with_genesis();
    let initial_version = gate.current_version();
    assert_eq!(initial_version, 13);

    let query = "Si tengo 50 y me embolatan 15, ¿cuánto queda?";
    let (_, answer_before) = gate.evaluate_query(query);
    assert_eq!(answer_before.status, "INCOMPLETE");
    assert_eq!(answer_before.route, "epistemic_rejection");

    let outcome = gate.propose_mutation(LexiconPayload::op_alias("embolatan", ArithmeticOp::Subtract), 1_700_000_001);
    match outcome {
        TransactionOutcome::Committed { version, ref entry_hash } => {
            assert_eq!(version, initial_version + 1);
            assert_eq!(entry_hash.len(), 64);
        }
        other => panic!("Se esperaba Committed, obtuvo: {other:?}"),
    }

    let (_, answer_after) = gate.evaluate_query(query);
    assert_eq!(answer_after.status, "VALID");
    assert_eq!(answer_after.route, "deterministic_arithmetic");
    assert!(answer_after.text.contains("35"));
    assert!(answer_after.text.contains("50 - 15 = 35"));
}

#[test]
fn test_2_blackwall_defense_blocks_poisoning_and_regression() {
    let mut gate = BlackwallGate::with_genesis();
    let (v_before, hash_before) = (gate.current_version(), gate.head_hash().to_string());

    let outcome_oracle = gate.propose_mutation(LexiconPayload::op_alias("más", ArithmeticOp::Subtract), 1_700_000_010);
    match outcome_oracle {
        TransactionOutcome::RejectedByOracle { ref reason } => {
            assert!(reason.contains("más") && reason.contains("reservada"));
        }
        other => panic!("El Oráculo debía bloquear 'más', obtuvo: {other:?}"),
    }
    assert_eq!(gate.current_version(), v_before);
    assert_eq!(gate.head_hash(), hash_before);
    let (_, check_mas) = gate.evaluate_query("dos más dos");
    assert_eq!(check_mas.status, "VALID");
    assert!(check_mas.text.contains("2 + 2 = 4"));

    let outcome_reg = gate.propose_mutation(LexiconPayload::entity_alias("Bogotá", "París"), 1_700_000_020);
    match outcome_reg {
        TransactionOutcome::RejectedByRegression { ref failed_case, ref reason } => {
            assert_eq!(failed_case, "reg_graph_spatial_07");
            assert!(reason.contains("AMBIGUOUS") || reason.contains("Bogotá"));
        }
        other => panic!("Debió bloquear la colisión de alias, obtuvo: {other:?}"),
    }

    let learn_ok = gate.propose_mutation(LexiconPayload::op_alias("embolatan", ArithmeticOp::Subtract), 1_700_000_030);
    assert!(matches!(learn_ok, TransactionOutcome::Committed { .. }));
    gate.add_regression_case(RegressionCase {
        id: "reg_user_jargon_embolatan".to_string(),
        query: "Si tengo 50 y me embolatan 15, ¿cuánto queda?".to_string(),
        expected_substring: "50 - 15 = 35".to_string(),
        expected_route: "deterministic_arithmetic".to_string(),
        expected_status: "VALID".to_string(),
    });

    let (v_protected, hash_protected) = (gate.current_version(), gate.head_hash().to_string());
    let poison_jargon = gate.propose_mutation(LexiconPayload::op_alias("embolatan", ArithmeticOp::Add), 1_700_000_040);
    match poison_jargon {
        TransactionOutcome::RejectedByRegression { ref failed_case, .. } => assert_eq!(failed_case, "reg_user_jargon_embolatan"),
        other => panic!("Debió rechazar por regresión, obtuvo: {other:?}"),
    }
    assert_eq!(gate.current_version(), v_protected);
    assert_eq!(gate.head_hash(), hash_protected);
}

#[test]
fn test_3_rollback_with_scar_preserves_history_and_restores_view() {
    let mut gate = BlackwallGate::with_genesis();
    let v0 = gate.current_version();

    let out_v1 = gate.propose_mutation(LexiconPayload::op_alias("coronar", ArithmeticOp::Add), 1_700_000_100);
    let v1 = v0 + 1;
    assert!(matches!(out_v1, TransactionOutcome::Committed { version, .. } if version == v1));

    let out_v2 = gate.propose_mutation(LexiconPayload::op_alias("embolatan", ArithmeticOp::Subtract), 1_700_000_200);
    let v2 = v0 + 2;
    assert!(matches!(out_v2, TransactionOutcome::Committed { version, .. } if version == v2));

    let (_, ans_coronar) = gate.evaluate_query("tengo 20 y logro coronar 10");
    assert!(ans_coronar.status == "VALID" && ans_coronar.text.contains("20 + 10 = 30"));
    let (_, ans_embolatan) = gate.evaluate_query("Si tengo 50 y me embolatan 15, ¿cuánto queda?");
    assert!(ans_embolatan.status == "VALID" && ans_embolatan.text.contains("50 - 15 = 35"));

    let v3 = v0 + 3;
    let out_rb = gate.rollback_to(v1, "Descartar jerga experimental de v2", 1_700_000_300);
    assert!(matches!(out_rb, TransactionOutcome::RolledBack { version, target_version, .. } if version == v3 && target_version == v1));

    let entries = gate.log_entries();
    assert_eq!(entries.len(), v3 as usize);
    assert_eq!(entries.last().unwrap().status, EntryStatus::Reverted);
    assert!(matches!(entries.last().unwrap().payload, LexiconPayload::Rollback { target_version, .. } if target_version == v1));

    let (_, ans_coronar_after) = gate.evaluate_query("tengo 20 y logro coronar 10");
    assert!(ans_coronar_after.status == "VALID" && ans_coronar_after.text.contains("20 + 10 = 30"));
    let (_, ans_embolatan_after) = gate.evaluate_query("Si tengo 50 y me embolatan 15, ¿cuánto queda?");
    assert_eq!(ans_embolatan_after.status, "INCOMPLETE");
}

#[test]
fn test_4_cryptographic_integrity_detects_hash_chain_tampering() {
    let mut gate = BlackwallGate::with_genesis();
    gate.propose_mutation(LexiconPayload::op_alias("embolatan", ArithmeticOp::Subtract), 1_700_000_500);
    let valid_jsonl = gate.to_jsonl_string();
    let loaded_ok = EngramStore::from_jsonl_str(&valid_jsonl).expect("JSONL legítimo");
    assert_eq!(loaded_ok.current_version(), 14);

    let mut lines: Vec<String> = valid_jsonl.lines().map(String::from).collect();
    assert!(lines[3].contains("Lima"));
    lines[3] = lines[3].replacen("Lima", "Limo", 1);
    let tampered_jsonl = lines.join("\n") + "\n";

    match EngramStore::from_jsonl_str(&tampered_jsonl) {
        Err(IntegrityError::HashMismatch { version: 4, .. }) => {}
        other => panic!("Debió rechazar por HashMismatch en versión 4, obtuvo: {other:?}"),
    }
}
