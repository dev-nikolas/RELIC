//! # Newton determinista
//!
//! Iteración de Newton con tope fijo (`MAX_ITER = 16`) sobre el evaluador
//! hyperdual. La parada **no** es el tamaño del paso: se certifica `|f(x)|`
//! contra `CERT_TOL`. Un paso pequeño puede dejar el iterado lejos de la raíz
//! cuando `f'` es minúscula, así que `|Δx| < tol` no es criterio suficiente.

use super::dual::EPS_SINGULAR;
use super::dual2::HyperDual;
use super::program::RpnProgram;
use super::rpn_eval::EvalFault;
use super::rpn_eval2::eval2;

pub const MAX_ITER: usize = 16;
pub const MAX_POLISH: usize = 4;
pub const CERT_TOL: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewtonReport {
    pub root: f64,
    pub residual: f64,
    pub slope: f64,
    pub curvature: f64,
    pub iters: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewtonFault {
    ZeroDerivative,
    NoConvergence,
    NonFinite,
    Eval(EvalFault),
}

/// Iteración completa desde `x0`.
pub fn run(program: &RpnProgram, x0: f64) -> Result<NewtonReport, NewtonFault> {
    iterate(program, x0, MAX_ITER)
}

/// Repaso corto para pulir una raíz ya obtenida por fórmula cerrada.
pub fn refine(program: &RpnProgram, x0: f64) -> Result<NewtonReport, NewtonFault> {
    iterate(program, x0, MAX_POLISH)
}

fn iterate(program: &RpnProgram, x0: f64, max_steps: usize) -> Result<NewtonReport, NewtonFault> {
    if !x0.is_finite() {
        return Err(NewtonFault::NonFinite);
    }
    let mut x = x0;
    let mut iters = 0u8;
    loop {
        let h = eval2(program, HyperDual::variable(x)).map_err(NewtonFault::Eval)?;
        if !h.real.is_finite() || !h.d1.is_finite() || !h.d2.is_finite() {
            return Err(NewtonFault::NonFinite);
        }
        if h.real.abs() <= CERT_TOL {
            return Ok(NewtonReport {
                root: x,
                residual: h.real.abs(),
                slope: h.d1,
                curvature: h.d2,
                iters,
            });
        }
        if iters as usize >= max_steps {
            return Err(NewtonFault::NoConvergence);
        }
        if h.d1.abs() < EPS_SINGULAR {
            return Err(NewtonFault::ZeroDerivative);
        }
        let next = x - h.real / h.d1;
        if !next.is_finite() {
            return Err(NewtonFault::NonFinite);
        }
        x = next;
        iters += 1;
    }
}
