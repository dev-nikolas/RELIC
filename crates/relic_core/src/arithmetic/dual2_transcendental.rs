//! # Funciones hyperduales
//!
//! Segundas derivadas de las funciones elementales. Cada una rechaza su
//! dominio inválido con `DomainFault`; ninguna propaga `NaN`/`inf`.

use super::dual::{DomainFault, EPS_SINGULAR};
use super::dual2::{HyperDual, HyperResult};

impl HyperDual {
    #[inline]
    pub fn sin(self) -> HyperResult {
        let (s, c) = (self.real.sin(), self.real.cos());
        Self::new(s, c * self.d1, -s * self.d1 * self.d1 + c * self.d2).checked()
    }

    #[inline]
    pub fn cos(self) -> HyperResult {
        let (s, c) = (self.real.sin(), self.real.cos());
        Self::new(c, -s * self.d1, -c * self.d1 * self.d1 - s * self.d2).checked()
    }

    #[inline]
    pub fn tan(self) -> HyperResult {
        let c = self.real.cos();
        if c.abs() < EPS_SINGULAR {
            return Err(DomainFault::TanPole);
        }
        let t = self.real.tan();
        let s = 1.0 + t * t;
        Self::new(t, s * self.d1, 2.0 * t * s * self.d1 * self.d1 + s * self.d2).checked()
    }

    #[inline]
    pub fn exp(self) -> HyperResult {
        let f = self.real.exp();
        if !f.is_finite() {
            return Err(DomainFault::Overflow);
        }
        Self::new(f, f * self.d1, f * (self.d1 * self.d1 + self.d2)).checked()
    }

    #[inline]
    pub fn ln(self) -> HyperResult {
        let a = self.real;
        if a <= 0.0 {
            return Err(DomainFault::Ln);
        }
        Self::new(a.ln(), self.d1 / a, self.d2 / a - self.d1 * self.d1 / (a * a)).checked()
    }

    #[inline]
    pub fn sqrt(self) -> HyperResult {
        let a = self.real;
        if a < 0.0 {
            return Err(DomainFault::Sqrt);
        }
        if a == 0.0 {
            return if self.d1 == 0.0 && self.d2 == 0.0 {
                Ok(Self::constant(0.0))
            } else {
                Err(DomainFault::SingularDerivative)
            };
        }
        let s = a.sqrt();
        let f1 = 0.5 / s;
        Self::new(s, f1 * self.d1, f1 * self.d2 - 0.25 * self.d1 * self.d1 / (a * s)).checked()
    }

    #[inline]
    pub fn abs(self) -> HyperResult {
        if self.real == 0.0 {
            return if self.d1 == 0.0 && self.d2 == 0.0 {
                Ok(Self::constant(0.0))
            } else {
                Err(DomainFault::NonDifferentiable)
            };
        }
        let sign = if self.real > 0.0 { 1.0 } else { -1.0 };
        Self::new(self.real.abs(), sign * self.d1, sign * self.d2).checked()
    }
}
