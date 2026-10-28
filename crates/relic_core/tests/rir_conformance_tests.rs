//! Pruebas de conformidad, verificación y casos adversariales para RIR v1 (Fase 4).

use relic_core::rir::{
    ArithmeticStep, DomainKind, RirArithmeticOp, RirDocumentV1, RirValidationError, ValidityStatus,
};

#[test]
fn test_valid_inventory_roundtrip() {
    let doc = RirDocumentV1::new_inventory(
        Some("manzanas".to_string()),
        2.0,
        vec![
            ArithmeticStep { operator: RirArithmeticOp::Add, operand: 1.0 },
            ArithmeticStep { operator: RirArithmeticOp::Subtract, operand: 2.0 },
        ],
    );
    assert!(doc.validate_semantic().is_ok());
    let json = doc.to_canonical_json().expect("Serializar");
    let parsed: RirDocumentV1 = RirDocumentV1::from_json_str(&json).expect("Deserializar");
    assert_eq!(doc, parsed);
    assert_eq!(parsed.operations.len(), 2);
    assert_eq!(parsed.operations[0].operator, RirArithmeticOp::Add);
    assert_eq!(parsed.operations[1].operator, RirArithmeticOp::Subtract);
}

#[test]
fn test_reject_missing_initial_quantity() {
    let mut doc = RirDocumentV1::new_inventory(
        Some("libros".to_string()),
        5.0,
        vec![ArithmeticStep { operator: RirArithmeticOp::Subtract, operand: 2.0 }],
    );
    doc.initial_quantity = None;
    assert_eq!(
        doc.validate_semantic(),
        Err(RirValidationError::MissingRequiredFields(vec!["initial_quantity".to_string()]))
    );
}

#[test]
fn test_reject_division_by_zero() {
    let doc = RirDocumentV1::new_inventory(
        None,
        10.0,
        vec![ArithmeticStep { operator: RirArithmeticOp::Divide, operand: 0.0 }],
    );
    assert_eq!(doc.validate_semantic(), Err(RirValidationError::DivisionByZero));
}

#[test]
fn test_reject_non_finite_number() {
    let doc = RirDocumentV1::new_inventory(
        None,
        f64::NAN,
        vec![ArithmeticStep { operator: RirArithmeticOp::Add, operand: 1.0 }],
    );
    assert_eq!(
        doc.validate_semantic(),
        Err(RirValidationError::NonFiniteNumber("initial_quantity".to_string()))
    );
}

#[test]
fn test_valid_graph_query() {
    let doc = RirDocumentV1::new_graph(
        DomainKind::SoftwareDependency,
        "relic_cli".to_string(),
        "memmap2".to_string(),
        None,
        Some("relic_core".to_string()),
        false,
    );
    assert!(doc.validate_semantic().is_ok());
}

#[test]
fn test_rejected_incomplete_document() {
    let doc = RirDocumentV1::new_rejected(
        ValidityStatus::Incomplete,
        "Se desconoce la cantidad inicial",
        vec!["initial_quantity".to_string()],
    );
    assert_eq!(
        doc.validate_semantic(),
        Err(RirValidationError::NonExecutableStatus(ValidityStatus::Incomplete))
    );
}

#[test]
fn test_adversarial_unsupported_version() {
    let mut doc = RirDocumentV1::new_inventory(
        None,
        1.0,
        vec![ArithmeticStep { operator: RirArithmeticOp::Add, operand: 1.0 }],
    );
    doc.schema_version = 99;
    assert_eq!(doc.validate_semantic(), Err(RirValidationError::UnsupportedSchemaVersion(99)));
}

#[test]
fn test_adversarial_contradictory_valid_but_rejected() {
    let mut doc = RirDocumentV1::new_inventory(
        None,
        10.0,
        vec![ArithmeticStep { operator: RirArithmeticOp::Add, operand: 2.0 }],
    );
    doc.uncertainty.is_rejected = true;
    assert_eq!(doc.validate_semantic(), Err(RirValidationError::ContradictoryRejectionState));
}

#[test]
fn test_adversarial_contradictory_invalid_but_not_rejected() {
    let mut doc = RirDocumentV1::new_rejected(ValidityStatus::Ambiguous, "Ambigüedad inducida", vec![]);
    doc.uncertainty.is_rejected = false;
    assert_eq!(doc.validate_semantic(), Err(RirValidationError::ContradictoryRejectionState));
}

#[test]
fn test_adversarial_infinity_and_nan_operands() {
    for (op, val) in [
        (RirArithmeticOp::Add, f64::INFINITY),
        (RirArithmeticOp::Subtract, f64::NEG_INFINITY),
        (RirArithmeticOp::Multiply, f64::NAN),
    ] {
        let doc = RirDocumentV1::new_inventory(None, 5.0, vec![ArithmeticStep { operator: op, operand: val }]);
        assert_eq!(
            doc.validate_semantic(),
            Err(RirValidationError::NonFiniteNumber("operations[0].operand".to_string()))
        );
    }
}

#[test]
fn test_adversarial_near_zero_division() {
    let doc = RirDocumentV1::new_inventory(
        None,
        10.0,
        vec![ArithmeticStep { operator: RirArithmeticOp::Divide, operand: 1e-14 }],
    );
    assert_eq!(doc.validate_semantic(), Err(RirValidationError::DivisionByZero));
}

#[test]
fn test_adversarial_empty_graph_nodes() {
    let doc = RirDocumentV1::new_graph(
        DomainKind::SpatialGraph, "   ".to_string(), "Madrid".to_string(), None, None, false,
    );
    assert_eq!(doc.validate_semantic(), Err(RirValidationError::InvalidGraphSpec));
}

#[test]
fn test_adversarial_empty_operations() {
    let doc = RirDocumentV1::new_inventory(None, 10.0, vec![]);
    assert_eq!(doc.validate_semantic(), Err(RirValidationError::InvalidInventorySpec));
}

#[test]
fn test_adversarial_json_injection_resilience() {
    let malicious_json = r#"{
        "schema_version": 1,
        "intent": "inventory_update",
        "domain": "inventory",
        "status": "VALID",
        "initial_quantity": 4.0,
        "operations": [{"operator": "add", "operand": 3.0}],
        "uncertainty": {"is_rejected": false, "status": "VALID", "missing_fields": []},
        "injected_untrusted_payload": "DROP TABLE memory;",
        "hidden_backdoor": 12345
    }"#;
    let doc = RirDocumentV1::from_json_str(malicious_json).expect("Debe ignorar campos espurios");
    assert!(doc.validate_semantic().is_ok());
    assert_eq!(doc.initial_quantity, Some(4.0));
}
