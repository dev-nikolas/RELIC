//! Compilación a RPN, precedencia, clasificación y evaluación dual.

use super::dual::{DomainFault, DualNumber};
use super::lexer::{compile, LexFault};
use super::program::{DiffOp, ProgramFault, RpnProgram, UNKNOWN_DEGREE};
use super::rpn_eval::{eval, EvalFault};

fn value(src: &str) -> f64 {
    eval(&compile(src).unwrap(), DualNumber::constant(0.0))
        .unwrap()
        .real
}

fn at(src: &str, x: f64) -> (f64, f64) {
    let r = eval(&compile(src).unwrap(), DualNumber::variable(x)).unwrap();
    (r.real, r.dual)
}

#[test]
fn arithmetic_and_derivatives() {
    assert_eq!(at("x^2 + 1", 3.0), (10.0, 6.0));
    assert_eq!(at("sin(x)", 0.0), (0.0, 1.0));
    assert_eq!(at("1/x", 2.0), (0.5, -0.25));
    assert_eq!(at("sqrt(x)", 4.0), (2.0, 0.25));
    assert_eq!(at("abs(x)", -2.0), (2.0, -1.0));
    assert_eq!(value("2 - -3"), 5.0);
    assert_eq!(value("-(2 + 3)"), -5.0);
    assert_eq!(value("8 / 2 / 2"), 2.0);
    assert_eq!(value("(1 + 2) * (3 + 4)"), 21.0);
}

#[test]
fn precedence_and_associativity() {
    assert_eq!(value("2 + 3 * 4"), 14.0);
    assert_eq!(value("(2 + 3) * 4"), 20.0);
    assert_eq!(value("2 - 3 - 4"), -5.0);
    // `-x^2` = -(x²), no (-x)²
    assert_eq!(at("-x^2", 3.0), (-9.0, -6.0));
    // `^` es asociativa a la derecha: 2^(3^2) = 512, no (2^3)^2 = 64
    assert_eq!(value("2^3^2"), 512.0);
}

#[test]
fn structural_classification() {
    let linear = compile("3*x + 1").unwrap().class().unwrap();
    assert_eq!(linear.degree_bound, 1);
    assert!(linear.is_linear() && !linear.is_quadratic());

    let quadratic = compile("x^2 + 1").unwrap().class().unwrap();
    assert_eq!(quadratic.degree_bound, 2);
    assert!(quadratic.is_quadratic() && !quadratic.is_linear());

    assert_eq!(compile("x*x*x").unwrap().class().unwrap().degree_bound, 3);

    let rational = compile("1/x").unwrap().class().unwrap();
    assert!(rational.var_in_denominator);
    assert_eq!(rational.degree_bound, UNKNOWN_DEGREE);

    assert!(compile("sin(x)").unwrap().class().unwrap().has_transcendental);
    assert!(compile("x^2.5").unwrap().class().unwrap().has_transcendental);
    assert!(!compile("x^2").unwrap().class().unwrap().has_transcendental);
}

#[test]
fn variable_detection() {
    assert!(compile("2*x").unwrap().has_variable());
    assert!(!compile("2*3").unwrap().has_variable());
    assert!(!RpnProgram::new().has_variable());
}

#[test]
fn lexical_faults_are_explicit() {
    assert_eq!(compile("").err(), Some(LexFault::Empty));
    assert_eq!(compile("   ").err(), Some(LexFault::Empty));
    assert_eq!(compile("2 +").err(), Some(LexFault::TrailingOperator));
    assert_eq!(compile("(2 + 3").err(), Some(LexFault::UnbalancedParen));
    assert_eq!(compile("2 3").err(), Some(LexFault::InvalidChar));
    assert_eq!(compile("foo(2)").err(), Some(LexFault::InvalidChar));
    assert_eq!(compile("sin x").err(), Some(LexFault::InvalidChar));
    assert_eq!(compile("1.2.3").err(), Some(LexFault::MalformedNumber));
    assert_eq!(compile("2 x").err(), Some(LexFault::InvalidChar));
}

#[test]
fn stack_limits_are_enforced() {
    let deep = "(".repeat(33) + "x" + &")".repeat(33);
    assert_eq!(compile(&deep).err(), Some(LexFault::TooDeep));

    let long = vec!["1"; 40].join("+");
    assert_eq!(compile(&long).err(), Some(LexFault::ProgramFull));
}

#[test]
fn evaluator_rejects_malformed_stacks() {
    let mut under = RpnProgram::new();
    under.push_op(DiffOp::Add).unwrap();
    assert_eq!(
        eval(&under, DualNumber::constant(0.0)),
        Err(EvalFault::StackUnderflow)
    );

    let mut leftover = RpnProgram::new();
    leftover.push_const(1.0).unwrap();
    leftover.push_const(2.0).unwrap();
    assert_eq!(
        eval(&leftover, DualNumber::constant(0.0)),
        Err(EvalFault::LeftoverStack)
    );

    assert_eq!(under.validate(), Err(ProgramFault::StackUnderflow));
}

#[test]
fn corrupted_len_is_rejected_not_a_panic() {
    let mut p = RpnProgram::new();
    p.len = 200;
    assert_eq!(p.validate(), Err(ProgramFault::Full));
    assert!(!p.has_variable());
    assert_eq!(super::poly2::extract(&p), Err(super::poly2::PolyFault::Malformed));
}

#[test]
fn domain_faults_cross_the_evaluator() {
    let p = compile("ln(x)").unwrap();
    assert_eq!(
        eval(&p, DualNumber::variable(-1.0)),
        Err(EvalFault::Domain(DomainFault::Ln))
    );
    let p = compile("1/x").unwrap();
    assert_eq!(
        eval(&p, DualNumber::variable(0.0)),
        Err(EvalFault::Domain(DomainFault::SingularDenominator))
    );
}
