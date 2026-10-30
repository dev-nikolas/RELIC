//! # bidirectional_certificate_tests
//!
//! Pruebas formales de certificados de búsqueda (positivos, negativos y parciales)
//! y resistencia adversarial ante certificados manipulados.
use relic_core::{
    verify_negative, verify_positive, EpisodicMemory, ReasoningEngine, SearchOutcome,
};

fn create_sample_graph() -> EpisodicMemory {
    let mut mem = EpisodicMemory::new();
    mem.insert_edge("Madrid", "Zaragoza");
    mem.insert_edge("Zaragoza", "Barcelona");
    mem.insert_edge("Madrid", "Valencia");
    mem.insert_edge("Valencia", "Barcelona");
    mem.insert_edge("Madrid", "Toledo");
    mem
}

#[test]
fn test_verifier_accepts_valid_positive_certificate() {
    let mut mem = create_sample_graph();
    let res = ReasoningEngine::solve_reachability_bidirectional(
        &mut mem, "Madrid", "Barcelona", None, &[], 10_000,
    );
    assert!(res.is_reachable);
    assert!(verify_positive(&mut mem, &res.path, &[]).is_ok());
}

#[test]
fn test_adversarial_tampered_positive_edge_rejected() {
    let mut mem = create_sample_graph();
    let forged_path = vec!["Madrid".into(), "Sevilla".into(), "Barcelona".into()];
    let check = verify_positive(&mut mem, &forged_path, &[]);
    assert!(check.is_err());
    assert!(check.unwrap_err().contains("Arista inexistente"));
}

#[test]
fn test_adversarial_positive_violating_avoid_rejected() {
    let mut mem = create_sample_graph();
    let path = vec!["Madrid".into(), "Zaragoza".into(), "Barcelona".into()];
    let check = verify_positive(&mut mem, &path, &["Zaragoza"]);
    assert!(check.is_err());
    assert!(check.unwrap_err().contains("nodo prohibido"));
}

#[test]
fn test_verifier_accepts_valid_negative_certificate() {
    let mut mem = create_sample_graph();
    let res = ReasoningEngine::solve_reachability_bidirectional(
        &mut mem, "Madrid", "Toledo", None, &["Toledo"], 10_000,
    );
    assert!(!res.is_reachable);
    if let SearchOutcome::Unreachable { ref closed_set, .. } = res.outcome {
        assert!(verify_negative(&mem, "Madrid", "Toledo", closed_set, &["Toledo"]).is_ok());
    } else {
        panic!("Se esperaba certificado Unreachable");
    }
}

#[test]
fn test_adversarial_incomplete_negative_closed_set_rejected() {
    let mem = create_sample_graph();
    // Conjunto S incompleto: solo Madrid, pero Madrid tiene vecinos (Zaragoza, Valencia, Toledo)
    let forged_s = vec!["Madrid".to_string()];
    let check = verify_negative(&mem, "Madrid", "Barcelona", &forged_s, &[]);
    assert!(check.is_err());
    assert!(check.unwrap_err().contains("Fuga de clausura"));
}

#[test]
fn test_adversarial_negative_set_containing_target_rejected() {
    let mem = create_sample_graph();
    let forged_s = vec!["Madrid".to_string(), "Barcelona".to_string()];
    let check = verify_negative(&mem, "Madrid", "Barcelona", &forged_s, &[]);
    assert!(check.is_err());
    assert!(check.unwrap_err().contains("contiene el nodo destino"));
}

#[test]
fn test_budget_exceeded_partial_certificate_does_not_claim_unreachable() {
    let mut mem = EpisodicMemory::new();
    for i in 0..100 {
        mem.insert_edge(format!("N{i}"), format!("N{}", i + 1));
    }
    // Con presupuesto de 5 operaciones sobre una cadena de 100 saltos
    let res = ReasoningEngine::solve_reachability_bidirectional(
        &mut mem, "N0", "N100", None, &[], 5,
    );
    assert!(!res.is_reachable);
    assert!(res.depth_exhausted);
    match res.outcome {
        SearchOutcome::BudgetExceeded { operations_spent, .. } => {
            assert!(operations_spent <= 5);
        }
        _ => panic!("Debe emitir BudgetExceeded, nunca Unreachable"),
    }
}
