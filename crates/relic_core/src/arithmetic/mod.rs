//! # Arithmetic
//!
//! Superficie pública congelada del razonamiento aritmético determinista.
//!
//! Capas:
//!
//! - **Contrato histórico** (inmutable): `ArithmeticPlan`, `ArithmeticOp`,
//!   `ArithmeticError`, `ArithmeticReduction`, con `Vec`/`String` y
//!   `format_formula` intactos.
//! - **Motor de primer orden**: `DualNumber`, `compile`, `eval`, `RpnProgram`.
//! - **Motor de segundo orden y solver**: `HyperDual`, `eval2`, `Poly2`,
//!   `Certified`, `solve`.
//!
//! Los módulos de implementación son privados a propósito: lo estable son los
//! re-exports de abajo, no las rutas internas. Ver `docs/API_ARITMETICA.md`.

mod certify;
mod classify;
mod dual;
mod dual2;
mod dual2_transcendental;
mod dual_transcendental;
mod lexer;
mod newton;
mod plan;
mod poly2;
mod program;
mod rpn_eval;
mod rpn_eval2;
mod solver;

pub use certify::{Certification, Certified};
pub use classify::ProgramClass;
pub use dual::{DomainFault, DualNumber, DualResult, EPS_SINGULAR};
pub use dual2::{HyperDual, HyperResult};
pub use lexer::{compile, LexFault};
pub use newton::{NewtonFault, NewtonReport, CERT_TOL, MAX_ITER, MAX_POLISH};
pub use plan::{ArithmeticError, ArithmeticOp, ArithmeticPlan, ArithmeticReduction};
pub use poly2::{extract, Poly2, PolyFault};
pub use program::{
    DiffOp, ProgramFault, RpnProgram, MAX_DEPTH, MAX_INSTRUCTIONS, UNKNOWN_DEGREE,
};
pub use rpn_eval::{eval, DualStack, EvalFault, MAX_STACK};
pub use rpn_eval2::{eval2, HyperStack, MAX_HYPER_STACK};
pub use solver::{solve, SolveFault, SolveOutcome};

#[cfg(test)]
mod dual2_tests;
#[cfg(test)]
mod dual_tests;
#[cfg(test)]
mod rpn_tests;
#[cfg(test)]
mod solver_tests;
