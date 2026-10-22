//! Capa de despacho y análisis sintáctico-estructural de consultas.
//! Menos de 180 líneas.

use relic_core::{ArithmeticOp, ArithmeticPlan};
use crate::graph::parse_query_to_spec;
use crate::math::parse_math_expression;
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

pub fn parse_inventory_problem(text: &str) -> Option<ArithmeticPlan> {
    let lower = text.to_lowercase();
    let prefixes = ["tengo ", "hay ", "poseo ", "tenía ", "tenia ", "cuento con ", "contaba con "];
    let (idx_start, prefix_len) = prefixes.iter().find_map(|&p| {
        lower.find(p).map(|idx| (idx, p.len()))
    })?;
    let after_start = lower[idx_start + prefix_len..].trim();

    let parts: Vec<&str> = after_start.split_whitespace().collect();
    if parts.is_empty() { return None; }

    let initial_val: f64 = parts[0].parse().ok()?;
    let unit = if parts.len() > 1 {
        let u = parts[1].trim_matches(|c: char| !c.is_alphanumeric());
        if !["y", "pero", "luego", "que", "de", "después", "despues"].contains(&u) {
            Some(u.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let mut plan = ArithmeticPlan::new(initial_val, unit);
    let normalized = lower
        .replace(',', " | ")
        .replace(" luego ", " | ")
        .replace(" después ", " | ")
        .replace(" despues ", " | ")
        .replace(" pero ", " | ")
        .replace(" y ", " | ");

    for clause in normalized.split('|').skip(1) {
        let trimmed = clause.trim();
        if trimmed.is_empty() { continue; }

        let is_add = [
            "recibo", "recibe", "me dan", "me da", "compro", "compré", "compre",
            "agrego", "más", "mas", "sumo", "adquirí", "adquiri", "recolecté", "recolecte",
            "incorporé", "incorpore", "encuentro", "encontré", "encontre"
        ].iter().any(|&w| trimmed.contains(w));
        let is_sub = [
            "regalo", "regala", "doy", "di", "le di", "les di", "me quitan", "pierdo", "perdí", "perdi",
            "vendo", "vendí", "vendi", "como", "comí", "comi", "me comí", "me comi", "zampé", "zampe",
            "consumo", "consumí", "consumi", "gasto", "gasté", "gaste", "resto", "menos",
            "extravié", "extravie", "presté", "preste", "dono", "doné", "done", "cedo", "cedí", "cedi"
        ].iter().any(|&w| trimmed.contains(w));

        let mut found_num = None;
        for word in trimmed.split_whitespace() {
            let clean_word = word.trim_matches(|c: char| !c.is_numeric() && c != '.' && c != '-');
            if let Ok(val) = clean_word.parse::<f64>() {
                found_num = Some(val);
                break;
            }
        }

        if let Some(val) = found_num {
            if is_add {
                plan.add_step(ArithmeticOp::Add, val);
            } else if is_sub {
                plan.add_step(ArithmeticOp::Subtract, val);
            } else {
                return None;
            }
        }
    }

    if !plan.steps.is_empty() { Some(plan) } else { None }
}

pub fn parse_dispatch_intent(text: &str) -> DispatchIntent {
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

    let inventory_keywords = ["tengo ", "hay ", "cuento con ", "poseo ", "tenía ", "tenia ", "contaba con "];
    if inventory_keywords.iter().any(|k| lower.contains(k)) {
        if let Some(plan) = parse_inventory_problem(clean) {
            return DispatchIntent::Arithmetic(plan);
        } else {
            return DispatchIntent::Incomplete {
                reason: "Problema de inventario no estructurado o con operaciones ambiguas/no reconocidas.".to_string(),
                missing: "especificación clara de operaciones".to_string(),
            };
        }
    }

    if let Some(plan) = parse_math_expression(clean) {
        return DispatchIntent::Arithmetic(plan);
    }

    let graph_keywords = ["depende", "llegar a", "alcanzar", "camino de", "ruta de", "ir de", "partiendo de"];
    if graph_keywords.iter().any(|k| lower.contains(k)) {
        let spec = parse_query_to_spec(clean);
        if spec.initial.is_none() || spec.target.is_none() {
            return DispatchIntent::Incomplete {
                reason: "Consulta de ruta incompleta: falta especificar el origen o el destino requerido.".to_string(),
                missing: "extremo de ruta".to_string(),
            };
        }
        return DispatchIntent::GraphReachability(spec);
    }

    if let Some(rel_spec) = parse_relational_query(clean) {
        return DispatchIntent::RelationalReasoning(rel_spec);
    }

    DispatchIntent::FreeLanguage(clean.to_string())
}
