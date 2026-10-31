//! # Solver algebraico con abstención
//!
//! Despacho determinista:
//!
//! 1. Programa constante ⇒ `InfinitelyMany` o `NoRealRoots`.
//! 2. Programa afín/cuadrático (cota sintáctica de `classify`) ⇒ coeficientes
//!    exactos con `poly2` y fórmula cerrada, sin depender de una semilla.
//! 3. Cualquier otro programa ⇒ Newton con semilla obligatoria.
//!
//! No hay heurística de "adivinar" una semilla. Sin semilla, un programa no
//! polinómico se abstiene con `NeedsInitialGuess`.

use super::certify::{self, Certified};
use super::dual::DualNumber;
use super::newton::{self, NewtonFault, CERT_TOL};
use super::poly2::{self, Poly2, PolyFault};
use super::program::{ProgramFault, RpnProgram};
use super::rpn_eval::{eval, EvalFault};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SolveOutcome {
    Linear(Certified),
    Quadratic(Certified, Certified),
    DoubleRoot(Certified),
    Newton(Certified),
    NoRealRoots,
    InfinitelyMany,
    Abstain(SolveFault),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveFault {
    Program(ProgramFault),
    NeedsInitialGuess,
    Poly(PolyFault),
    Eval(EvalFault),
    Newton(NewtonFault),
    Uncertified,
}

pub fn solve(program: &RpnProgram, x0: Option<f64>) -> SolveOutcome {
    if let Err(e) = program.validate() {
        return SolveOutcome::Abstain(SolveFault::Program(e));
    }
    if !program.has_variable() {
        return match eval(program, DualNumber::constant(0.0)) {
            Ok(v) if v.real.abs() <= CERT_TOL => SolveOutcome::InfinitelyMany,
            Ok(_) => SolveOutcome::NoRealRoots,
            Err(e) => SolveOutcome::Abstain(SolveFault::Eval(e)),
        };
    }
    let class = match program.class() {
        Ok(c) => c,
        Err(e) => return SolveOutcome::Abstain(SolveFault::Program(e)),
    };
    if class.is_linear() || class.is_quadratic() {
        return match poly2::extract(program) {
            Ok(p) => solve_poly(program, p),
            Err(e) => SolveOutcome::Abstain(SolveFault::Poly(e)),
        };
    }
    match x0 {
        Some(seed) => solve_newton(program, seed),
        None => SolveOutcome::Abstain(SolveFault::NeedsInitialGuess),
    }
}

fn solve_poly(program: &RpnProgram, p: Poly2) -> SolveOutcome {
    match p.degree() {
        0 => {
            if p.c0.abs() <= CERT_TOL {
                SolveOutcome::InfinitelyMany
            } else {
                SolveOutcome::NoRealRoots
            }
        }
        1 => match refresh(program, p, -p.c0 / p.c1, 1, 0) {
            Ok(c) => SolveOutcome::Linear(c),
            Err(e) => SolveOutcome::Abstain(e),
        },
        _ => solve_quadratic(program, p),
    }
}

fn solve_quadratic(program: &RpnProgram, p: Poly2) -> SolveOutcome {
    let scale = p.scale();
    if scale <= 0.0 {
        return SolveOutcome::InfinitelyMany;
    }
    let (a, b, c) = (p.c2 / scale, p.c1 / scale, p.c0 / scale);
    let disc = b * b - 4.0 * a * c;
    if !disc.is_finite() {
        return SolveOutcome::Abstain(SolveFault::Uncertified);
    }
    if disc < 0.0 {
        return SolveOutcome::NoRealRoots;
    }
    if disc == 0.0 {
        let root = -b / (2.0 * a);
        return match refresh(program, p, root, 2, 0) {
            Ok(c) => SolveOutcome::DoubleRoot(c),
            Err(e) => SolveOutcome::Abstain(e),
        };
    }
    // Forma de Citardauq: q evita la cancelación de la fórmula clásica.
    let s = disc.sqrt();
    let q = if b < 0.0 { -0.5 * (b - s) } else { -0.5 * (b + s) };
    if q == 0.0 {
        return SolveOutcome::Abstain(SolveFault::Uncertified);
    }
    let r1 = q / a;
    let r2 = c / q;
    if !r1.is_finite() || !r2.is_finite() {
        return SolveOutcome::Abstain(SolveFault::Uncertified);
    }
    let (lo, hi) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
    let left = match refresh(program, p, lo, 1, 0) {
        Ok(c) => c,
        Err(e) => return SolveOutcome::Abstain(e),
    };
    let right = match refresh(program, p, hi, 1, 0) {
        Ok(c) => c,
        Err(e) => return SolveOutcome::Abstain(e),
    };
    if left.root == right.root {
        return SolveOutcome::DoubleRoot(right);
    }
    SolveOutcome::Quadratic(left, right)
}

/// Certifica una raíz de la fórmula cerrada; si falla, intenta pulirla con
/// Newton sobre el programa original y vuelve a certificar.
fn refresh(
    program: &RpnProgram,
    p: Poly2,
    root: f64,
    mult: u8,
    iters: u8,
) -> Result<Certified, SolveFault> {
    if let Ok(c) = certify::poly(p, root, mult, iters) {
        return Ok(c);
    }
    let n = newton::refine(program, root).map_err(SolveFault::Newton)?;
    certify::probe(n.root, n.residual, n.slope, n.curvature, n.iters)
        .map_err(|_| SolveFault::Uncertified)
}

fn solve_newton(program: &RpnProgram, seed: f64) -> SolveOutcome {
    match newton::run(program, seed) {
        Ok(n) => match certify::probe(n.root, n.residual, n.slope, n.curvature, n.iters) {
            Ok(c) => SolveOutcome::Newton(c),
            Err(_) => SolveOutcome::Abstain(SolveFault::Uncertified),
        },
        Err(e) => SolveOutcome::Abstain(SolveFault::Newton(e)),
    }
}
