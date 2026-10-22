//! # ArithmeticEngine
//!
//! Motor de razonamiento aritmético determinista y cuantitativo para RELIC.
//! Ejecuta operaciones matemáticas con tipos nativos, comprobación de desbordamiento
//! y soporte para inventarios de unidades (singular/plural).

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct ArithmeticPlan {
    pub initial: f64,
    pub steps: Vec<(ArithmeticOp, f64)>,
    pub unit: Option<String>,
}

impl ArithmeticPlan {
    pub fn new(initial: f64, unit: Option<String>) -> Self {
        Self {
            initial,
            steps: Vec::new(),
            unit,
        }
    }

    pub fn add_step(&mut self, op: ArithmeticOp, val: f64) {
        self.steps.push((op, val));
    }

    /// Ejecuta el plan aritmético completo de forma determinista y segura.
    pub fn execute(&self) -> Result<f64, ArithmeticError> {
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
            // Concordancia singular/plural básica
            if (val - 1.0).abs() < 1e-6 && u_clean.ends_with('s') {
                let singular = &u_clean[..u_clean.len() - 1];
                format!("{} {}", num_str, singular)
            } else {
                format!("{} {}", num_str, u_clean)
            }
        } else {
            num_str
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_addition() {
        let mut plan = ArithmeticPlan::new(1.0, None);
        plan.add_step(ArithmeticOp::Add, 1.0);
        assert_eq!(plan.execute(), Ok(2.0));
        assert_eq!(plan.format_result(2.0), "2");
    }

    #[test]
    fn test_sequential_operations_with_unit() {
        let mut plan = ArithmeticPlan::new(2.0, Some("manzanas".to_string()));
        plan.add_step(ArithmeticOp::Add, 1.0);
        plan.add_step(ArithmeticOp::Subtract, 2.0);
        let res = plan.execute().unwrap();
        assert_eq!(res, 1.0);
        assert_eq!(plan.format_result(res), "1 manzana");
    }

    #[test]
    fn test_division_by_zero() {
        let mut plan = ArithmeticPlan::new(10.0, None);
        plan.add_step(ArithmeticOp::Divide, 0.0);
        assert_eq!(plan.execute(), Err(ArithmeticError::DivisionByZero));
    }

    #[test]
    fn test_decimals_and_negatives() {
        let mut plan = ArithmeticPlan::new(-5.0, None);
        plan.add_step(ArithmeticOp::Add, 3.0);
        assert_eq!(plan.execute(), Ok(-2.0));

        let mut p2 = ArithmeticPlan::new(2.5, None);
        p2.add_step(ArithmeticOp::Add, 1.5);
        assert_eq!(p2.execute(), Ok(4.0));
        assert_eq!(p2.format_result(4.0), "4");
    }
}
