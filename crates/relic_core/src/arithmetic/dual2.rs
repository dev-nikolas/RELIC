//! # HyperDual
//!
//! Segundo orden forward en `R[ε₁, ε₂]/(ε₁² = ε₁ε₂ = ε₂² = 0)`:
//! `(f, f', f'')` en 24 bytes, siempre en la pila.

use super::dual::{DomainFault, EPS_SINGULAR};

/// `(real, d1, d2)` = `(f, f', f'')`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HyperDual {
    pub real: f64,
    pub d1: f64,
    pub d2: f64,
}

const _: () = assert!(core::mem::size_of::<HyperDual>() == 24);
const _: () = assert!(core::mem::align_of::<HyperDual>() == 8);

pub type HyperResult = Result<HyperDual, DomainFault>;

impl HyperDual {
    #[inline]
    pub const fn new(real: f64, d1: f64, d2: f64) -> Self {
        Self { real, d1, d2 }
    }

    #[inline]
    pub const fn constant(value: f64) -> Self {
        Self { real: value, d1: 0.0, d2: 0.0 }
    }

    #[inline]
    pub const fn variable(value: f64) -> Self {
        Self { real: value, d1: 1.0, d2: 0.0 }
    }

    #[inline]
    pub(crate) fn checked(self) -> HyperResult {
        if self.real.is_finite() && self.d1.is_finite() && self.d2.is_finite() {
            Ok(self)
        } else {
            Err(DomainFault::NonFinite)
        }
    }

    #[inline]
    pub fn checked_add(self, rhs: Self) -> HyperResult {
        Self::new(self.real + rhs.real, self.d1 + rhs.d1, self.d2 + rhs.d2).checked()
    }

    #[inline]
    pub fn checked_sub(self, rhs: Self) -> HyperResult {
        Self::new(self.real - rhs.real, self.d1 - rhs.d1, self.d2 - rhs.d2).checked()
    }

    #[inline]
    pub fn checked_mul(self, rhs: Self) -> HyperResult {
        // (uv)'' = u''v + 2u'v' + uv''; sin FMA para ser bit-reproducible.
        let d1 = self.d1 * rhs.real + self.real * rhs.d1;
        let d2 = self.d2 * rhs.real + 2.0 * self.d1 * rhs.d1 + self.real * rhs.d2;
        Self::new(self.real * rhs.real, d1, d2).checked()
    }

    /// Cociente con escalado previo: nunca forma `b²`.
    ///
    /// Con `q = a/b`, `u = a'/b`, `v = b'/b`:
    /// `f' = u − qv`, `f'' = a''/b − 2uv + 2qv² − qb''/b`.
    #[inline]
    pub fn checked_div(self, rhs: Self) -> HyperResult {
        if rhs.real.abs() < EPS_SINGULAR {
            return Err(DomainFault::SingularDenominator);
        }
        let b = rhs.real;
        let q = self.real / b;
        let u = self.d1 / b;
        let v = rhs.d1 / b;
        let d1 = u - q * v;
        let d2 = self.d2 / b - 2.0 * u * v + 2.0 * q * v * v - q * rhs.d2 / b;
        Self::new(q, d1, d2).checked()
    }

    /// Potencia con exponente constante (`b' = b'' = 0`).
    /// Un exponente variable exige `ln(a)` y se rechaza: el solver solo lo
    /// necesita para clasificar, y abstenerse es preferible a improvisar.
    pub fn checked_pow(self, exp: Self) -> HyperResult {
        if exp.d1 != 0.0 || exp.d2 != 0.0 {
            return Err(DomainFault::Pow);
        }
        let (a, a1, a2) = (self.real, self.d1, self.d2);
        let b = exp.real;
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
                Ok(Self::new(0.0, a1, a2))
            } else if b > 1.0 {
                Ok(Self::constant(0.0))
            } else {
                Err(DomainFault::SingularDerivative)
            };
        }
        if a < 0.0 && b.fract() != 0.0 {
            return Err(DomainFault::Pow);
        }
        let f = a.powf(b);
        if !f.is_finite() {
            return Err(DomainFault::Overflow);
        }
        let f1 = b * a.powf(b - 1.0);
        let f2 = b * (b - 1.0) * a.powf(b - 2.0);
        Self::new(f, f1 * a1, f2 * a1 * a1 + f1 * a2).checked()
    }
}

impl core::ops::Neg for HyperDual {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.real, -self.d1, -self.d2)
    }
}
