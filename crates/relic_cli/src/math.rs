//! Analizador sintáctico de expresiones matemáticas puras, con precedencia y paréntesis anidados.
use relic_core::{ArithmeticOp, ArithmeticPlan, ArithmeticReduction};

#[derive(Debug, Clone, PartialEq)]
pub enum MathToken {
    Num(f64),
    Op(ArithmeticOp),
    LParen,
    RParen,
}

pub fn tokenize_math(expr: &str) -> Option<Vec<MathToken>> {
    let (mut tokens, mut cur, mut i) = (Vec::new(), String::new(), 0);
    let chars: Vec<char> = expr.chars().collect();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            if !cur.is_empty() { tokens.push(MathToken::Num(cur.parse().ok()?)); cur.clear(); }
        } else if c == '(' || c == ')' {
            if !cur.is_empty() { tokens.push(MathToken::Num(cur.parse().ok()?)); cur.clear(); }
            tokens.push(if c == '(' { MathToken::LParen } else { MathToken::RParen });
        } else if matches!(c, '+' | '*' | '/') {
            if !cur.is_empty() { tokens.push(MathToken::Num(cur.parse().ok()?)); cur.clear(); }
            let op = match c { '+' => ArithmeticOp::Add, '*' => ArithmeticOp::Multiply, '/' => ArithmeticOp::Divide, _ => unreachable!() };
            tokens.push(MathToken::Op(op));
        } else if c == 'x' || c == 'X' {
            if (i > 0 && chars[i - 1].is_alphabetic()) || (i + 1 < chars.len() && chars[i + 1].is_alphabetic()) { return None; }
            if !cur.is_empty() { tokens.push(MathToken::Num(cur.parse().ok()?)); cur.clear(); }
            tokens.push(MathToken::Op(ArithmeticOp::Multiply));
        } else if c == '-' {
            if cur.is_empty() && matches!(tokens.last(), None | Some(MathToken::Op(_)) | Some(MathToken::LParen)) {
                cur.push('-');
            } else {
                if !cur.is_empty() { tokens.push(MathToken::Num(cur.parse().ok()?)); cur.clear(); }
                tokens.push(MathToken::Op(ArithmeticOp::Subtract));
            }
        } else if c.is_numeric() || c == '.' {
            cur.push(c);
        } else {
            return None;
        }
        i += 1;
    }
    if !cur.is_empty() { tokens.push(MathToken::Num(cur.parse().ok()?)); }
    Some(tokens)
}

fn precedence(op: ArithmeticOp) -> u8 {
    match op { ArithmeticOp::Add | ArithmeticOp::Subtract => 1, ArithmeticOp::Multiply | ArithmeticOp::Divide => 2 }
}

fn apply_op(op: ArithmeticOp, a: f64, b: f64) -> Result<f64, ArithmeticOp> {
    if op == ArithmeticOp::Divide && b.abs() < 1e-12 { return Err(ArithmeticOp::Divide); }
    Ok(match op { ArithmeticOp::Add => a + b, ArithmeticOp::Subtract => a - b, ArithmeticOp::Multiply => a * b, ArithmeticOp::Divide => a / b })
}

fn pop_and_reduce(
    op_stack: &mut Vec<ArithmeticOp>,
    output: &mut Vec<f64>,
    reductions: &mut Vec<ArithmeticReduction>,
) -> Option<Result<(), ArithmeticOp>> {
    let (op, b, a) = (op_stack.pop()?, output.pop()?, output.pop()?);
    match apply_op(op, a, b) {
        Ok(res) => {
            reductions.push(ArithmeticReduction { step_index: reductions.len() + 1, left: a, op, right: b, result: res });
            output.push(res);
            Some(Ok(()))
        }
        Err(e) => Some(Err(e)),
    }
}

pub fn evaluate_math_tokens_with_reductions(
    tokens: &[MathToken],
) -> Option<Result<(f64, Vec<ArithmeticReduction>), ArithmeticOp>> {
    if tokens.is_empty() { return None; }
    let (mut output, mut op_stack, mut p_stack, mut reductions) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut expect_operand = true;

    for tok in tokens {
        match tok {
            MathToken::Num(n) => {
                if !expect_operand { return None; }
                output.push(*n);
                expect_operand = false;
            }
            MathToken::LParen => {
                if !expect_operand { return None; }
                p_stack.push(op_stack.len());
            }
            MathToken::RParen => {
                if expect_operand { return None; }
                let base = p_stack.pop()?;
                while op_stack.len() > base {
                    if let Err(e) = pop_and_reduce(&mut op_stack, &mut output, &mut reductions)? { return Some(Err(e)); }
                }
            }
            MathToken::Op(op) => {
                if expect_operand { return None; }
                let limit = p_stack.last().copied().unwrap_or(0);
                while op_stack.len() > limit && precedence(*op_stack.last().unwrap()) >= precedence(*op) {
                    if let Err(e) = pop_and_reduce(&mut op_stack, &mut output, &mut reductions)? { return Some(Err(e)); }
                }
                op_stack.push(*op);
                expect_operand = true;
            }
        }
    }

    if expect_operand || !p_stack.is_empty() { return None; }
    while !op_stack.is_empty() {
        if let Err(e) = pop_and_reduce(&mut op_stack, &mut output, &mut reductions)? { return Some(Err(e)); }
    }
    if output.len() == 1 { Some(Ok((output[0], reductions))) } else { None }
}

pub fn evaluate_math_tokens(tokens: &[MathToken]) -> Option<Result<f64, ArithmeticOp>> {
    evaluate_math_tokens_with_reductions(tokens).map(|res| res.map(|(val, _)| val))
}

fn plan_from_tokens(tokens: &[MathToken]) -> Option<ArithmeticPlan> {
    let first = match tokens.first() { Some(MathToken::Num(n)) => *n, _ => return None };
    let mut plan = ArithmeticPlan::new(first, None);
    let mut first_class: Option<bool> = None;
    for i in (1..tokens.len()).step_by(2) {
        let op = match &tokens[i] { MathToken::Op(op) => *op, _ => return None };
        let num = match tokens.get(i + 1) { Some(MathToken::Num(n)) => *n, _ => return None };
        plan.add_step(op, num);
        let cls = matches!(op, ArithmeticOp::Add | ArithmeticOp::Subtract);
        if *first_class.get_or_insert(cls) != cls { return None; }
    }
    if plan.steps.is_empty() { None } else { Some(plan) }
}

pub fn parse_math_expression(text: &str) -> Option<ArithmeticPlan> {
    let clean = text.trim_matches(|c| matches!(c, '¿' | '?' | '¡' | '!' | '.'));
    let lower = clean.to_lowercase();
    let mut expr = clean;
    for prefix in ["¿cuánto es ", "cuánto es ", "cuanto es ", "¿cuanto es ", "¿cuánto da ", "cuánto da ", "calcula ", "resultado de "] {
        if let Some(idx) = lower.find(prefix) { expr = &clean[idx + prefix.len()..]; break; }
    }
    if expr.trim().is_empty() { return None; }
    let tokens = tokenize_math(expr.trim())?;
    match evaluate_math_tokens_with_reductions(&tokens)? {
        Ok((val, reductions)) => {
            if let Some(mut plan) = plan_from_tokens(&tokens) {
                if plan.execute().ok()? == val {
                    for r in &reductions { plan.add_reduction(r.left, r.op, r.right, r.result); }
                    return Some(plan);
                }
            }
            if reductions.is_empty() { return None; }
            let mut plan = ArithmeticPlan::new(reductions[0].left, None);
            for r in &reductions { plan.add_reduction(r.left, r.op, r.right, r.result); plan.add_step(r.op, r.right); }
            Some(plan)
        }
        Err(ArithmeticOp::Divide) => {
            let mut plan = ArithmeticPlan::new(1.0, None);
            plan.add_step(ArithmeticOp::Divide, 0.0);
            Some(plan)
        }
        _ => None,
    }
}
