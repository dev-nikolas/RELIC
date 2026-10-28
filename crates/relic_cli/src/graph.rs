//! Analizador sintáctico para consultas de grafos y dependencias de software.
//! Menos de 180 líneas.

use crate::dispatch::QuerySpec;

fn extract_entity_token(rest: &str) -> Option<String> {
    let mut words = rest.split_whitespace();
    let mut token = words.next()?.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
    if ["el", "la", "los", "las", "un", "una", "al", "del"].contains(&token.to_lowercase().as_str()) {
        token = words.next()?.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
    }
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

pub fn parse_query_to_spec(text: &str) -> QuerySpec {
    let clean = text.trim_matches(|c| c == '¿' || c == '?' || c == '¡' || c == '!' || c == '.');
    let lower = clean.to_lowercase();

    let is_negated = if lower.contains("no es imposible") {
        false
    } else {
        lower.contains("es imposible") || lower.contains("no es posible") || lower.contains("no se puede")
    };

    let avoid = ["sin pasar por ", "evitando ", "sin cruzar por ", "sin atravesar "].iter().find_map(|&prefix| {
        lower.find(prefix).and_then(|idx| {
            let rest = &clean[idx + prefix.len()..];
            extract_entity_token(rest)
        })
    });

    let via = ["pasando por ", "vía ", "via ", "a través de ", "haciendo escala en "].iter().find_map(|&prefix| {
        lower.find(prefix).and_then(|idx| {
            let rest = &clean[idx + prefix.len()..];
            extract_entity_token(rest)
        })
    });

    if lower.contains("depende") && (lower.contains("del ") || lower.contains("de ")) {
        let parts: Vec<&str> = clean.split_whitespace().collect();
        let mut initial = None;
        let mut target = None;
        for (i, &word) in parts.iter().enumerate() {
            if (word.eq_ignore_ascii_case("crate") || word.eq_ignore_ascii_case("componente")) && i + 1 < parts.len() {
                initial = Some(parts[i + 1].trim_matches(|c| c == '?' || c == ',').to_string());
            }
            if (word.eq_ignore_ascii_case("del") || word.eq_ignore_ascii_case("de") || word.eq_ignore_ascii_case("paquete")) && i + 1 < parts.len() {
                let candidate = parts[i + 1].trim_matches(|c| c == '?' || c == ',');
                if candidate != "paquete" && candidate != "crate" && candidate != "el" && candidate != "la" && candidate != "un" {
                    target = Some(candidate.to_string());
                } else if i + 2 < parts.len() {
                    let cand2 = parts[i + 2].trim_matches(|c| c == '?' || c == ',');
                    if cand2 != "paquete" && cand2 != "crate" {
                        target = Some(cand2.to_string());
                    }
                }
            }
        }
        if initial.is_some() && target.is_some() {
            return QuerySpec { initial, target, via, avoid, is_negated };
        }
    }

    let mut initial = None;
    let mut target = None;

    // Estructura gramatical 1: Verbo modal + Sujeto + Verbo de movimiento/alcance (+ Destino)
    // Ejemplos: "¿Puede Marta llegar a la biblioteca...?", "¿Podría el robot alcanzar la meta...?"
    for modal in ["puede ", "podría ", "podria ", "logra ", "consigue "] {
        if let Some(m_idx) = lower.find(modal) {
            let after_modal = &clean[m_idx + modal.len()..];
            let after_modal_lower = after_modal.to_lowercase();
            for mv in ["llegar a ", "llegar hasta ", "ir a ", "ir hasta ", "alcanzar ", "acceder a ", "viajar a ", "viajar hasta "] {
                if let Some(mv_idx) = after_modal_lower.find(mv) {
                    let subject_clause = after_modal[..mv_idx].trim();
                    let subj_lower = subject_clause.to_lowercase();
                    // Evitar falsos sujetos impersonales como "se" ("se puede ir a...")
                    if !subj_lower.is_empty() && subj_lower != "se" && subj_lower != "uno" {
                        if let Some(subj) = extract_entity_token(subject_clause) {
                            initial = Some(subj);
                            let target_clause = &after_modal[mv_idx + mv.len()..];
                            if let Some(t) = extract_entity_token(target_clause) {
                                target = Some(t);
                            }
                            break;
                        }
                    }
                }
            }
            if initial.is_some() {
                break;
            }
        }
    }

    // Estructura gramatical 1b: "¿Se puede llegar a Y desde X?" -> cuando "llegar a " o "alcanzar " precede a "desde "
    if initial.is_none() && target.is_none() {
        for mv in ["llegar a ", "llegar hasta ", "alcanzar "] {
            if let Some(mv_idx) = lower.find(mv) {
                let after_mv = &clean[mv_idx + mv.len()..];
                let after_mv_lower = after_mv.to_lowercase();
                if let Some(d_idx) = after_mv_lower.find(" desde ") {
                    let target_part = after_mv[..d_idx].trim();
                    let orig_part = after_mv[d_idx + 7..].trim();
                    target = extract_entity_token(target_part);
                    initial = extract_entity_token(orig_part);
                    break;
                }
            }
        }
    }

    // Estructura gramatical 2: Preposiciones estándar de origen ("desde X", "de X", "partiendo de X", "salir de X", "ir de X")
    if initial.is_none() {
        for prefix in ["partiendo de ", "desde ", "salir de ", "ir de ", " de "] {
            if let Some(idx) = lower.find(prefix) {
                let rest = &clean[idx + prefix.len()..];
                if let Some(token) = extract_entity_token(rest) {
                    if !["ir", "salir", "llegar", "alcanzar"].contains(&token.to_lowercase().as_str()) {
                        initial = Some(token);
                        break;
                    }
                }
            }
        }
    }
    if initial.is_none() && lower.starts_with("de ") {
        let rest = &clean[3..];
        if let Some(token) = extract_entity_token(rest) {
            if !["ir", "salir", "llegar", "alcanzar"].contains(&token.to_lowercase().as_str()) {
                initial = Some(token);
            }
        }
    }

    // Estructura gramatical 3: Preposiciones estándar de destino ("hasta Y", "hacia Y", "llegar a Y", "llegar hasta Y", "alcanzar Y", " a Y")
    if target.is_none() {
        for prefix in ["hasta ", "hacia ", "llegar a ", "llegar hasta ", "alcanzar ", " a "] {
            if let Some(idx) = lower.find(prefix) {
                let rest = &clean[idx + prefix.len()..];
                if let Some(token) = extract_entity_token(rest) {
                    if initial.as_deref() != Some(&token) {
                        target = Some(token);
                        break;
                    }
                }
            }
        }
    }

    QuerySpec { initial, target, via, avoid, is_negated }
}
