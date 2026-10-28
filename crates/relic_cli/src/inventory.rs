//! Extractor y evaluador de problemas de inventario con expresiones anidadas, porcentajes y reparto.
//! Menos de 180 líneas.

use relic_core::{ArithmeticOp, ArithmeticPlan, ArithmeticReduction, CollapseDecision};
use relic_lexicon::LexiconView;
use crate::math::{evaluate_math_tokens_with_reductions, tokenize_math};
use crate::normalize::normalize_query_with_lexicon;

fn evaluate_clause_quantity(
    clause_text: &str,
    lexicon: Option<&LexiconView>,
) -> Option<(f64, Vec<ArithmeticReduction>)> {
    let norm = normalize_query_with_lexicon(clause_text, lexicon);
    let mut math_tokens_str = Vec::new();
    for tok in norm.split_whitespace() {
        let is_math_piece = tok.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | '+' | '-' | '*' | '/' | '(' | ')'));
        if is_math_piece {
            math_tokens_str.push(tok);
        } else if !math_tokens_str.is_empty() {
            break;
        }
    }
    if math_tokens_str.is_empty() { return None; }
    let tokens = tokenize_math(&math_tokens_str.join(" "))?;
    evaluate_math_tokens_with_reductions(&tokens)?.ok()
}

pub fn parse_inventory_problem(text: &str) -> Option<ArithmeticPlan> {
    parse_inventory_problem_with_lexicon(text, None)
}

pub fn parse_inventory_problem_with_lexicon(
    text: &str,
    lexicon: Option<&LexiconView>,
) -> Option<ArithmeticPlan> {
    parse_inventory_problem_estandar(text, lexicon)
        .or_else(|| crate::inverted::parse_inverted_inventory(text))
}

fn parse_inventory_problem_estandar(
    text: &str,
    lexicon: Option<&LexiconView>,
) -> Option<ArithmeticPlan> {
    let raw_lower = text.to_lowercase();
    let lower = lexicon.map_or(raw_lower.clone(), |v| v.apply_phrase_macros(&raw_lower));

    let prefixes = [
        "tengo ", "hay ", "poseo ", "tenía ", "tenia ",
        "cuento con ", "contaba con ", "gano ", "gané ", "gane ", "recibo ",
    ];
    let (idx_start, prefix_len) = prefixes
        .iter()
        .filter_map(|&p| lower.find(p).map(|idx| (idx, p.len())))
        .min_by_key(|&(idx, _)| idx)?;
    let after_start = lower[idx_start + prefix_len..].trim();

    let normalized = after_start
        .replace(',', " | ")
        .replace(" luego ", " | ")
        .replace(" después ", " | ")
        .replace(" despues ", " | ")
        .replace(" pero ", " | ")
        .replace(" y ", " | ");

    let mut clauses = normalized.split('|');
    let first_clause = clauses.next()?.trim();
    if first_clause.is_empty() { return None; }

    let first_parts: Vec<&str> = first_clause.split_whitespace().collect();
    let (initial_val, initial_reductions, unit) = if let Ok(v) = first_parts[0].parse::<f64>() {
        let u = if first_parts.len() > 1 {
            let candidate = first_parts[1].trim_matches(|c: char| !c.is_alphanumeric());
            if !["y", "pero", "luego", "que", "de", "después", "despues"].contains(&candidate) {
                Some(candidate.to_string())
            } else {
                None
            }
        } else {
            None
        };
        (v, Vec::new(), u)
    } else {
        let (v, reds) = evaluate_clause_quantity(first_clause, lexicon)?;
        (v, reds, None)
    };

    let mut plan = ArithmeticPlan::new(initial_val, unit);
    for r in initial_reductions { plan.add_reduction(r.left, r.op, r.right, r.result); }
    let mut running = initial_val;

    for clause in clauses {
        let trimmed = clause.trim();
        if trimmed.is_empty() { continue; }
        let is_question_tail = ["cuánto", "cuanto", "qué queda", "que queda"].iter().any(|q| trimmed.contains(q))
            && evaluate_clause_quantity(trimmed, lexicon).is_none();
        if is_question_tail { continue; }

        let mut lexicon_op = None;
        if let Some(view) = lexicon {
            let clause_words: Vec<String> = trimmed
                .split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                .filter(|w| !w.is_empty())
                .collect();
            let ctx_refs: Vec<&str> = clause_words.iter().map(|s| s.as_str()).collect();
            for clean_w in &clause_words {
                if let Some(op) = view.resolve_op_alias(clean_w) {
                    lexicon_op = Some(op);
                    break;
                }
                if let Some(decision) = view.resolve_polysemous_quantum(clean_w, &ctx_refs, 3.0, 0.0) {
                    match decision {
                        CollapseDecision::Collapsed { target, .. } => { lexicon_op = Some(target); break; }
                        CollapseDecision::Abstained { .. } => return None,
                    }
                }
            }
        }

        let is_add = [
            "recibo", "recibe", "recibió", "recibio", "me dan", "me da", "compro", "compré", "compre",
            "agrego", "más", "mas", "sumo", "adquirí", "adquiri", "recolecté", "recolecte",
            "incorporé", "incorpore", "encuentro", "encontré", "encontre", "gano", "gané", "gane",
        ].iter().any(|&w| trimmed.contains(w));
        let is_sub = [
            "regalo", "regala", "doy", "di", "le di", "les di", "me quitan", "pierdo", "perdí", "perdi",
            "vendo", "vendí", "vendi", "como", "comí", "comi", "me comí", "me comi", "zampé", "zampe",
            "consumo", "consumí", "consumi", "gasto", "gasté", "gaste", "resto", "menos",
            "extravié", "extravie", "presté", "preste", "dono", "doné", "done", "cedo", "cedí", "cedi",
        ].iter().any(|&w| trimmed.contains(w));

        let (val, sub_reductions) = evaluate_clause_quantity(trimmed, lexicon)?;
        let resolved_op = if let Some(op) = lexicon_op {
            op
        } else if is_add {
            ArithmeticOp::Add
        } else if is_sub {
            ArithmeticOp::Subtract
        } else {
            return None;
        };

        plan.add_step(resolved_op, val);
        if plan.unit.is_none() || !sub_reductions.is_empty() || !plan.reductions.is_empty() {
            for r in sub_reductions { plan.add_reduction(r.left, r.op, r.right, r.result); }
            let next_val = match resolved_op {
                ArithmeticOp::Add => running + val,
                ArithmeticOp::Subtract => running - val,
                ArithmeticOp::Multiply => running * val,
                ArithmeticOp::Divide => if val.abs() >= 1e-12 { running / val } else { f64::NAN },
            };
            plan.add_reduction(running, resolved_op, val, next_val);
            running = next_val;
        }
    }

    if !plan.steps.is_empty() { Some(plan) } else { None }
}

