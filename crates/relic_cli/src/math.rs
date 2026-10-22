//! Analizador sintáctico de expresiones matemáticas puras.
//! Aislamiento estructural estricto de operadores sin listas léxicas ad-hoc.
//! Menos de 180 líneas.

use relic_core::{ArithmeticOp, ArithmeticPlan};

pub fn parse_math_expression(text: &str) -> Option<ArithmeticPlan> {
    let clean = text.trim_matches(|c| c == '¿' || c == '?' || c == '¡' || c == '!' || c == '.');
    let lower = clean.to_lowercase();

    let mut expr = clean;
    for prefix in ["¿cuánto es ", "cuánto es ", "cuanto es ", "¿cuanto es ", "¿cuánto da ", "cuánto da ", "calcula ", "resultado de "] {
        if let Some(idx) = lower.find(prefix) {
            expr = &clean[idx + prefix.len()..];
            break;
        }
    }
    let expr = expr.trim();

    let mut tokens = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            if !cur.is_empty() { tokens.push(cur.clone()); cur.clear(); }
        } else if matches!(c, '+' | '*' | '/') {
            if !cur.is_empty() { tokens.push(cur.clone()); cur.clear(); }
            tokens.push(c.to_string());
        } else if c == 'x' || c == 'X' {
            let prev_is_alpha = if i > 0 { chars[i - 1].is_alphabetic() } else { false };
            let next_is_alpha = if i + 1 < chars.len() { chars[i + 1].is_alphabetic() } else { false };
            if prev_is_alpha || next_is_alpha { return None; }
            if !cur.is_empty() { tokens.push(cur.clone()); cur.clear(); }
            tokens.push(c.to_string());
        } else if c == '-' {
            let is_unary = cur.is_empty() && (tokens.is_empty() || matches!(tokens.last().map(|s| s.as_str()), Some("+") | Some("-") | Some("*") | Some("/")));
            if is_unary {
                cur.push('-');
            } else {
                if !cur.is_empty() { tokens.push(cur.clone()); cur.clear(); }
                tokens.push("-".to_string());
            }
        } else if c.is_numeric() || c == '.' {
            cur.push(c);
        } else {
            return None;
        }
        i += 1;
    }
    if !cur.is_empty() { tokens.push(cur); }
    if tokens.len() < 3 { return None; }

    let initial: f64 = tokens[0].parse().ok()?;
    let mut plan = ArithmeticPlan::new(initial, None);

    let mut j = 1;
    while j + 1 < tokens.len() {
        let op_str = &tokens[j];
        let val: f64 = tokens[j + 1].parse().ok()?;
        let op = match op_str.as_str() {
            "+" => ArithmeticOp::Add,
            "-" => ArithmeticOp::Subtract,
            "*" | "x" | "X" => ArithmeticOp::Multiply,
            "/" => ArithmeticOp::Divide,
            _ => return None,
        };
        plan.add_step(op, val);
        j += 2;
    }

    if !plan.steps.is_empty() { Some(plan) } else { None }
}
