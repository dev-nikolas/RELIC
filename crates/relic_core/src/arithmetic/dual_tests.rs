//! Reglas de propagación dual y contrato de tamaño (16 B).

use super::dual::{DomainFault, DualNumber, EPS_SINGULAR};
use super::program::RpnProgram;
use super::rpn_eval::DualStack;

#[inline]
fn d(re: f64, du: f64) -> DualNumber {
    DualNumber::new(re, du)
}

#[test]
fn layout_contract() {
    assert_eq!(core::mem::size_of::<DualNumber>(), 16);
    assert_eq!(core::mem::align_of::<DualNumber>(), 16);
    assert_eq!(core::mem::size_of::<DualStack>(), 528);
    assert_eq!(core::mem::size_of::<RpnProgram>(), 584);
}

#[test]
fn sum_sub_mul_propagate_linearly() {
    assert_eq!(d(2.0, 3.0).checked_add(d(5.0, 7.0)).unwrap(), d(7.0, 10.0));
    assert_eq!(d(2.0, 3.0).checked_sub(d(5.0, 7.0)).unwrap(), d(-3.0, -4.0));
    // (u·v)' = u'v + uv' = 3·5 + 2·7 = 29
    assert_eq!(d(2.0, 3.0).checked_mul(d(5.0, 7.0)).unwrap(), d(10.0, 29.0));
}

#[test]
fn quotient_rule_has_correct_sign() {
    // u = 1 + 3ε, v = 2 + ε  ⇒  (u/v)' = (3·2 − 1·1)/4 = 1.25
    let q = d(1.0, 3.0).checked_div(d(2.0, 1.0)).unwrap();
    assert_eq!(q, d(0.5, 1.25));
    // La forma con α/β intercambiados daría (1 − 0.5·3)/2 = −0.25.
    assert_ne!(q.dual, -0.25);
}

#[test]
fn quotient_rule_survives_b_squared_overflow() {
    // b = 1e200: b² = 1e400 = inf. La forma clásica (αb − aβ)/b² colapsa a 0.0.
    let u = d(1.0, 3.0);
    let v = d(1e200, 0.0);
    assert!((v.real * v.real).is_infinite());

    let robust = u.checked_div(v).unwrap();
    assert_eq!(robust.real, 1.0 / 1e200);
    assert_eq!(robust.dual, 3.0 / 1e200);
    assert!(robust.dual > 0.0);

    let classic = (u.dual * v.real - u.real * v.dual) / (v.real * v.real);
    assert_eq!(classic, 0.0);
}

#[test]
fn quotient_rule_avoids_nan_when_numerator_overflows() {
    // Numerador clásico = 1e200·1e200 = inf, denominador = inf ⇒ inf/inf = NaN.
    let u = d(1e200, 1e200);
    let v = d(1e200, 0.0);
    let robust = u.checked_div(v).unwrap();
    assert_eq!(robust, d(1.0, 1.0));

    let classic = (u.dual * v.real - u.real * v.dual) / (v.real * v.real);
    assert!(classic.is_nan());
}

#[test]
fn quotient_rejects_singular_denominator() {
    assert_eq!(
        d(1.0, 1.0).checked_div(d(EPS_SINGULAR * 0.5, 0.0)),
        Err(DomainFault::SingularDenominator)
    );
    assert_eq!(
        d(1.0, 1.0).checked_div(d(0.0, 0.0)),
        Err(DomainFault::SingularDenominator)
    );
}

#[test]
fn integer_powers_cover_negative_base() {
    assert_eq!(d(2.0, 1.0).checked_pow(d(3.0, 0.0)).unwrap(), d(8.0, 12.0));
    assert_eq!(d(-2.0, 1.0).checked_pow(d(3.0, 0.0)).unwrap(), d(-8.0, 12.0));
    assert_eq!(d(2.0, 1.0).checked_pow(d(0.0, 0.0)).unwrap(), d(1.0, 0.0));
    assert_eq!(d(0.0, 1.0).checked_pow(d(1.0, 0.0)).unwrap(), d(0.0, 1.0));
    assert_eq!(d(0.0, 1.0).checked_pow(d(2.0, 0.0)).unwrap(), d(0.0, 0.0));
}

#[test]
fn power_domain_and_singular_cases() {
    assert_eq!(
        d(0.0, 0.0).checked_pow(d(0.0, 0.0)),
        Err(DomainFault::Indeterminate00)
    );
    assert_eq!(
        d(0.0, 1.0).checked_pow(d(-1.0, 0.0)),
        Err(DomainFault::SingularDenominator)
    );
    assert_eq!(
        d(-8.0, 1.0).checked_pow(d(0.5, 0.0)),
        Err(DomainFault::Pow)
    );
    assert_eq!(
        d(0.0, 1.0).checked_pow(d(0.5, 0.0)),
        Err(DomainFault::SingularDerivative)
    );
}

#[test]
fn real_exponent_uses_log_derivative() {
    let r = d(2.0, 1.0).checked_pow(d(0.5, 0.0)).unwrap();
    assert!((r.real - 2.0_f64.sqrt()).abs() < 1e-15);
    assert!((r.dual - 0.5 * 2.0_f64.sqrt() / 2.0).abs() < 1e-15);

    // 2^x en x = 0 ⇒ (1, ln 2)
    let e = d(2.0, 0.0).checked_pow(d(0.0, 1.0)).unwrap();
    assert_eq!(e.real, 1.0);
    assert!((e.dual - 2.0_f64.ln()).abs() < 1e-15);
}

#[test]
fn transcendentals_propagate_known_derivatives() {
    assert_eq!(d(0.0, 1.0).sin().unwrap(), d(0.0, 1.0));
    assert_eq!(d(0.0, 1.0).cos().unwrap(), d(1.0, 0.0));
    assert_eq!(d(0.0, 1.0).tan().unwrap(), d(0.0, 1.0));
    assert_eq!(d(0.0, 1.0).exp().unwrap(), d(1.0, 1.0));
    assert_eq!(d(1.0, 1.0).ln().unwrap(), d(0.0, 1.0));
    assert_eq!(d(4.0, 1.0).sqrt().unwrap(), d(2.0, 0.25));
    assert_eq!(d(-3.0, 1.0).abs().unwrap(), d(3.0, -1.0));
    assert_eq!(d(3.0, 1.0).abs().unwrap(), d(3.0, 1.0));
}

#[test]
fn transcendentals_abstain_on_invalid_domains() {
    assert_eq!(d(-1.0, 1.0).ln(), Err(DomainFault::Ln));
    assert_eq!(d(0.0, 1.0).ln(), Err(DomainFault::Ln));
    assert_eq!(d(-1.0, 1.0).sqrt(), Err(DomainFault::Sqrt));
    assert_eq!(d(0.0, 1.0).sqrt(), Err(DomainFault::SingularDerivative));
    assert_eq!(d(0.0, 0.0).sqrt().unwrap(), d(0.0, 0.0));
    assert_eq!(
        d(core::f64::consts::FRAC_PI_2, 1.0).tan(),
        Err(DomainFault::TanPole)
    );
    assert_eq!(d(1000.0, 1.0).exp(), Err(DomainFault::Overflow));
    assert_eq!(d(0.0, 1.0).abs(), Err(DomainFault::NonDifferentiable));
    assert_eq!(d(0.0, 0.0).abs().unwrap(), d(0.0, 0.0));
}

#[test]
fn non_finite_inputs_are_rejected_not_propagated() {
    let nan = d(f64::NAN, 0.0);
    assert_eq!(nan.checked_add(d(1.0, 0.0)), Err(DomainFault::NonFinite));
    assert_eq!(
        d(f64::INFINITY, 0.0).checked_mul(d(0.0, 0.0)),
        Err(DomainFault::NonFinite)
    );
}

#[test]
fn composite_derivative_sin_times_exp_at_zero() {
    // f = sin(x)·exp(x) ⇒ f(0)=0, f'(0)=1
    let f = d(0.0, 1.0).sin().unwrap().checked_mul(d(0.0, 1.0).exp().unwrap());
    assert_eq!(f.unwrap(), d(0.0, 1.0));
}
