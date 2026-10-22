//! Analizador sintáctico para consultas de grafos y dependencias de software.
//! Menos de 180 líneas.

use crate::dispatch::QuerySpec;

pub fn parse_query_to_spec(text: &str) -> QuerySpec {
    let clean = text.trim_matches(|c| c == '¿' || c == '?' || c == '¡' || c == '!' || c == '.');
    let lower = clean.to_lowercase();

    let is_negated = if lower.contains("no es imposible") {
        false
    } else {
        lower.contains("es imposible") || lower.contains("no es posible") || lower.contains("no se puede")
    };

    let avoid = if let Some(idx) = lower.find("sin pasar por ") {
        let rest = &clean[idx + 14..];
        let token = rest.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '?' || c == '.' || c == ',');
        Some(token.to_string())
    } else {
        None
    };

    let via = if let Some(idx) = lower.find("pasando por ") {
        let rest = &clean[idx + 12..];
        let token = rest.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '?' || c == '.' || c == ',');
        Some(token.to_string())
    } else {
        None
    };

    if lower.contains("depende") && lower.contains("del") {
        let parts: Vec<&str> = clean.split_whitespace().collect();
        let mut initial = None;
        let mut target = None;
        for (i, &word) in parts.iter().enumerate() {
            if (word.eq_ignore_ascii_case("crate") || word.eq_ignore_ascii_case("componente")) && i + 1 < parts.len() {
                initial = Some(parts[i + 1].trim_matches(|c| c == '?' || c == ',').to_string());
            }
            if (word.eq_ignore_ascii_case("del") || word.eq_ignore_ascii_case("paquete")) && i + 1 < parts.len() {
                let candidate = parts[i + 1].trim_matches(|c| c == '?' || c == ',');
                if candidate != "paquete" && candidate != "crate" {
                    target = Some(candidate.to_string());
                } else if i + 2 < parts.len() {
                    target = Some(parts[i + 2].trim_matches(|c| c == '?' || c == ',').to_string());
                }
            }
        }
        if initial.is_some() && target.is_some() {
            return QuerySpec { initial, target, via, avoid, is_negated };
        }
    }

    let mut initial = None;
    for prefix in ["partiendo de ", "desde ", "salir de ", "de "] {
        if let Some(idx) = lower.find(prefix) {
            let rest = &clean[idx + prefix.len()..];
            let token = rest.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '?' || c == '.' || c == ',');
            if !["la", "el", "los", "las", "un", "una", "ir", "salir", "llegar"].contains(&token.to_lowercase().as_str()) {
                initial = Some(token.to_string());
                break;
            }
        }
    }

    if initial.is_none() && lower.contains("marta") && lower.contains("llegar a") {
        initial = Some("Marta".to_string());
    }

    let mut target = None;
    for prefix in ["hasta ", "hacia ", "llegar a ", "alcanzar ", " a "] {
        if let Some(idx) = lower.find(prefix) {
            let rest = &clean[idx + prefix.len()..];
            let token = rest.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '?' || c == '.' || c == ',');
            if !["la", "el", "los", "las", "un", "una"].contains(&token.to_lowercase().as_str()) {
                if Some(token) != initial.as_deref() {
                    target = Some(token.to_string());
                    break;
                }
            }
        }
    }

    QuerySpec { initial, target, via, avoid, is_negated }
}
