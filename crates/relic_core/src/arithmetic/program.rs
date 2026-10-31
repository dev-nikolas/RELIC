//! # RpnProgram
//!
//! Programa RPN de tamaño fijo (584 bytes). Toda la instrucción vive en pila:
//! `opcodes` (1 byte por operación) + `imms` (8 bytes por literal).

pub const MAX_INSTRUCTIONS: usize = 64;
pub const MAX_DEPTH: usize = 32;
/// Cota superior desconocida: no demostrable afín/cuadrático.
pub const UNKNOWN_DEGREE: u16 = u16::MAX;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DiffOp {
    PushConst = 0,
    PushX = 1,
    Add = 2,
    Sub = 3,
    Mul = 4,
    Div = 5,
    Pow = 6,
    Neg = 7,
    Sin = 8,
    Cos = 9,
    Tan = 10,
    Exp = 11,
    Ln = 12,
    Sqrt = 13,
    Abs = 14,
}

impl DiffOp {
    #[inline]
    pub const fn arity(self) -> u8 {
        match self {
            Self::PushConst | Self::PushX => 0,
            Self::Add | Self::Sub | Self::Mul | Self::Div | Self::Pow => 2,
            _ => 1,
        }
    }

    #[inline]
    pub const fn is_transcendental(self) -> bool {
        matches!(
            self,
            Self::Sin | Self::Cos | Self::Tan | Self::Exp | Self::Ln | Self::Sqrt
        )
    }
}

#[repr(C, align(8))]
#[derive(Clone)]
pub struct RpnProgram {
    pub opcodes: [DiffOp; MAX_INSTRUCTIONS],
    pub imms: [f64; MAX_INSTRUCTIONS],
    pub len: u8,
    pub _pad: [u8; 7],
}

const _: () = assert!(core::mem::size_of::<DiffOp>() == 1);
const _: () = assert!(core::mem::size_of::<RpnProgram>() == 584);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramFault {
    Full,
    StackUnderflow,
    StackOverflow,
    LeftoverStack,
}

impl RpnProgram {
    pub const fn new() -> Self {
        Self {
            opcodes: [DiffOp::PushConst; MAX_INSTRUCTIONS],
            imms: [0.0; MAX_INSTRUCTIONS],
            len: 0,
            _pad: [0u8; 7],
        }
    }

    pub fn push_const(&mut self, value: f64) -> Result<(), ProgramFault> {
        let i = self.len as usize;
        if i >= MAX_INSTRUCTIONS {
            return Err(ProgramFault::Full);
        }
        self.opcodes[i] = DiffOp::PushConst;
        self.imms[i] = value;
        self.len += 1;
        Ok(())
    }

    pub fn push_op(&mut self, op: DiffOp) -> Result<(), ProgramFault> {
        let i = self.len as usize;
        if i >= MAX_INSTRUCTIONS {
            return Err(ProgramFault::Full);
        }
        self.opcodes[i] = op;
        self.imms[i] = 0.0;
        self.len += 1;
        Ok(())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn has_variable(&self) -> bool {
        let n = self.len as usize;
        n <= MAX_INSTRUCTIONS && self.opcodes[..n].contains(&DiffOp::PushX)
    }

    /// Verifica la aritmética de pila del programa (sin evaluar).
    ///
    /// `len` es un campo público, así que se comprueba antes de cortar el
    /// slice: un `RpnProgram` manipulado a mano devuelve `Full`, no un panic.
    pub fn validate(&self) -> Result<(), ProgramFault> {
        if self.len as usize > MAX_INSTRUCTIONS {
            return Err(ProgramFault::Full);
        }
        let mut depth = 0usize;
        for op in &self.opcodes[..self.len()] {
            let arity = op.arity() as usize;
            if depth < arity {
                return Err(ProgramFault::StackUnderflow);
            }
            depth = depth - arity + 1;
            if depth > MAX_DEPTH {
                return Err(ProgramFault::StackOverflow);
            }
        }
        if depth == 1 {
            Ok(())
        } else {
            Err(ProgramFault::LeftoverStack)
        }
    }
}

impl Default for RpnProgram {
    fn default() -> Self {
        Self::new()
    }
}
