//! # Clasificación estructural del programa RPN
//!
//! Calcula una cota superior sintáctica del grado y dos banderas que deciden
//! si el solver puede usar la ruta algebraica exacta. Es una inferencia sobre
//! la forma del programa, nunca una medición numérica.

use super::program::{DiffOp, ProgramFault, RpnProgram, MAX_DEPTH, UNKNOWN_DEGREE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramClass {
    pub degree_bound: u16,
    pub has_transcendental: bool,
    pub var_in_denominator: bool,
}

impl ProgramClass {
    #[inline]
    pub const fn is_linear(&self) -> bool {
        self.degree_bound == 1 && !self.has_transcendental && !self.var_in_denominator
    }

    #[inline]
    pub const fn is_quadratic(&self) -> bool {
        self.degree_bound == 2 && !self.has_transcendental && !self.var_in_denominator
    }
}

#[derive(Copy, Clone)]
struct Derivation {
    degree: u16,
    literal: Option<f64>,
}

impl RpnProgram {
    /// Cota superior de grado + banderas. `UNKNOWN_DEGREE` significa
    /// "no demostrable afín/cuadrático", no "grado infinito medido".
    pub fn class(&self) -> Result<ProgramClass, ProgramFault> {
        self.validate()?;
        let mut stack = [Derivation { degree: 0, literal: None }; MAX_DEPTH];
        let mut sp = 0usize;
        let mut has_transcendental = false;
        let mut var_in_denominator = false;

        for i in 0..self.len() {
            match self.opcodes[i] {
                DiffOp::PushConst => {
                    stack[sp] = Derivation {
                        degree: 0,
                        literal: Some(self.imms[i]),
                    };
                    sp += 1;
                }
                DiffOp::PushX => {
                    stack[sp] = Derivation { degree: 1, literal: None };
                    sp += 1;
                }
                DiffOp::Neg => {}
                DiffOp::Add | DiffOp::Sub => {
                    sp -= 1;
                    stack[sp - 1].degree = stack[sp - 1].degree.max(stack[sp].degree);
                    stack[sp - 1].literal = None;
                }
                DiffOp::Mul => {
                    sp -= 1;
                    stack[sp - 1].degree = stack[sp - 1].degree.saturating_add(stack[sp].degree);
                    stack[sp - 1].literal = None;
                }
                DiffOp::Div => {
                    sp -= 1;
                    if stack[sp].degree > 0 {
                        var_in_denominator = true;
                        stack[sp - 1].degree = UNKNOWN_DEGREE;
                    }
                    stack[sp - 1].literal = None;
                }
                DiffOp::Pow => {
                    sp -= 1;
                    stack[sp - 1].degree = match stack[sp].literal {
                        Some(v) if (0.0..=8.0).contains(&v) && v.fract() == 0.0 => {
                            stack[sp - 1].degree.saturating_mul(v as u16)
                        }
                        _ => {
                            has_transcendental = true;
                            UNKNOWN_DEGREE
                        }
                    };
                    stack[sp - 1].literal = None;
                }
                op => {
                    if op.is_transcendental() {
                        has_transcendental = true;
                    }
                    stack[sp - 1].degree = UNKNOWN_DEGREE;
                    stack[sp - 1].literal = None;
                }
            }
        }

        Ok(ProgramClass {
            degree_bound: stack[0].degree,
            has_transcendental,
            var_in_denominator,
        })
    }
}
