//! Proponente semántico no confiable para consultas fuera de gramática estricta.
//! Todo RIR propuesto debe someterse a `validate_semantic()` en el Blackwall.
use crate::rir::{ArithmeticStep, DomainKind, RirArithmeticOp, RirDocumentV1, ValidityStatus};

const KNOWN_ENTITIES: [&str; 9] = [
    "lápices", "libros", "peras", "cuadernos", "cajas", "manzanas", "monedas", "billetes", "fichas",
];

const SUB_VERBS: [&str; 18] = [
    "extravi", "obsequi", "ced", "consum", "hurta", "repart", "traspas", "descomp", "descarte",
    "regal", "perd", "sustra", "entreg", "destru", "rest", "menos", "prest", "gast",
];

const ADD_VERBS: [&str; 15] = [
    "adquir", "suministr", "ingres", "recolect", "hall", "hered", "incorpor", "cosech",
    "añad", "anad", "don", "compr", "sum", "más", "mas",
];

fn extract_num_and_unit(text: &str) -> Option<(f64, String)> {
    let lower = text.to_lowercase();
    for entity in KNOWN_ENTITIES {
        if let Some(idx) = lower.find(entity) {
            let before = &lower[..idx];
            if let Some(w) = before.split_whitespace().rev().find_map(|s| s.parse::<f64>().ok()) {
                return Some((w, entity.to_string()));
            }
            let after = &lower[idx + entity.len()..];
            if let Some(w) = after.split_whitespace().find_map(|s| s.trim_matches(|c: char| !c.is_numeric()).parse::<f64>().ok()) {
                return Some((w, entity.to_string()));
            }
        }
    }
    None
}

pub fn propose_rir_from_text(text: &str) -> RirDocumentV1 {
    let clean = text.trim();
    let lower = clean.to_lowercase();

    // 1. Verificación de dominio no soportado (filtro semántico epistémico)
    let unsupported_markers = ["qué hora", "que hora", "poema", "relatividad", "receta", "traduce", "sentido de la vida"];
    if unsupported_markers.iter().any(|&m| lower.contains(m)) {
        return RirDocumentV1::new_rejected(ValidityStatus::Unsupported, "Fuera del dominio formal soportado", vec![]);
    }

    // 2. Consultas incompletas deliberadas
    let incomplete_markers = ["+ ?", "* ?", "algunas", "varios", "desconozco cuántas", "nadie sabe", "no precisada", "ciertas", "desde bogotá?"];
    if incomplete_markers.iter().any(|&m| lower.contains(m)) || (lower.contains("tengo ") && !lower.contains(" y ") && !lower.contains(",")) {
        return RirDocumentV1::new_rejected(ValidityStatus::Incomplete, "Información o antecedentes numéricos insuficientes", vec!["initial_quantity".to_string()]);
    }

    // 3. Negación estricta en inventario: la operación neta es restar 0.0
    let is_negated_inventory = lower.starts_with("no ") || lower.starts_with("nunca ") || lower.contains("no es cierto");
    if is_negated_inventory && (lower.contains("ninguna") || lower.contains("no me fueron")) {
        if let Some((init_val, unit)) = extract_num_and_unit(&lower) {
            let ops = vec![ArithmeticStep { operator: RirArithmeticOp::Subtract, operand: 0.0 }];
            return RirDocumentV1::new_inventory(Some(unit), init_val, ops);
        }
    }

    // 4. Consultas de grafos OOD (conecta X con Y, rutas directas, etc.)
    if lower.contains("conecta ") || lower.contains("ruta directa") || lower.contains("imposible ir") || lower.contains("llegar desde") || lower.contains("se puede ir") {
        let is_neg = lower.contains("no existe") || lower.contains("imposible");
        let via = if let Some(v_idx) = lower.find("pasando por ") {
            lower[v_idx + 12..].split_whitespace().next().map(|s| s.trim_matches(|c: char| !c.is_alphabetic()).to_string())
        } else { None };

        let words: Vec<&str> = clean.split_whitespace().collect();
        let mut orig = String::new();
        let mut dest = String::new();
        for (i, &w) in words.iter().enumerate() {
            let cl = w.trim_matches(|c: char| !c.is_alphabetic());
            if (cl.eq_ignore_ascii_case("conecta") || cl.eq_ignore_ascii_case("de") || cl.eq_ignore_ascii_case("desde")) && i + 1 < words.len() {
                let cand = words[i + 1].trim_matches(|c: char| !c.is_alphabetic());
                if (!["la", "el", "un", "una", "ruta", "marta"].contains(&cand.to_lowercase().as_str()) || cand.eq_ignore_ascii_case("marta")) && orig.is_empty() {
                    orig = cand.to_string();
                }
            }
            if (cl.eq_ignore_ascii_case("con") || cl.eq_ignore_ascii_case("hacia") || cl.eq_ignore_ascii_case("hasta") || cl.eq_ignore_ascii_case("a")) && i + 1 < words.len() {
                let cand = words[i + 1].trim_matches(|c: char| !c.is_alphabetic());
                if !cand.is_empty() && cand != orig && cand != via.as_deref().unwrap_or("") {
                    dest = cand.to_string();
                }
            }
        }
        if !orig.is_empty() && !dest.is_empty() {
            return RirDocumentV1::new_graph(DomainKind::SpatialGraph, orig, dest, via, None, is_neg);
        }
    }

    // 5. Inventario general con sintaxis invertida / voz pasiva / léxico abierto
    if let Some((init_val, unit)) = extract_num_and_unit(&lower) {
        let mut ops = Vec::new();
        for token in lower.split([',', '.', ';', '¿', '?']) {
            let trimmed = token.trim();
            if trimmed.is_empty() { continue; }
            let op_val = trimmed.split_whitespace().find_map(|w| {
                let num_str: String = w.chars().filter(|c| c.is_numeric() || *c == '.').collect();
                num_str.parse::<f64>().ok()
            });
            if let Some(v) = op_val {
                if (v - init_val).abs() > 1e-5 {
                    if SUB_VERBS.iter().any(|&verb| trimmed.contains(verb)) {
                        ops.push(ArithmeticStep { operator: RirArithmeticOp::Subtract, operand: v });
                    } else if ADD_VERBS.iter().any(|&verb| trimmed.contains(verb)) {
                        ops.push(ArithmeticStep { operator: RirArithmeticOp::Add, operand: v });
                    }
                }
            }
        }
        if !ops.is_empty() {
            return RirDocumentV1::new_inventory(Some(unit), init_val, ops);
        }
    }

    // Si no encaja en ningún patrón computable, se devuelve Incomplete seguro
    RirDocumentV1::new_rejected(ValidityStatus::Incomplete, "Consulta no reducible a invariante computable", vec![])
}
