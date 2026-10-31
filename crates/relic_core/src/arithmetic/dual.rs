//! # DualNumber
//!
//! Números duales en `D = R[ε]/(ε² = 0)` para diferenciación automática forward.
//! Un valor dual ocupa 16 bytes alineados a 16 y vive siempre en la pila.

use core::ops::{Add, Mul, Neg, Sub};

/// Umbral de singularidad idéntico al de `plan.rs` (`ArithmeticPlan::execute`).
pub const EPS_SINGULAR: f64 = 1e-12;

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DualNumber {
    pub real: f64,
    pub dual: f64,
}

const _: () = assert!(core::mem::size_of::<DualNumber>() == 16);
const _: () = assert!(core::mem::align_of::<DualNumber>() == 16);

/// Motivo formal de abstención. El motor nunca devuelve un valor envenenado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainFault {
    Ln,
    Sqrt,
    Pow,
    TanPole,
    NonDifferentiable,
    Indeterminate00,
    SingularDenominator,
    SingularDerivative,
    Overflow,
    NonFinite,
}

pub type DualResult = Result<DualNumber, DomainFault>;

impl DualNumber {
    #[inline]
    pub const fn new(real: f64, dual: f64) -> Self {
        Self { real, dual }
    }

    #[inline]
    pub const fn constant(real: f64) -> Self {
        Self { real, dual: 0.0 }
    }

    #[inline]
    pub const fn variable(real: f64) -> Self {
        Self { real, dual: 1.0 }
    }

    #[inline]
    pub(crate) fn checked(self) -> DualResult {
        if self.real.is_finite() && self.dual.is_finite() {
            Ok(self)
        } else {
            Err(DomainFault::NonFinite)
        }
    }

    #[inline]
    pub fn checked_add(self, rhs: Self) -> DualResult {
        Self::new(self.real + rhs.real, self.dual + rhs.dual).checked()
    }

    #[inline]
    pub fn checked_sub(self, rhs: Self) -> DualResult {
        Self::new(self.real - rhs.real, self.dual - rhs.dual).checked()
    }

    #[inline]
    pub fn checked_mul(self, rhs: Self) -> DualResult {
        // Sin FMA: el orden elegido es bit-reproducible en cualquier target.
        let dual = self.real * rhs.dual + self.dual * rhs.real;
        Self::new(self.real * rhs.real, dual).checked()
    }

    /// Regla del cociente: `(α·b − a·β)/b² ≡ (α − q·β)/b`, con `q = a/b`.
    ///
    /// No es una variante estética: evita formar `b²`, que desborda a `inf`
    /// para `|b| ≳ 1.34e154` aun siendo `b` representable. Invertir el orden
    /// de los productos (`(β − q·α)/b`) intercambia `α` y `β` y cambia el signo.
    #[inline]
    pub fn checked_div(self, rhs: Self) -> DualResult {
        if rhs.real.abs() < EPS_SINGULAR {
            return Err(DomainFault::SingularDenominator);
        }
        let q = self.real / rhs.real;
        let dual = (self.dual - q * rhs.dual) / rhs.real;
        Self::new(q, dual).checked()
    }

    /// Potencia con tabla de casos explícita (entero constante, base positiva,
    /// base negativa, base cero). Ver matriz P1–P5 del diseño.
    pub fn checked_pow(self, exp: Self) -> DualResult {
        let (a, alpha) = (self.real, self.dual);
        let (b, beta) = (exp.real, exp.dual);

        if beta == 0.0 && b.fract() == 0.0 {
            if b == 0.0 {
                return if a == 0.0 {
                    Err(DomainFault::Indeterminate00)
                } else {
                    Ok(Self::constant(1.0))
                };
            }
            if a == 0.0 {
                return if b < 0.0 {
                    Err(DomainFault::SingularDenominator)
                } else if b == 1.0 {
                    Ok(Self::new(0.0, alpha))
                } else if b > 1.0 {
                    Ok(Self::constant(0.0))
                } else {
                    Err(DomainFault::SingularDerivative)
                };
            }
            let real = a.powf(b);
            let dual = b * a.powf(b - 1.0) * alpha;
            return Self::new(real, dual).checked();
        }

        if a <= 0.0 {
            // 0^b con 0 < b < 1: derivada lateral infinita en el origen.
            if a == 0.0 && b > 0.0 && b < 1.0 {
                return Err(DomainFault::SingularDerivative);
            }
            return Err(DomainFault::Pow);
        }
        let real = a.powf(b);
        if !real.is_finite() {
            return Err(DomainFault::Overflow);
        }
        let dual = real * (beta * a.ln() + b * alpha / a);
        Self::new(real, dual).checked()
    }
}

impl Add for DualNumber {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.real + rhs.real, self.dual + rhs.dual)
    }
}

impl Sub for DualNumber {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.real - rhs.real, self.dual - rhs.dual)
    }
}

impl Mul for DualNumber {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.real * rhs.real,
            self.real * rhs.dual + self.dual * rhs.real,
        )
    }
}

impl Neg for DualNumber {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.real, -self.dual)
    }
}
