//! # Poly2
//!
//! Álgebra de polinomios de grado ≤ 2 (24 B) para extraer los coeficientes
//! exactos de un programa clasificado como afín o cuadrático. Cualquier
//! operación que exceda el grado 2 devuelve un fallo, nunca un truncamiento
//! silencioso.

use super::dual::{DomainFault, EPS_SINGULAR};
use super::program::{DiffOp, RpnProgram, MAX_DEPTH};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Poly2 {
    pub c0: f64,
    pub c1: f64,
    pub c2: f64,
}

const _: () = assert!(core::mem::size_of::<Poly2>() == 24);
const _: () = assert!(core::mem::align_of::<Poly2>() == 8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyFault {
    StackUnderflow,
    StackOverflow,
    LeftoverStack,
    DegreeExceeded,
    NonConstantDivisor,
    NonPolynomial,
    Malformed,
    Domain(DomainFault),
}

pub type PolyResult = Result<Poly2, PolyFault>;

impl Poly2 {
    pub const ZERO: Self = Self { c0: 0.0, c1: 0.0, c2: 0.0 };
    pub const ONE: Self = Self { c0: 1.0, c1: 0.0, c2: 0.0 };
    pub const X: Self = Self { c0: 0.0, c1: 1.0, c2: 0.0 };

    #[inline]
    pub const fn constant(v: f64) -> Self {
        Self { c0: v, c1: 0.0, c2: 0.0 }
    }
    #[inline]
    pub fn degree(self) -> u8 {
        if self.c2 != 0.0 { 2 } else if self.c1 != 0.0 { 1 } else { 0 }
    }
    /// Máximo |coeficiente|: factor de escala que nunca forma `b²`.
    #[inline]
    pub fn scale(self) -> f64 {
        self.c0.abs().max(self.c1.abs()).max(self.c2.abs())
    }
    /// `f(x)` por Horner: `(c2·x + c1)·x + c0`.
    #[inline]
    pub fn eval(self, x: f64) -> f64 {
        (self.c2 * x + self.c1) * x + self.c0
    }

    #[inline]
    fn finite(self) -> PolyResult {
        if self.c0.is_finite() && self.c1.is_finite() && self.c2.is_finite() {
            Ok(self)
        } else {
            Err(PolyFault::Domain(DomainFault::NonFinite))
        }
    }

    pub fn add(self, o: Self) -> PolyResult { Self { c0: self.c0 + o.c0, c1: self.c1 + o.c1, c2: self.c2 + o.c2 }.finite() }

    pub fn sub(self, o: Self) -> PolyResult { Self { c0: self.c0 - o.c0, c1: self.c1 - o.c1, c2: self.c2 - o.c2 }.finite() }

    pub fn neg(self) -> PolyResult { Self { c0: -self.c0, c1: -self.c1, c2: -self.c2 }.finite() }

    pub fn mul(self, o: Self) -> PolyResult {
        let c3 = self.c1 * o.c2 + self.c2 * o.c1;
        let c4 = self.c2 * o.c2;
        if c3 != 0.0 || c4 != 0.0 {
            return Err(PolyFault::DegreeExceeded);
        }
        Self {
            c0: self.c0 * o.c0,
            c1: self.c0 * o.c1 + self.c1 * o.c0,
            c2: self.c0 * o.c2 + self.c1 * o.c1 + self.c2 * o.c0,
        }
        .finite()
    }

    pub fn div(self, o: Self) -> PolyResult {
        if o.c1 != 0.0 || o.c2 != 0.0 {
            return Err(PolyFault::NonConstantDivisor);
        }
        if o.c0.abs() < EPS_SINGULAR {
            return Err(PolyFault::Domain(DomainFault::SingularDenominator));
        }
        Self { c0: self.c0 / o.c0, c1: self.c1 / o.c0, c2: self.c2 / o.c0 }.finite()
    }

    pub fn pow(self, o: Self) -> PolyResult {
        if o.c1 != 0.0 || o.c2 != 0.0 {
            return Err(PolyFault::NonPolynomial);
        }
        let exp = o.c0;
        if exp.fract() != 0.0 {
            return Err(PolyFault::NonPolynomial);
        }
        if self.c1 == 0.0 && self.c2 == 0.0 {
            if exp == 0.0 && self.c0 == 0.0 {
                return Err(PolyFault::Domain(DomainFault::Indeterminate00));
            }
            return Self::constant(self.c0.powf(exp)).finite();
        }
        if exp == 0.0 {
            Ok(Self::ONE)
        } else if exp == 1.0 {
            Ok(self)
        } else if exp == 2.0 {
            self.mul(self)
        } else {
            Err(PolyFault::DegreeExceeded)
        }
    }
}

pub fn extract(program: &RpnProgram) -> Result<Poly2, PolyFault> {
    program.validate().map_err(|_| PolyFault::Malformed)?;
    let mut st = [Poly2::ZERO; MAX_DEPTH];
    let mut sp = 0usize;
    for i in 0..program.len() {
        match program.opcodes[i] {
            DiffOp::PushConst => {
                if sp >= MAX_DEPTH {
                    return Err(PolyFault::StackOverflow);
                }
                st[sp] = Poly2::constant(program.imms[i]);
                sp += 1;
            }
            DiffOp::PushX => {
                if sp >= MAX_DEPTH {
                    return Err(PolyFault::StackOverflow);
                }
                st[sp] = Poly2::X;
                sp += 1;
            }
            DiffOp::Neg => {
                if sp == 0 {
                    return Err(PolyFault::StackUnderflow);
                }
                st[sp - 1] = st[sp - 1].neg()?;
            }
            DiffOp::Add => binary(&mut st, &mut sp, Poly2::add)?,
            DiffOp::Sub => binary(&mut st, &mut sp, Poly2::sub)?,
            DiffOp::Mul => binary(&mut st, &mut sp, Poly2::mul)?,
            DiffOp::Div => binary(&mut st, &mut sp, Poly2::div)?,
            DiffOp::Pow => binary(&mut st, &mut sp, Poly2::pow)?,
            _ => return Err(PolyFault::NonPolynomial),
        }
    }
    match sp {
        0 => Err(PolyFault::StackUnderflow),
        1 => Ok(st[0]),
        _ => Err(PolyFault::LeftoverStack),
    }
}

#[inline]
fn binary(
    st: &mut [Poly2; MAX_DEPTH],
    sp: &mut usize,
    f: fn(Poly2, Poly2) -> PolyResult,
) -> Result<(), PolyFault> {
    if *sp < 2 {
        return Err(PolyFault::StackUnderflow);
    }
    let rhs = st[*sp - 1];
    let lhs = st[*sp - 2];
    st[*sp - 2] = f(lhs, rhs)?;
    *sp -= 1;
    Ok(())
}
