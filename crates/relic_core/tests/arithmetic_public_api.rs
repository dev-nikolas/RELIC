//! Gate de Fase 5: la superficie pública de `arithmetic` está congelada.
//!
//! Este test vive fuera del crate a propósito: si un nombre deja de ser
//! `pub`, deja de compilar. No prueba rendimiento, solo contrato.

use std::mem::size_of;

use relic_core::arithmetic::{
    compile, eval, eval2, extract, solve, ArithmeticError, ArithmeticOp, ArithmeticPlan,
    ArithmeticReduction, Certified, Certification, DualNumber, DualStack, HyperDual, HyperStack,
    LexFault, Poly2, ProgramClass, RpnProgram, SolveOutcome,
};

#[test]
fn historical_variants_are_frozen() {
    // Match exhaustivo sin comodín: añadir una variante rompe este test.
    let ops = [
        ArithmeticOp::Add,
        ArithmeticOp::Subtract,
        ArithmeticOp::Multiply,
        ArithmeticOp::Divide,
    ];
    let mut names = [""; 4];
    for (i, op) in ops.into_iter().enumerate() {
        names[i] = match op {
            ArithmeticOp::Add => "add",
            ArithmeticOp::Subtract => "subtract",
            ArithmeticOp::Multiply => "multiply",
            ArithmeticOp::Divide => "divide",
        };
    }
    assert_eq!(names, ["add", "subtract", "multiply", "divide"]);
}

#[test]
fn historical_plan_behaves_as_before() {
    let mut plan = ArithmeticPlan::new(2.0, None);
    plan.add_step(ArithmeticOp::Multiply, 3.0);
    plan.add_step(ArithmeticOp::Subtract, 1.0);
    assert_eq!(plan.execute(), Ok(5.0));
    assert_eq!(plan.steps.len(), 2);
    assert!(plan.unit.is_none());
    assert!(plan.reductions.is_empty());

    assert_eq!(
        ArithmeticError::DivisionByZero.to_string(),
        "División por cero no permitida"
    );
    let reduction = ArithmeticReduction {
        step_index: 1,
        left: 2.0,
        op: ArithmeticOp::Add,
        right: 3.0,
        result: 5.0,
    };
    assert_eq!(reduction.result, 5.0);
}

#[test]
fn layouts_are_frozen() {
    assert_eq!(size_of::<DualNumber>(), 16);
    assert_eq!(size_of::<HyperDual>(), 24);
    assert_eq!(size_of::<Poly2>(), 24);
    assert_eq!(size_of::<Certified>(), 24);
    assert_eq!(size_of::<RpnProgram>(), 584);
    assert_eq!(size_of::<DualStack>(), 528);
    assert_eq!(size_of::<HyperStack>(), 776);
}

#[test]
fn first_and_second_order_agree() {
    let program = compile("x^3 - 2*x").unwrap();
    let first = eval(&program, DualNumber::variable(2.0)).unwrap();
    let second = eval2(&program, HyperDual::variable(2.0)).unwrap();
    assert_eq!((first.real, first.dual), (4.0, 10.0));
    assert_eq!((second.real, second.d1, second.d2), (4.0, 10.0, 12.0));
}

#[test]
fn solver_and_extraction_are_reachable() {
    let program = compile("x^2 - 1").unwrap();
    let class: ProgramClass = program.class().unwrap();
    assert!(class.is_quadratic() && !class.is_linear());

    let poly = extract(&program).unwrap();
    assert_eq!((poly.c0, poly.c1, poly.c2), (-1.0, 0.0, 1.0));

    match solve(&program, None) {
        SolveOutcome::Quadratic(l, h) => assert_eq!((l.root, h.root), (-1.0, 1.0)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn abstention_is_part_of_the_contract() {
    assert_eq!(compile("").err(), Some(LexFault::Empty));

    let program = compile("sin(x)").unwrap();
    assert!(matches!(solve(&program, None), SolveOutcome::Abstain(_)));

    let certified = Certified {
        root: 0.0,
        residual: 0.0,
        criterion: Certification::Absolute,
        multiplicity: 1,
        iters: 0,
    };
    assert!(certified.is_simple());

    let empty = RpnProgram::default();
    assert!(empty.is_empty());
}
