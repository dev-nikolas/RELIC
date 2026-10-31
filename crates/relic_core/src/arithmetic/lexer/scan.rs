//! # Escáner de tokens
//!
//! Recorrido de un solo paso sobre los bytes de la expresión. Emite
//! instrucciones al `Compiler` y delega el apilado a sus primitivas.

use super::{
    Compiler, LexFault, KIND_FUNC, KIND_OP, KIND_PAREN, PREC_ADD, PREC_MUL, PREC_NEG, PREC_POW,
};
use crate::arithmetic::program::DiffOp;

pub(super) fn run(c: &mut Compiler, src: &str) -> Result<(), LexFault> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut expect_operand = true;
    let mut saw_token = false;

    while i < b.len() {
        let ch = b[i];
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        saw_token = true;

        if ch.is_ascii_digit() || ch == b'.' {
            if !expect_operand {
                return Err(LexFault::InvalidChar);
            }
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            let v = src[start..i]
                .parse::<f64>()
                .map_err(|_| LexFault::MalformedNumber)?;
            c.emit_const(v)?;
            expect_operand = false;
            continue;
        }

        match ch {
            b'(' => {
                if !expect_operand {
                    return Err(LexFault::InvalidChar);
                }
                c.push_paren()?;
                i += 1;
            }
            b')' => {
                if expect_operand {
                    return Err(LexFault::InvalidChar);
                }
                c.close_paren()?;
                expect_operand = false;
                i += 1;
            }
            b'+' | b'-' | b'*' | b'/' | b'^' => {
                if expect_operand {
                    if ch == b'-' {
                        c.push_prefix()?;
                    } else if ch != b'+' {
                        return Err(LexFault::InvalidChar);
                    }
                    i += 1;
                    continue;
                }
                let (op, prec) = match ch {
                    b'+' => (DiffOp::Add, PREC_ADD),
                    b'-' => (DiffOp::Sub, PREC_ADD),
                    b'*' => (DiffOp::Mul, PREC_MUL),
                    b'/' => (DiffOp::Div, PREC_MUL),
                    _ => (DiffOp::Pow, PREC_POW),
                };
                c.push_binary(op, prec, ch == b'^')?;
                expect_operand = true;
                i += 1;
            }
            b'x' | b'X' => {
                if !expect_operand {
                    return Err(LexFault::InvalidChar);
                }
                c.emit_var()?;
                expect_operand = false;
                i += 1;
            }
            _ if ch.is_ascii_alphabetic() => {
                let start = i;
                while i < b.len() && b[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let op = function(&src[start..i]).ok_or(LexFault::InvalidChar)?;
                if !expect_operand {
                    return Err(LexFault::InvalidChar);
                }
                let mut j = i;
                while j < b.len() && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j >= b.len() || b[j] != b'(' {
                    return Err(LexFault::InvalidChar);
                }
                c.push_function(op)?;
                expect_operand = true;
            }
            _ => return Err(LexFault::InvalidChar),
        }
    }

    if !saw_token {
        return Err(LexFault::Empty);
    }
    if expect_operand {
        return Err(LexFault::TrailingOperator);
    }
    c.finish()
}

fn function(name: &str) -> Option<DiffOp> {
    Some(match name {
        "sin" => DiffOp::Sin,
        "cos" => DiffOp::Cos,
        "tan" => DiffOp::Tan,
        "exp" => DiffOp::Exp,
        "ln" => DiffOp::Ln,
        "sqrt" => DiffOp::Sqrt,
        "abs" => DiffOp::Abs,
        _ => return None,
    })
}

const _: () = assert!(KIND_OP < KIND_FUNC && KIND_FUNC < KIND_PAREN);
const _: () = assert!(PREC_ADD < PREC_MUL && PREC_MUL < PREC_NEG && PREC_NEG < PREC_POW);
