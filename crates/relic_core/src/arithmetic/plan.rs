//! # ArithmeticEngine
//!
//! Motor de razonamiento aritmético determinista y cuantitativo para RELIC.
//! Ejecuta operaciones matemáticas con tipos nativos, comprobación de desbordamiento
//! y soporte para inventarios de unidades (singular/plural) y trazas RPN exactas.
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Error, PartialEq)]
pub enum ArithmeticError {
    #[error("División por cero no permitida")]
    DivisionByZero,
    #[error("Desbordamiento numérico detectado")]
    Overflow,
    #[error("Valor numérico no válido (NaN o Infinito)")]
    InvalidNumber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArithmeticReduction {
    pub step_index: usize,
    pub left: f64,
    pub op: ArithmeticOp,
    pub right: f64,
    pub result: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArithmeticPlan {
    pub initial: f64,
    pub steps: Vec<(ArithmeticOp, f64)>,
    pub unit: Option<String>,
    pub reductions: Vec<ArithmeticReduction>,
    pub exact_division_note: Option<String>,
}

impl ArithmeticPlan {
    pub fn new(initial: f64, unit: Option<String>) -> Self {
        Self { initial, steps: Vec::new(), unit, reductions: Vec::new(), exact_division_note: None }
    }

    pub fn add_step(&mut self, op: ArithmeticOp, val: f64) {
        self.steps.push((op, val));
    }

    pub fn add_reduction(&mut self, left: f64, op: ArithmeticOp, right: f64, result: f64) {
        let step_index = self.reductions.len() + 1;
        self.reductions.push(ArithmeticReduction { step_index, left, op, right, result });
    }

    /// Ejecuta el plan aritmético completo de forma determinista y segura.
    pub fn execute(&self) -> Result<f64, ArithmeticError> {
        if let Some(last) = self.reductions.last() {
            if last.result.is_nan() || last.result.is_infinite() {
                return Err(ArithmeticError::InvalidNumber);
            }
            return Ok(last.result);
        }
        let mut current = self.initial;
        if current.is_nan() || current.is_infinite() {
            return Err(ArithmeticError::InvalidNumber);
        }
        for &(op, val) in &self.steps {
            if val.is_nan() || val.is_infinite() {
                return Err(ArithmeticError::InvalidNumber);
            }
            match op {
                ArithmeticOp::Add => current += val,
                ArithmeticOp::Subtract => current -= val,
                ArithmeticOp::Multiply => current *= val,
                ArithmeticOp::Divide => {
                    if val.abs() < 1e-12 {
                        return Err(ArithmeticError::DivisionByZero);
                    }
                    current /= val;
                }
            }
            if current.is_nan() || current.is_infinite() {
                return Err(ArithmeticError::Overflow);
            }
        }
        Ok(current)
    }

    /// Formatea el resultado respetando enteros, decimales y concordancia de unidad.
    pub fn format_result(&self, val: f64) -> String {
        let is_integer = (val.fract()).abs() < 1e-6;
        let num_str = if is_integer && val >= (i64::MIN as f64) && val <= (i64::MAX as f64) {
            format!("{}", val as i64)
        } else if is_integer {
            format!("{:.0}", val)
        } else {
            format!("{:.2}", val)
        };
        if let Some(ref u) = self.unit {
            let u_clean = u.trim();
            if (val - 1.0).abs() < 1e-6 && u_clean.ends_with('s') {
                format!("{} {}", num_str, &u_clean[..u_clean.len() - 1])
            } else {
                format!("{} {}", num_str, u_clean)
            }
        } else {
            num_str
        }
    }

    /// Genera la representación textual de la fórmula interpretada para transparencia.
    pub fn format_formula(&self) -> String {
        let fmt_n = |n: f64| -> String {
            if (n.fract()).abs() < 1e-6 { format!("{}", n.round() as i64) } else { format!("{:.2}", n) }
        };
        let op_sym = |op: ArithmeticOp| match op {
            ArithmeticOp::Add => "+",
            ArithmeticOp::Subtract => "-",
            ArithmeticOp::Multiply => "*",
            ArithmeticOp::Divide => "/",
        };
        if !self.reductions.is_empty() {
            let parts: Vec<String> = self.reductions.iter().map(|r| {
                format!("{} {} {} = {}", fmt_n(r.left), op_sym(r.op), fmt_n(r.right), fmt_n(r.result))
            }).collect();
            let base = parts.join("; ");
            return match self.exact_division_note {
                Some(ref note) => format!("{base} ({note})"),
                None => base,
            };
        }
        let mut s = fmt_n(self.initial);
        if let Some(ref u) = self.unit { s = format!("{} {}", s, u); }
        for &(op, val) in &self.steps {
            let sym = op_sym(op);
            if let Some(ref u) = self.unit {
                s.push_str(&format!(" {} {} {}", sym, fmt_n(val), u));
            } else {
                s.push_str(&format!(" {} {}", sym, fmt_n(val)));
            }
        }
        if let Some(ref note) = self.exact_division_note {
            s.push_str(&format!(" ({note})"));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic_plan_operations_and_div_zero() {
        let mut p1 = ArithmeticPlan::new(2.0, Some("manzanas".to_string()));
        p1.add_step(ArithmeticOp::Add, 1.0);
        p1.add_step(ArithmeticOp::Subtract, 2.0);
        assert_eq!(p1.format_result(p1.execute().unwrap()), "1 manzana");

        let mut p_div0 = ArithmeticPlan::new(10.0, None);
        p_div0.add_step(ArithmeticOp::Divide, 0.0);
        assert_eq!(p_div0.execute(), Err(ArithmeticError::DivisionByZero));
    }
}
