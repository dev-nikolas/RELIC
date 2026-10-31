//! # Certificación de raíces
//!
//! Una raíz solo se devuelve si `residual ≤ CERT_TOL`. Hay dos criterios y el
//! informe dice cuál se usó, porque no son equivalentes:
//!
//! - `Absolute`: `|f(x)| ≤ 1e-12` con los coeficientes originales.
//! - `Relative`: `|f(x)| ≤ 1e-12 · Σ|aᵢ||x|ⁱ`. Es el error backward estándar y
//!   es el único criterio posible cuando evaluar `f` en `x` desborda (por
//!   ejemplo `x = 1e200` en un programa que forma `x²`).
//!
//! `residual` guarda siempre la métrica normalizada, así que en ambos casos
//! `residual ≤ 1e-12` significa "certificada".

use super::dual::EPS_SINGULAR;
use super::newton::CERT_TOL;
use super::poly2::Poly2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Certification {
    Absolute,
    Relative,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Certified {
    pub root: f64,
    /// Métrica de certificación ya normalizada: `|f|` o `|f|/escala`.
    pub residual: f64,
    pub criterion: Certification,
    /// 1 = simple; 2 = doble (`f'≈0`, `f''≠0`); 0 = mayor o indeterminada.
    pub multiplicity: u8,
    pub iters: u8,
}

const _: () = assert!(core::mem::size_of::<Certified>() == 24);

impl Certified {
    #[inline]
    pub const fn is_simple(&self) -> bool {
        self.multiplicity == 1
    }
}

/// Multiplicidad estructural a partir de las dos primeras derivadas.
#[inline]
pub fn multiplicity(slope: f64, curvature: f64) -> u8 {
    if slope.abs() > EPS_SINGULAR {
        1
    } else if curvature.abs() > EPS_SINGULAR {
        2
    } else {
        0
    }
}

/// Certifica una raíz de un polinomio de grado ≤ 2.
pub fn poly(p: Poly2, root: f64, mult: u8, iters: u8) -> Result<Certified, ()> {
    if !root.is_finite() {
        return Err(());
    }
    let direct = p.eval(root);
    if direct.is_finite() && direct.abs() <= CERT_TOL {
        return Ok(Certified {
            root,
            residual: direct.abs(),
            criterion: Certification::Absolute,
            multiplicity: mult,
            iters,
        });
    }
    let relative = relative_at(p, root).ok_or(())?;
    if relative <= CERT_TOL {
        Ok(Certified {
            root,
            residual: relative,
            criterion: Certification::Relative,
            multiplicity: mult,
            iters,
        })
    } else {
        Err(())
    }
}

/// `|f(r)| / Σ|aᵢ||r|ⁱ` con los coeficientes normalizados por `max|aᵢ|`.
/// Si la cota desborda, el residuo es despreciable frente a los términos y se
/// devuelve `0.0` (el cociente real está por debajo de `f64::MIN_POSITIVE`).
fn relative_at(p: Poly2, root: f64) -> Option<f64> {
    let scale = p.scale();
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let (a, b, c) = (p.c2 / scale, p.c1 / scale, p.c0 / scale);
    let value = a * root * root + b * root + c;
    if !value.is_finite() {
        return None;
    }
    let bound = a.abs() * root * root + b.abs() * root.abs() + c.abs();
    if !bound.is_finite() {
        return Some(0.0);
    }
    if bound <= 0.0 {
        return None;
    }
    Some(value.abs() / bound)
}

/// Certifica una raíz del programa completo a partir de su sonda
/// `(f, f', f'')` en `root`.
pub fn probe(root: f64, value: f64, slope: f64, curvature: f64, iters: u8) -> Result<Certified, ()> {
    if !root.is_finite() || !value.is_finite() {
        return Err(());
    }
    let mult = multiplicity(slope, curvature);
    if value.abs() <= CERT_TOL {
        return Ok(Certified {
            root,
            residual: value.abs(),
            criterion: Certification::Absolute,
            multiplicity: mult,
            iters,
        });
    }
    let bound = value.abs() + (root * slope).abs();
    if !bound.is_finite() || bound <= 0.0 {
        return Err(());
    }
    let relative = value.abs() / bound;
    if relative <= CERT_TOL {
        Ok(Certified {
            root,
            residual: relative,
            criterion: Certification::Relative,
            multiplicity: mult,
            iters,
        })
    } else {
        Err(())
    }
}
