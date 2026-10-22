//! Pruebas de regresión causal y de despacho para RELIC (Fase 7).
//! Menos de 180 líneas.

use relic_core::{
    ArithmeticOp, ArithmeticPlan, ArithmeticStep,
    RirArithmeticOp, RirDocumentV1,
};
use relic_cli::{parse_dispatch_intent, parse_math_expression, DispatchIntent};

#[test]
fn test_dispatch_simple_arithmetic() {
    let intent = parse_dispatch_intent("1 + 1");
    if let DispatchIntent::Arithmetic(plan) = intent {
        assert_eq!(plan.execute(), Ok(2.0));
        assert_eq!(plan.format_result(2.0), "2");
    } else {
        panic!("Esperaba intención aritmética");
    }
}

#[test]
fn test_dispatch_addition_question() {
    let intent = parse_dispatch_intent("¿Cuánto es 2 + 3?");
    if let DispatchIntent::Arithmetic(plan) = intent {
        assert_eq!(plan.execute(), Ok(5.0));
        assert_eq!(plan.format_result(5.0), "5");
    } else {
        panic!("Esperaba intención aritmética");
    }
}

#[test]
fn test_dispatch_inventory_addition() {
    let intent = parse_dispatch_intent("Tengo 2 manzanas y recibo 1 más");
    if let DispatchIntent::Arithmetic(plan) = intent {
        let res = plan.execute().unwrap();
        assert_eq!(res, 3.0);
        assert_eq!(plan.format_result(res), "3 manzanas");
    } else {
        panic!("Esperaba intención de inventario");
    }
}

#[test]
fn test_dispatch_inventory_multi_step() {
    let intent = parse_dispatch_intent("Tengo 2 manzanas y recibo 1 más, luego regalo 2");
    if let DispatchIntent::Arithmetic(plan) = intent {
        let res = plan.execute().unwrap();
        assert_eq!(res, 1.0);
        assert_eq!(plan.format_result(res), "1 manzana");
    } else {
        panic!("Esperaba intención de inventario multi-paso");
    }
}

#[test]
fn test_dispatch_incomplete_missing_initial() {
    assert!(matches!(parse_dispatch_intent("Tengo algunas manzanas y recibo 1 más"), DispatchIntent::Incomplete { .. }));
}

#[test]
fn test_dispatch_incomplete_missing_operand() {
    assert!(matches!(parse_dispatch_intent("¿Cuánto es 2 + ?"), DispatchIntent::Incomplete { .. }));
}

#[test]
fn test_dispatch_decimals_and_negatives() {
    if let DispatchIntent::Arithmetic(plan) = parse_dispatch_intent("¿Cuánto es -5 + 3?") {
        assert_eq!(plan.execute(), Ok(-2.0));
    } else {
        panic!("Esperaba aritmética negativa");
    }
    if let DispatchIntent::Arithmetic(plan) = parse_dispatch_intent("2.5 + 1.5") {
        assert_eq!(plan.execute(), Ok(4.0));
        assert_eq!(plan.format_result(4.0), "4");
    } else {
        panic!("Esperaba aritmética decimal");
    }
}

#[test]
fn test_dispatch_graph_query() {
    assert!(matches!(parse_dispatch_intent("¿Depende el crate relic_cli del paquete memmap2?"), DispatchIntent::GraphReachability(_)));
}

#[test]
fn test_dispatch_architect_original_query() {
    if let DispatchIntent::Arithmetic(plan) = parse_dispatch_intent("¿Cuánto es 1 + 1?") {
        assert_eq!(plan.execute(), Ok(2.0));
        assert_eq!(plan.format_result(2.0), "2");
    } else {
        panic!("Se esperaba resolución aritmética para ¿Cuánto es 1 + 1?");
    }
}

#[test]
fn test_dispatch_unsupported_rejection() {
    assert!(matches!(parse_dispatch_intent("¿Quién eres?"), DispatchIntent::FreeLanguage(_) | DispatchIntent::Incomplete { .. }));
    assert!(matches!(parse_dispatch_intent("¿Qué número viene después del 1?"), DispatchIntent::FreeLanguage(_) | DispatchIntent::Incomplete { .. }));
    assert!(matches!(parse_dispatch_intent("Completa la secuencia: 1, 2, 1, 2, 1, ..."), DispatchIntent::FreeLanguage(_) | DispatchIntent::Incomplete { .. }));
}

#[test]
fn test_parse_math_expression_isolated_operator() {
    assert!(parse_math_expression("Cuento con 3 lápices, pero extravié 1").is_none());
    let plan = parse_math_expression("3 x 4").expect("3 x 4 debe ser válido");
    assert_eq!(plan.execute(), Ok(12.0));

    if let DispatchIntent::Arithmetic(plan) = parse_dispatch_intent("Cuento con 3 lápices, pero extravié 1") {
        let res = plan.execute().unwrap();
        assert_eq!(res, 2.0);
        assert_eq!(plan.format_result(res), "2 lápices");
    } else {
        panic!("Se esperaba resolución de inventario para extravié");
    }
}

#[test]
fn test_dispatch_inventory_multi_step_extended() {
    let intent = parse_dispatch_intent("Poseo 11 peras, recibo 1 y luego dono 11");
    if let DispatchIntent::Arithmetic(plan) = intent {
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0], (ArithmeticOp::Add, 1.0));
        assert_eq!(plan.steps[1], (ArithmeticOp::Subtract, 11.0));
        let res = plan.execute().unwrap();
        assert_eq!(res, 1.0);
        assert_eq!(plan.format_result(res), "1 pera");
    } else {
        panic!("Se esperaba resolución multi-paso");
    }
}

#[test]
fn test_dispatch_unrecognized_verb_rejection() {
    let intent = parse_dispatch_intent("Tenía 4 manzanas y hallé en el almacén 2");
    assert!(matches!(intent, DispatchIntent::Incomplete { .. }));
}

#[test]
fn test_dispatch_legitimate_architecture_query() {
    let intent = parse_dispatch_intent("El arquitecto tenía 5 planos y recibió 3 más");
    if let DispatchIntent::Arithmetic(plan) = intent {
        let res = plan.execute().unwrap();
        assert_eq!(res, 8.0);
        assert_eq!(plan.format_result(res), "8 planos");
    } else {
        panic!("La consulta legítima con 'arquitecto' debe ser soportada");
    }
}

#[test]
fn test_rir_validation_before_execution() {
    let mut invalid_plan = ArithmeticPlan::new(10.0, Some("metros".to_string()));
    invalid_plan.add_step(ArithmeticOp::Divide, 0.0);

    let rir_steps: Vec<ArithmeticStep> = invalid_plan.steps.iter().map(|&(op, val)| {
        let rir_op = match op {
            ArithmeticOp::Add => RirArithmeticOp::Add,
            ArithmeticOp::Subtract => RirArithmeticOp::Subtract,
            ArithmeticOp::Multiply => RirArithmeticOp::Multiply,
            ArithmeticOp::Divide => RirArithmeticOp::Divide,
        };
        ArithmeticStep { operator: rir_op, operand: val }
    }).collect();

    let doc = RirDocumentV1::new_inventory(invalid_plan.unit.clone(), invalid_plan.initial, rir_steps);
    assert!(doc.validate_semantic().is_err());
}
