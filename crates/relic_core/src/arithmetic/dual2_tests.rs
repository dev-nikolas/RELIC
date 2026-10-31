//! Segundas derivadas, reglas hyperduales y rango del cociente.

use super::dual::{DomainFault, EPS_SINGULAR};
use super::dual2::HyperDual;

fn h(real: f64, d1: f64, d2: f64) -> HyperDual {
    HyperDual::new(real, d1, d2)
}

#[test]
fn product_second_derivative() {
    // (uv)'' = u''v + 2u'v' + uv'' = 20 + 42 + 22
    assert_eq!(h(2.0, 3.0, 4.0).checked_mul(h(5.0, 7.0, 11.0)).unwrap(), h(10.0, 29.0, 84.0));
}

#[test]
fn quotient_second_derivative_matches_closed_form() {
    // f'' = a''/b − 2a'b'/b² + 2a(b')²/b³ − ab''/b² = 2.5 − 1.5 + 0.25
    assert_eq!(h(1.0, 3.0, 5.0).checked_div(h(2.0, 1.0, 0.0)).unwrap(), h(0.5, 1.25, 1.25));
}

#[test]
fn quotient_never_forms_b_squared() {
    let q = h(1.0, 3.0, 5.0).checked_div(h(1e200, 0.0, 0.0)).unwrap();
    assert_eq!(q.real, 1.0 / 1e200);
    assert_eq!(q.d1, 3.0 / 1e200);
    assert_eq!(q.d2, 5.0 / 1e200);
}

#[test]
fn quotient_rejects_singular_denominator() {
    assert_eq!(
        h(1.0, 1.0, 1.0).checked_div(h(EPS_SINGULAR * 0.5, 0.0, 0.0)),
        Err(DomainFault::SingularDenominator)
    );
}

#[test]
fn transcendentals_carry_second_derivatives() {
    assert_eq!(h(0.0, 1.0, 0.0).sin().unwrap(), h(0.0, 1.0, 0.0));
    assert_eq!(h(0.0, 1.0, 0.0).cos().unwrap(), h(1.0, 0.0, -1.0));
    assert_eq!(h(0.0, 1.0, 0.0).exp().unwrap(), h(1.0, 1.0, 1.0));
    assert_eq!(h(1.0, 1.0, 0.0).ln().unwrap(), h(0.0, 1.0, -1.0));
    assert_eq!(h(4.0, 1.0, 0.0).sqrt().unwrap(), h(2.0, 0.25, -0.03125));
    assert_eq!(h(0.0, 1.0, 0.0).tan().unwrap(), h(0.0, 1.0, 0.0));
}

#[test]
fn abs_and_domain_faults() {
    assert_eq!(h(-3.0, 1.0, 2.0).abs().unwrap(), h(3.0, -1.0, -2.0));
    assert_eq!(h(-1.0, 1.0, 0.0).ln(), Err(DomainFault::Ln));
    assert_eq!(h(0.0, 1.0, 0.0).sqrt(), Err(DomainFault::SingularDerivative));
}

#[test]
fn integer_power_second_derivative() {
    // f = x³ en x = 2 ⇒ (8, 12, 12)
    assert_eq!(
        h(2.0, 1.0, 0.0).checked_pow(h(3.0, 0.0, 0.0)).unwrap(),
        h(8.0, 12.0, 12.0)
    );
    // f = x² en x = 2 ⇒ (4, 4, 2)
    assert_eq!(
        h(2.0, 1.0, 0.0).checked_pow(h(2.0, 0.0, 0.0)).unwrap(),
        h(4.0, 4.0, 2.0)
    );
    // x² en x = 0 con u'' = 1 ⇒ f'' = 2u''u + 2(u')² = 0
    assert_eq!(
        h(0.0, 0.0, 1.0).checked_pow(h(2.0, 0.0, 0.0)).unwrap(),
        h(0.0, 0.0, 0.0)
    );
}

#[test]
fn variable_exponent_is_rejected() {
    assert_eq!(
        h(2.0, 1.0, 0.0).checked_pow(h(3.0, 1.0, 0.0)),
        Err(DomainFault::Pow)
    );
}

#[test]
fn non_finite_inputs_are_rejected() {
    assert_eq!(
        h(f64::NAN, 0.0, 0.0).checked_add(h(1.0, 0.0, 0.0)),
        Err(DomainFault::NonFinite)
    );
}
