//! Capa de despacho y análisis sintáctico-estructural de consultas. Menos de 180 líneas.

use relic_core::{ArithmeticOp, ArithmeticPlan};
use relic_lexicon::LexiconView;
use crate::graph::parse_query_to_spec;
pub use crate::inventory::{parse_inventory_problem, parse_inventory_problem_with_lexicon};
use crate::math::parse_math_expression;
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

fn annotate_reparto_note(query_lower: &str, plan: &mut ArithmeticPlan) {
    if !(query_lower.contains("repartir ") || query_lower.contains("reparte ")) {
        return;
    }
    let check_int_div = |a: f64, b: f64| -> Option<String> {
        if b.abs() >= 1.0 && a.fract().abs() < 1e-6 && b.fract().abs() < 1e-6 {
            let (ai, bi) = (a.round() as i64, b.round() as i64);
            if bi != 0 {
                let (q, rem) = (ai / bi, ai % bi);
                return Some(if rem == 0 {
                    format!("reparto exacto: cociente {q}, resto 0")
                } else {
                    format!("reparto inexacto: cociente entero {q}, resto {rem}")
                });
            }
        }
        None
    };

    if let Some(last) = plan.reductions.last() {
        if last.op == ArithmeticOp::Divide {
            plan.exact_division_note = check_int_div(last.left, last.right);
        }
    } else if plan.steps.len() == 1 && plan.steps[0].0 == ArithmeticOp::Divide {
        let (a, b) = (plan.initial, plan.steps[0].1);
        if let Some(note) = check_int_div(a, b) {
            plan.add_reduction(a, ArithmeticOp::Divide, b, a / b);
            plan.exact_division_note = Some(note);
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
