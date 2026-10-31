//! # Lexer y Shunting-Yard
//!
//! Compila una expresión a `RpnProgram` con una pila de operadores de tamaño
//! fijo. No hay `Vec` en ninguna etapa.

mod scan;

use super::program::{DiffOp, ProgramFault, RpnProgram};

pub const MAX_OP_STACK: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexFault {
    Empty,
    InvalidChar,
    MalformedNumber,
    UnbalancedParen,
    TrailingOperator,
    TooDeep,
    ProgramFull,
    StackUnderflow,
    StackOverflow,
    LeftoverStack,
}

impl From<ProgramFault> for LexFault {
    fn from(f: ProgramFault) -> Self {
        match f {
            ProgramFault::Full => Self::ProgramFull,
            ProgramFault::StackUnderflow => Self::StackUnderflow,
            ProgramFault::StackOverflow => Self::StackOverflow,
            ProgramFault::LeftoverStack => Self::LeftoverStack,
        }
    }
}

const KIND_OP: u8 = 0;
const KIND_FUNC: u8 = 1;
const KIND_PAREN: u8 = 2;

const PREC_ADD: u8 = 2;
const PREC_MUL: u8 = 4;
const PREC_NEG: u8 = 5;
const PREC_POW: u8 = 6;

pub fn compile(expr: &str) -> Result<RpnProgram, LexFault> {
    let mut compiler = Compiler::new();
    scan::run(&mut compiler, expr)?;
    Ok(compiler.prog)
}

pub(crate) struct Compiler {
    prog: RpnProgram,
    ops: [DiffOp; MAX_OP_STACK],
    kinds: [u8; MAX_OP_STACK],
    sp: u8,
}

impl Compiler {
    fn new() -> Self {
        Self {
            prog: RpnProgram::new(),
            ops: [DiffOp::PushConst; MAX_OP_STACK],
            kinds: [KIND_OP; MAX_OP_STACK],
            sp: 0,
        }
    }

    pub(crate) fn emit_const(&mut self, value: f64) -> Result<(), LexFault> {
        self.prog.push_const(value)?;
        Ok(())
    }

    pub(crate) fn emit_var(&mut self) -> Result<(), LexFault> {
        self.prog.push_op(DiffOp::PushX)?;
        Ok(())
    }

    pub(crate) fn push_paren(&mut self) -> Result<(), LexFault> {
        self.push(KIND_PAREN, DiffOp::PushConst)
    }

    pub(crate) fn push_prefix(&mut self) -> Result<(), LexFault> {
        self.push(KIND_OP, DiffOp::Neg)
    }

    pub(crate) fn push_function(&mut self, op: DiffOp) -> Result<(), LexFault> {
        self.push(KIND_FUNC, op)
    }

    pub(crate) fn push_binary(&mut self, op: DiffOp, prec: u8, right: bool) -> Result<(), LexFault> {
        self.reduce(prec, right)?;
        self.push(KIND_OP, op)
    }

    pub(crate) fn close_paren(&mut self) -> Result<(), LexFault> {
        while self.sp > 0 {
            let i = (self.sp - 1) as usize;
            if self.kinds[i] == KIND_PAREN {
                break;
            }
            self.pop_emit()?;
        }
        if self.sp == 0 {
            return Err(LexFault::UnbalancedParen);
        }
        self.sp -= 1;
        if self.sp > 0 && self.kinds[(self.sp - 1) as usize] == KIND_FUNC {
            self.pop_emit()?;
        }
        Ok(())
    }

    pub(crate) fn finish(&mut self) -> Result<(), LexFault> {
        self.reduce(0, false)?;
        if self.sp != 0 {
            return Err(LexFault::UnbalancedParen);
        }
        self.prog.validate()?;
        Ok(())
    }

    fn push(&mut self, kind: u8, op: DiffOp) -> Result<(), LexFault> {
        if self.sp as usize >= MAX_OP_STACK {
            return Err(LexFault::TooDeep);
        }
        let i = self.sp as usize;
        self.ops[i] = op;
        self.kinds[i] = kind;
        self.sp += 1;
        Ok(())
    }

    fn reduce(&mut self, prec: u8, right_assoc: bool) -> Result<(), LexFault> {
        while self.sp > 0 {
            let i = (self.sp - 1) as usize;
            if self.kinds[i] != KIND_OP {
                break;
            }
            let p = precedence(self.ops[i]);
            if p > prec || (!right_assoc && p == prec) {
                self.pop_emit()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn pop_emit(&mut self) -> Result<(), LexFault> {
        if self.sp == 0 {
            return Err(LexFault::UnbalancedParen);
        }
        self.sp -= 1;
        let i = self.sp as usize;
        if self.kinds[i] != KIND_PAREN {
            self.prog.push_op(self.ops[i])?;
        }
        Ok(())
    }
}

fn precedence(op: DiffOp) -> u8 {
    match op {
        DiffOp::Add | DiffOp::Sub => PREC_ADD,
        DiffOp::Mul | DiffOp::Div => PREC_MUL,
        DiffOp::Neg => PREC_NEG,
        DiffOp::Pow => PREC_POW,
        _ => 0,
    }
}
