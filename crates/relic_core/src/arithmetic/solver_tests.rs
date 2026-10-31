//! Matriz M1–M22 del solver: contratos de grado, certificación y abstención.

use super::certify::{Certification, Certified};
use super::dual::DomainFault;
use super::lexer::compile;
use super::newton::NewtonFault;
use super::rpn_eval::EvalFault;
use super::solver::{solve, SolveFault, SolveOutcome};

fn outcome(expr: &str) -> SolveOutcome {
    solve(&compile(expr).unwrap(), None)
}

fn outcome_at(expr: &str, x0: f64) -> SolveOutcome {
    solve(&compile(expr).unwrap(), Some(x0))
}

fn certified(o: SolveOutcome) -> Certified {
    match o {
        SolveOutcome::Linear(c) | SolveOutcome::Newton(c) => c,
        other => panic!("se esperaba una raíz certificada, llegó {other:?}"),
    }
}

fn pair(o: SolveOutcome) -> (Certified, Certified) {
    match o {
        SolveOutcome::Quadratic(l, h) => (l, h),
        other => panic!("se esperaba un par de raíces, llegó {other:?}"),
    }
}

fn double(o: SolveOutcome) -> Certified {
    match o {
        SolveOutcome::DoubleRoot(c) => c,
        other => panic!("se esperaba una raíz doble, llegó {other:?}"),
    }
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0)
}

fn rel_close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-12 * expected.abs()
}

#[test]
fn m01_linear_basic() {
    let c = certified(outcome("2*x + 1"));
    assert_eq!(c.root, -0.5);
    assert!(c.is_simple());
    assert_eq!(c.criterion, Certification::Absolute);
}

#[test]
fn m02_linear_fraction() {
    assert_eq!(certified(outcome("x/2 - 3")).root, 6.0);
}

#[test]
fn m03_linear_identity_root() {
    assert_eq!(certified(outcome("x")).root, 0.0);
}

#[test]
fn m04_constant_without_root() {
    assert_eq!(outcome("5"), SolveOutcome::NoRealRoots);
}

#[test]
fn m05_constant_zero_is_everywhere() {
    assert_eq!(outcome("0"), SolveOutcome::InfinitelyMany);
}

#[test]
fn m06_quadratic_two_roots_sorted() {
    let (l, h) = pair(outcome("x^2 - 1"));
    assert_eq!((l.root, h.root), (-1.0, 1.0));
    assert!(l.is_simple() && h.is_simple());
}

#[test]
fn m07_quadratic_double_root() {
    let c = double(outcome("x^2 - 2*x + 1"));
    assert_eq!((c.root, c.multiplicity), (1.0, 2));
    assert!(!c.is_simple());
}

#[test]
fn m08_quadratic_without_real_roots() {
    assert_eq!(outcome("x^2 + 1"), SolveOutcome::NoRealRoots);
}

#[test]
fn m09_quadratic_cancellation_is_stable() {
    let (l, h) = pair(outcome("x^2 - 100000000*x + 1"));
    assert!(close(l.root, 1e-8) && close(h.root, 1e8));
    assert_eq!((l.criterion, h.criterion), (Certification::Absolute, Certification::Relative));
}

#[test]
fn m10_quadratic_survives_b_squared_overflow() {
    let big = format!("1{}", "0".repeat(200));
    let (l, h) = pair(solve(&compile(&format!("x^2 - {big}*x + 1")).unwrap(), None));
    assert!(rel_close(l.root, 1e-200) && rel_close(h.root, 1e200));
    assert_eq!(h.criterion, Certification::Relative);
}

#[test]
fn m11_newton_sin_converges_to_pi() {
    assert!(close(certified(outcome_at("sin(x)", 3.0)).root, core::f64::consts::PI));
}

#[test]
fn m12_newton_exp_converges_to_ln2() {
    assert!(close(certified(outcome_at("exp(x) - 2", 0.0)).root, 2.0_f64.ln()));
}

#[test]
fn m13_newton_log_converges_to_e() {
    assert!(close(certified(outcome_at("ln(x) - 1", 2.0)).root, core::f64::consts::E));
}

#[test]
fn m14_newton_sqrt_converges_to_nine() {
    assert_eq!(certified(outcome_at("sqrt(x) - 3", 10.0)).root, 9.0);
}

#[test]
fn m15_newton_rational_converges_to_half() {
    assert!(close(certified(outcome_at("1/x - 2", 0.3)).root, 0.5));
}

#[test]
fn m16_newton_abs_reaches_negative_root() {
    let c = certified(outcome_at("abs(x) - 1", -0.5));
    assert_eq!(c.root, -1.0);
    assert!(c.is_simple());
}

#[test]
fn m17_non_polynomial_without_seed_abstains() {
    assert_eq!(outcome("sin(x)"), SolveOutcome::Abstain(SolveFault::NeedsInitialGuess));
}

#[test]
fn m18_domain_fault_is_reported_not_hidden() {
    let expected = SolveOutcome::Abstain(SolveFault::Newton(NewtonFault::Eval(
        EvalFault::Domain(DomainFault::Ln),
    )));
    assert_eq!(outcome_at("ln(x) - 1", -1.0), expected);
}

#[test]
fn m19_triple_root_is_flagged_as_non_simple() {
    let c = certified(outcome_at("x^3", 0.0));
    assert_eq!((c.root, c.multiplicity, c.iters), (0.0, 0, 0));
}

#[test]
fn m20_divergence_ends_in_zero_derivative() {
    assert_eq!(
        outcome_at("exp(x) + 1", 0.0),
        SolveOutcome::Abstain(SolveFault::Newton(NewtonFault::ZeroDerivative))
    );
}

#[test]
fn m21_quadratic_double_negative_root() {
    assert_eq!(double(outcome("x^2 + 2*x + 1")).root, -1.0);
}

#[test]
fn m22_linear_negative_slope() {
    assert_eq!(certified(outcome("-3*x + 6")).root, 2.0);
}
