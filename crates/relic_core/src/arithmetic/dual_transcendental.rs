//! # Funciones duales
//!
//! Propagación forward de `sin`, `cos`, `tan`, `exp`, `ln`, `sqrt` y `abs`.
//! Cada función rechaza su dominio inválido con `DomainFault`; ninguna
//! devuelve `NaN` o `inf` como si fuese un resultado.

use super::dual::{DomainFault, DualNumber, DualResult, EPS_SINGULAR};

impl DualNumber {
    #[inline]
    pub fn sin(self) -> DualResult {
        Self::new(self.real.sin(), self.dual * self.real.cos()).checked()
    }

    #[inline]
    pub fn cos(self) -> DualResult {
        Self::new(self.real.cos(), -self.dual * self.real.sin()).checked()
    }

    #[inline]
    pub fn tan(self) -> DualResult {
        let c = self.real.cos();
        if c.abs() < EPS_SINGULAR {
            return Err(DomainFault::TanPole);
        }
        let t = self.real.tan();
        Self::new(t, self.dual * (1.0 + t * t)).checked()
    }

    #[inline]
    pub fn exp(self) -> DualResult {
        let f = self.real.exp();
        if !f.is_finite() {
            return Err(DomainFault::Overflow);
        }
        Self::new(f, f * self.dual).checked()
    }

    #[inline]
    pub fn ln(self) -> DualResult {
        if self.real <= 0.0 {
            return Err(DomainFault::Ln);
        }
        Self::new(self.real.ln(), self.dual / self.real).checked()
    }

    #[inline]
    pub fn sqrt(self) -> DualResult {
        if self.real < 0.0 {
            return Err(DomainFault::Sqrt);
        }
        if self.real == 0.0 {
            return if self.dual == 0.0 {
                Ok(Self::constant(0.0))
            } else {
                Err(DomainFault::SingularDerivative)
            };
        }
        let s = self.real.sqrt();
        Self::new(s, self.dual / (2.0 * s)).checked()
    }

    #[inline]
    pub fn abs(self) -> DualResult {
        if self.real == 0.0 {
            return if self.dual == 0.0 {
                Ok(Self::constant(0.0))
            } else {
                Err(DomainFault::NonDifferentiable)
            };
        }
        let dual = if self.real > 0.0 { self.dual } else { -self.dual };
        Self::new(self.real.abs(), dual).checked()
    }
}
