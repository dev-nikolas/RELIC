//! # Evaluador RPN dual
//!
//! Pila de 32 valores duales (528 bytes) en la pila del hilo. Cada instrucción
//! produce un `DualNumber` o una abstención formal; nunca un `NaN` silencioso.

use super::dual::{DomainFault, DualNumber, DualResult};
use super::program::{DiffOp, RpnProgram, MAX_INSTRUCTIONS};

pub const MAX_STACK: usize = 32;

#[repr(C, align(16))]
pub struct DualStack {
    pub buf: [DualNumber; MAX_STACK],
    pub sp: u32,
    pub _pad: [u8; 12],
}

const _: () = assert!(core::mem::size_of::<DualStack>() == 528);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalFault {
    StackUnderflow,
    StackOverflow,
    LeftoverStack,
    MalformedProgram,
    Domain(DomainFault),
}

impl DualStack {
    pub const fn new() -> Self {
        Self {
            buf: [DualNumber::constant(0.0); MAX_STACK],
            sp: 0,
            _pad: [0u8; 12],
        }
    }

    #[inline]
    fn push(&mut self, value: DualNumber) -> Result<(), EvalFault> {
        let i = self.sp as usize;
        if i >= MAX_STACK {
            return Err(EvalFault::StackOverflow);
        }
        self.buf[i] = value;
        self.sp += 1;
        Ok(())
    }

    #[inline]
    fn pop(&mut self) -> Result<DualNumber, EvalFault> {
        if self.sp == 0 {
            return Err(EvalFault::StackUnderflow);
        }
        self.sp -= 1;
        Ok(self.buf[self.sp as usize])
    }

    fn finish(self) -> Result<DualNumber, EvalFault> {
        match self.sp {
            0 => Err(EvalFault::StackUnderflow),
            1 => Ok(self.buf[0]),
            _ => Err(EvalFault::LeftoverStack),
        }
    }
}

/// Evalúa `program` en `x`. Si el programa declara más instrucciones de las
/// que puede contener, se rechaza en vez de leer fuera de rango.
pub fn eval(program: &RpnProgram, x: DualNumber) -> Result<DualNumber, EvalFault> {
    if program.len() > MAX_INSTRUCTIONS {
        return Err(EvalFault::MalformedProgram);
    }
    let mut st = DualStack::new();

    for i in 0..program.len() {
        match program.opcodes[i] {
            DiffOp::PushConst => st.push(DualNumber::constant(program.imms[i]))?,
            DiffOp::PushX => st.push(x)?,
            DiffOp::Neg => {
                let a = st.pop()?;
                st.push(-a)?;
            }
            DiffOp::Add => binary(&mut st, DualNumber::checked_add)?,
            DiffOp::Sub => binary(&mut st, DualNumber::checked_sub)?,
            DiffOp::Mul => binary(&mut st, DualNumber::checked_mul)?,
            DiffOp::Div => binary(&mut st, DualNumber::checked_div)?,
            DiffOp::Pow => binary(&mut st, DualNumber::checked_pow)?,
            DiffOp::Sin => unary(&mut st, DualNumber::sin)?,
            DiffOp::Cos => unary(&mut st, DualNumber::cos)?,
            DiffOp::Tan => unary(&mut st, DualNumber::tan)?,
            DiffOp::Exp => unary(&mut st, DualNumber::exp)?,
            DiffOp::Ln => unary(&mut st, DualNumber::ln)?,
            DiffOp::Sqrt => unary(&mut st, DualNumber::sqrt)?,
            DiffOp::Abs => unary(&mut st, DualNumber::abs)?,
        }
    }

    st.finish()
}

#[inline]
fn binary(st: &mut DualStack, f: fn(DualNumber, DualNumber) -> DualResult) -> Result<(), EvalFault> {
    let rhs = st.pop()?;
    let lhs = st.pop()?;
    st.push(f(lhs, rhs).map_err(EvalFault::Domain)?)
}

#[inline]
fn unary(st: &mut DualStack, f: fn(DualNumber) -> DualResult) -> Result<(), EvalFault> {
    let a = st.pop()?;
    st.push(f(a).map_err(EvalFault::Domain)?)
}
