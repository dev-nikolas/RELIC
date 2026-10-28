//! Capa de despacho y análisis sintáctico-estructural de consultas.
//! Soporta problemas de inventario con expresiones anidadas, porcentajes, proporcionales y reparto.

use relic_core::{ArithmeticOp, ArithmeticPlan, ArithmeticReduction};
use relic_lexicon::LexiconView;
use crate::graph::parse_query_to_spec;
use crate::math::{evaluate_math_tokens_with_reductions, parse_math_expression, tokenize_math};
use crate::normalize::normalize_query_with_lexicon;
use crate::relational::{parse_relational_query, RelationalQuerySpec};

#[derive(Debug, PartialEq)]
pub enum DispatchIntent {
    Arithmetic(ArithmeticPlan),
    GraphReachability(QuerySpec),
    RelationalReasoning(RelationalQuerySpec),
    FreeLanguage(String),
    Incomplete { reason: String, missing: String },
}

#[derive(Debug, PartialEq, Eq)]
pub struct QuerySpec {
    pub initial: Option<String>,
    pub target: Option<String>,
    pub via: Option<String>,
    pub avoid: Option<String>,
    pub is_negated: bool,
}

/// Evalúa un fragmento de cláusula (ej. `"el doble de 100"`, `"el 10% de 200"`, `"15"`)
/// devolviendo `(valor, reducciones_internas)`.
fn evaluate_clause_quantity(
    clause_text: &str,
    lexicon: Option<&LexiconView>,
) -> Option<(f64, Vec<ArithmeticReduction>)> {
    let norm = normalize_query_with_lexicon(clause_text, lexicon);
    // Buscar la sub-expresión matemática contigua dentro del texto normalizado
    let mut math_tokens_str = Vec::new();
    for tok in norm.split_whitespace() {
        let is_math_piece = tok.chars().all(|c| {
            c.is_ascii_digit() || matches!(c, '.' | '+' | '-' | '*' | '/' | '(' | ')')
        });
        if is_math_piece {
            math_tokens_str.push(tok);
        } else if !math_tokens_str.is_empty() {
            break;
        }
    }
    if math_tokens_str.is_empty() {
        return None;
    }
    let expr_str = math_tokens_str.join(" ");
    let tokens = tokenize_math(&expr_str)?;
    let (val, reductions) = evaluate_math_tokens_with_reductions(&tokens)?.ok()?;
    Some((val, reductions))
}

/// Parse de inventario: camino estándar + extractor invertido como fallback.
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
    let lower = if let Some(view) = lexicon {
        view.apply_phrase_macros(&raw_lower)
    } else {
        raw_lower
    };

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
    if first_clause.is_empty() {
        return None;
    }

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
    for r in initial_reductions {
        plan.add_reduction(r.left, r.op, r.right, r.result);
    }
    let mut running = initial_val;

    for clause in clauses {
        let trimmed = clause.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Ignorar cláusulas puramente interrogativas finales ("¿cuánto queda?")
        let is_question_tail = ["cuánto", "cuanto", "qué queda", "que queda"]
            .iter()
            .any(|q| trimmed.contains(q))
            && evaluate_clause_quantity(trimmed, lexicon).is_none();
        if is_question_tail {
            continue;
        }

        let mut lexicon_op = None;
        if let Some(view) = lexicon {
            for word in trimmed.split_whitespace() {
                let clean_w = word.trim_matches(|c: char| !c.is_alphanumeric());
                if let Some(op) = view.resolve_op_alias(clean_w) {
                    lexicon_op = Some(op);
                    break;
                }
            }
        }

        let is_add = [
            "recibo", "recibe", "recibió", "recibio", "me dan", "me da", "compro", "compré", "compre",
            "agrego", "más", "mas", "sumo", "adquirí", "adquiri", "recolecté", "recolecte",
            "incorporé", "incorpore", "encuentro", "encontré", "encontre", "gano", "gané", "gane",
        ]
        .iter()
        .any(|&w| trimmed.contains(w));
        let is_sub = [
            "regalo", "regala", "doy", "di", "le di", "les di", "me quitan", "pierdo", "perdí", "perdi",
            "vendo", "vendí", "vendi", "como", "comí", "comi", "me comí", "me comi", "zampé", "zampe",
            "consumo", "consumí", "consumi", "gasto", "gasté", "gaste", "resto", "menos",
            "extravié", "extravie", "presté", "preste", "dono", "doné", "done", "cedo", "cedí", "cedi",
        ]
        .iter()
        .any(|&w| trimmed.contains(w));

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
            for r in sub_reductions {
                plan.add_reduction(r.left, r.op, r.right, r.result);
            }
            let next_val = match resolved_op {
                ArithmeticOp::Add => running + val,
                ArithmeticOp::Subtract => running - val,
                ArithmeticOp::Multiply => running * val,
                ArithmeticOp::Divide => {
                    if val.abs() >= 1e-12 {
                        running / val
                    } else {
                        f64::NAN
                    }
                }
            };
            plan.add_reduction(running, resolved_op, val, next_val);
            running = next_val;
        }
    }

    if !plan.steps.is_empty() {
        Some(plan)
    } else {
        None
    }
}

/// Anota diagnósticos de reparto exacto vs resto entero cuando la consulta pide repartir/dividir.
fn annotate_reparto_note(query_lower: &str, plan: &mut ArithmeticPlan) {
    if !(query_lower.contains("repartir ") || query_lower.contains("reparte ")) {
        return;
    }
    if let Some(last) = plan.reductions.last() {
        if last.op == ArithmeticOp::Divide && last.right.abs() >= 1.0 {
            let a = last.left;
            let b = last.right;
            if (a.fract()).abs() < 1e-6 && (b.fract()).abs() < 1e-6 {
                let ai = a.round() as i64;
                let bi = b.round() as i64;
                if bi != 0 {
                    let q = ai / bi;
                    let rem = ai % bi;
                    if rem == 0 {
                        plan.exact_division_note = Some(format!("reparto exacto: cociente {q}, resto 0"));
                    } else {
                        plan.exact_division_note = Some(format!(
                            "reparto inexacto: cociente entero {q}, resto {rem}"
                        ));
                    }
                }
            }
        }
    } else if plan.steps.len() == 1 && plan.steps[0].0 == ArithmeticOp::Divide {
        let a = plan.initial;
        let b = plan.steps[0].1;
        if b.abs() >= 1.0 && (a.fract()).abs() < 1e-6 && (b.fract()).abs() < 1e-6 {
            let ai = a.round() as i64;
            let bi = b.round() as i64;
            if bi != 0 {
                let q = ai / bi;
                let rem = ai % bi;
                plan.add_reduction(a, ArithmeticOp::Divide, b, a / b);
                if rem == 0 {
                    plan.exact_division_note = Some(format!("reparto exacto: cociente {q}, resto 0"));
                } else {
                    plan.exact_division_note = Some(format!(
                        "reparto inexacto: cociente entero {q}, resto {rem}"
                    ));
                }
            }
        }
    }
}

pub fn parse_dispatch_intent(text: &str) -> DispatchIntent {
    parse_dispatch_intent_with_lexicon(text, None)
}

pub fn parse_dispatch_intent_with_lexicon(
    text: &str,
    lexicon: Option<&LexiconView>,
) -> DispatchIntent {
    let clean = text.trim();
    let lower = clean.to_lowercase();

    if lower.contains("+ ?") || lower.contains("- ?") || lower.contains("* ?") || lower.contains("/ ?")
        || clean.ends_with('+') || clean.ends_with('-') || clean.ends_with('*') || clean.ends_with('/')
    {
        return DispatchIntent::Incomplete {
            reason: "Falta el segundo operando para completar la operación solicitada.".to_string(),
            missing: "segundo operando".to_string(),
        };
    }

    if lower.contains("tengo algunas ") || lower.contains("tengo algunos ") || lower.contains("hay algunas ") {
        return DispatchIntent::Incomplete {
            reason: "Se desconoce la cantidad inicial disponible. Por favor especifica cuántas tienes al inicio.".to_string(),
            missing: "cantidad inicial".to_string(),
        };
    }

    let inventory_keywords = [
        "tengo ", "hay ", "cuento con ", "poseo ", "tenía ", "tenia ",
        "contaba con ", "si gano ", "gano ", "si recibo ",
    ];
    if inventory_keywords.iter().any(|k| lower.contains(k)) {
        if let Some(plan) = parse_inventory_problem_with_lexicon(clean, lexicon) {
            return DispatchIntent::Arithmetic(plan);
        }
        let norm_inventario = normalize_query_with_lexicon(clean, lexicon);
        if norm_inventario != clean {
            if let Some(plan) = parse_inventory_problem_with_lexicon(&norm_inventario, lexicon) {
                return DispatchIntent::Arithmetic(plan);
            }
        }
        return DispatchIntent::Incomplete {
            reason: "Problema de inventario no estructurado o con operaciones ambiguas/no reconocidas.".to_string(),
            missing: "especificación clara de operaciones".to_string(),
        };
    }

    if let Some(mut plan) = parse_math_expression(clean) {
        annotate_reparto_note(&lower, &mut plan);
        return DispatchIntent::Arithmetic(plan);
    }
    let norm = normalize_query_with_lexicon(clean, lexicon);
    if norm != clean {
        if let Some(mut plan) = parse_math_expression(&norm) {
            annotate_reparto_note(&lower, &mut plan);
            return DispatchIntent::Arithmetic(plan);
        }
    }

    let graph_keywords = ["depende", "llegar a", "alcanzar", "camino de", "ruta de", "ir de", "partiendo de"];
    if graph_keywords.iter().any(|k| lower.contains(k)) {
        let spec = parse_query_to_spec(clean);
        if spec.initial.is_none() || spec.target.is_none() {
            return DispatchIntent::Incomplete {
                reason: "Consulta de ruta con redacción no reconocida: no se identificó el origen o destino requerido. Ejemplo válido: '¿Se puede ir de Bogotá a Lima?' o '¿Existe un camino de Madrid hasta París?'.".to_string(),
                missing: "extremo de ruta reconocido".to_string(),
            };
        }
        return DispatchIntent::GraphReachability(spec);
    }

    if let Some(rel_spec) = parse_relational_query(clean) {
        return DispatchIntent::RelationalReasoning(rel_spec);
    }

    DispatchIntent::FreeLanguage(clean.to_string())
}
