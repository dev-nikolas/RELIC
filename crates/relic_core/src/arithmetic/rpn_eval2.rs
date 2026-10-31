//! # Evaluador RPN hyperdual
//!
//! Pila de 32 valores `(f, f', f'')` (776 B) en la pila del hilo. Reutiliza
//! `DiffOp` y `RpnProgram` de la Fase 3; solo cambia el tipo de celda.
//!
//! La duplicación de `rpn_eval` es deliberada: unificar ambos evaluadores con
//! un trait exigiría despacho dinámico o monomorfización sobre toda la
//! superficie pública, y la Fase 4 no debe tocar el evaluador ya certificado.

use super::dual2::{HyperDual, HyperResult};
use super::program::{DiffOp, RpnProgram, MAX_INSTRUCTIONS};
use super::rpn_eval::EvalFault;

pub const MAX_HYPER_STACK: usize = 32;

#[repr(C, align(8))]
pub struct HyperStack {
    pub buf: [HyperDual; MAX_HYPER_STACK],
    pub sp: u32,
    pub _pad: [u8; 4],
}

const _: () = assert!(core::mem::size_of::<HyperStack>() == 776);
const _: () = assert!(core::mem::align_of::<HyperStack>() == 8);

impl HyperStack {
    pub const fn new() -> Self {
        Self {
            buf: [HyperDual::constant(0.0); MAX_HYPER_STACK],
            sp: 0,
            _pad: [0u8; 4],
        }
    }

    #[inline]
    fn push(&mut self, value: HyperDual) -> Result<(), EvalFault> {
        let i = self.sp as usize;
        if i >= MAX_HYPER_STACK {
            return Err(EvalFault::StackOverflow);
        }
        self.buf[i] = value;
        self.sp += 1;
        Ok(())
    }

    #[inline]
    fn pop(&mut self) -> Result<HyperDual, EvalFault> {
        if self.sp == 0 {
            return Err(EvalFault::StackUnderflow);
        }
        self.sp -= 1;
        Ok(self.buf[self.sp as usize])
    }

    fn finish(self) -> Result<HyperDual, EvalFault> {
        match self.sp {
            0 => Err(EvalFault::StackUnderflow),
            1 => Ok(self.buf[0]),
            _ => Err(EvalFault::LeftoverStack),
        }
    }
}

impl Default for HyperStack {
    fn default() -> Self {
        Self::new()
    }
}

pub fn eval2(program: &RpnProgram, x: HyperDual) -> Result<HyperDual, EvalFault> {
    if program.len() > MAX_INSTRUCTIONS {
        return Err(EvalFault::MalformedProgram);
    }
    let mut st = HyperStack::new();

    for i in 0..program.len() {
        match program.opcodes[i] {
            DiffOp::PushConst => st.push(HyperDual::constant(program.imms[i]))?,
            DiffOp::PushX => st.push(x)?,
            DiffOp::Neg => {
                let a = st.pop()?;
                st.push(-a)?;
            }
            DiffOp::Add => binary(&mut st, HyperDual::checked_add)?,
            DiffOp::Sub => binary(&mut st, HyperDual::checked_sub)?,
            DiffOp::Mul => binary(&mut st, HyperDual::checked_mul)?,
            DiffOp::Div => binary(&mut st, HyperDual::checked_div)?,
            DiffOp::Pow => binary(&mut st, HyperDual::checked_pow)?,
            DiffOp::Sin => unary(&mut st, HyperDual::sin)?,
            DiffOp::Cos => unary(&mut st, HyperDual::cos)?,
            DiffOp::Tan => unary(&mut st, HyperDual::tan)?,
            DiffOp::Exp => unary(&mut st, HyperDual::exp)?,
            DiffOp::Ln => unary(&mut st, HyperDual::ln)?,
            DiffOp::Sqrt => unary(&mut st, HyperDual::sqrt)?,
            DiffOp::Abs => unary(&mut st, HyperDual::abs)?,
        }
    }

    st.finish()
}

#[inline]
fn binary(
    st: &mut HyperStack,
    f: fn(HyperDual, HyperDual) -> HyperResult,
) -> Result<(), EvalFault> {
    let rhs = st.pop()?;
    let lhs = st.pop()?;
    st.push(f(lhs, rhs).map_err(EvalFault::Domain)?)
}

#[inline]
fn unary(st: &mut HyperStack, f: fn(HyperDual) -> HyperResult) -> Result<(), EvalFault> {
    let a = st.pop()?;
    st.push(f(a).map_err(EvalFault::Domain)?)
}
