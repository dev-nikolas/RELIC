//! Analizador composicional de roles semánticos y relaciones de orden (Opción 1).
//! Menos de 180 líneas.

use relic_core::TemporalGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    Greater, // A > B (ej. más alto, supera en altura)
    Lesser,  // A < B (ej. más bajo, menor estatura)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpretationStatus {
    Valid,
    Negated,            // "A no es más alto que B" -> No se debe asumir A <= B como hecho positivo
    AmbiguousDimension, // "A supera a B" -> dimensión desconocida
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticComparison {
    pub left_entity: String,
    pub right_entity: String,
    pub dimension: Option<String>,
    pub polarity: Polarity,
    pub is_negated: bool,
    pub status: InterpretationStatus,
}

impl SemanticComparison {
    /// Si la relación es válida y afirmativa, devuelve el par normalizado (mayor, relación, menor).
    pub fn to_normalized_edge(&self) -> Option<(String, String, String)> {
        if self.status != InterpretationStatus::Valid || self.is_negated {
            return None;
        }
        let dim = self.dimension.as_deref()?;
        let rel_name = format!("{}_mayor", dim);

        match self.polarity {
            Polarity::Greater => Some((self.left_entity.clone(), rel_name, self.right_entity.clone())),
            Polarity::Lesser => Some((self.right_entity.clone(), rel_name, self.left_entity.clone())),
        }
    }
}

/// Extrae roles semánticos de una cláusula comparativa aislada.
pub fn extract_semantic_role(clause: &str) -> Option<SemanticComparison> {
    let clean = clause.trim_matches(|c: char| !c.is_alphanumeric() && c != ' ' && c != 'á' && c != 'é' && c != 'í' && c != 'ó' && c != 'ú' && c != 'ñ');
    let lower = clean.to_lowercase();
    let is_negated = lower.contains("no es ") || lower.contains("no supera") || lower.contains("no aventaja");

    // Detección de dimensión
    let dimension = if lower.contains("alto") || lower.contains("alta") || lower.contains("bajo") || lower.contains("baja") || lower.contains("altura") || lower.contains("estatura") {
        Some("altura".to_string())
    } else if lower.contains("rápido") || lower.contains("rápida") || lower.contains("lento") || lower.contains("lenta") || lower.contains("velocidad") {
        Some("velocidad".to_string())
    } else if lower.contains("pesado") || lower.contains("pesada") || lower.contains("ligero") || lower.contains("ligera") || lower.contains("peso") {
        Some("peso".to_string())
    } else if lower.contains("fuerte") || lower.contains("fuerza") || lower.contains("débil") || lower.contains("debil") {
        Some("fuerza".to_string())
    } else {
        None
    };

    // Ambigüedad: verbo comparativo sin dimensión especificada
    if dimension.is_none() && (lower.contains("supera a ") || lower.contains("aventaja a ")) {
        let parts: Vec<&str> = clean.split_whitespace().collect();
        return Some(SemanticComparison {
            left_entity: parts.first().unwrap_or(&"").to_string(),
            right_entity: parts.last().unwrap_or(&"").to_string(),
            dimension: None,
            polarity: Polarity::Greater,
            is_negated,
            status: InterpretationStatus::AmbiguousDimension,
        });
    }

    // Reglas composicionales de comparación en altura (con concordancia de género)
    let (left, right, polarity) = if let Some(idx) = lower.find(" es más alto que ").or_else(|| lower.find(" es más alta que ")) {
        (clean[..idx].trim(), clean[idx + 17..].trim(), Polarity::Greater)
    } else if let Some(idx) = lower.find(" es más bajo que ").or_else(|| lower.find(" es más baja que ")) {
        (clean[..idx].trim(), clean[idx + 17..].trim(), Polarity::Lesser)
    } else if let Some(idx) = lower.find(" tiene menor altura que ").or_else(|| lower.find(" tiene menos altura que ")) {
        (clean[..idx].trim(), clean[idx + 24..].trim(), Polarity::Lesser)
    } else if let Some(idx) = lower.find(" supera a ") {
        let after = clean[idx + 10..].trim();
        let target_part = after.to_lowercase().find(" en ").map(|i| after[..i].trim()).unwrap_or(after);
        (clean[..idx].trim(), target_part, Polarity::Greater)
    } else if let Some(idx) = lower.find(" aventaja a ") {
        let after = clean[idx + 12..].trim();
        let target_part = after.to_lowercase().find(" en ").map(|i| after[..i].trim()).unwrap_or(after);
        (clean[..idx].trim(), target_part, Polarity::Greater)
    } else if lower.starts_with("más alto que ") {
        // Orden inverso: "Más alto que Juan es Pedro"
        let rest = &clean[13..];
        if let Some(es_idx) = rest.to_lowercase().find(" es ") {
            let right_ent = rest[..es_idx].trim();
            let left_ent = rest[es_idx + 4..].trim();
            (left_ent, right_ent, Polarity::Greater)
        } else { return None; }
    } else {
        return None;
    };

    let left_clean = left.trim_start_matches("Si ").trim_start_matches("si ").split_whitespace().last().unwrap_or("").trim_matches(|c: char| !c.is_alphanumeric()).to_string();
    let right_clean = right.split_whitespace().next().unwrap_or("").trim_matches(|c: char| !c.is_alphanumeric()).to_string();

    if left_clean.is_empty() || right_clean.is_empty() { return None; }

    let status = if is_negated {
        InterpretationStatus::Negated
    } else {
        InterpretationStatus::Valid
    };

    Some(SemanticComparison {
        left_entity: left_clean,
        right_entity: right_clean,
        dimension,
        polarity,
        is_negated,
        status,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub struct RelationalQuerySpec {
    pub graph: TemporalGraph,
    pub question_type: RelationalQuestionType,
    pub unparsed_clauses: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RelationalQuestionType {
    Maximal(String),
}

/// Parsea consultas comparativas generales mapeando cláusulas afirmativas a aristas normalizadas.
pub fn parse_relational_query(text: &str) -> Option<RelationalQuerySpec> {
    let clean = text.trim_matches(|c| c == '¿' || c == '?' || c == '¡' || c == '!' || c == '.');
    let lower = clean.to_lowercase();

    let mut graph = TemporalGraph::new();
    let mut unparsed_clauses = Vec::new();
    let normalized = clean.replace(';', ".").replace(" y ", ". ");

    for sentence in normalized.split('.') {
        let s_trim = sentence.trim().trim_matches(|c| c == '¿' || c == '?' || c == '¡' || c == '!');
        let s_lower = s_trim.to_lowercase();
        if s_trim.is_empty() || s_lower.starts_with("quién ") || s_lower.starts_with("quien ") || s_lower.starts_with("cuál ") || s_lower.starts_with("cual ") {
            continue;
        }
        if let Some(comp) = extract_semantic_role(s_trim) {
            if let Some((greater, rel, lesser)) = comp.to_normalized_edge() {
                graph.add_relation(greater, rel, lesser);
            } else {
                unparsed_clauses.push(format!("{} [{:?}]", s_trim, comp.status));
            }
        } else {
            unparsed_clauses.push(s_trim.to_string());
        }
    }

    if graph.node_count() < 2 {
        return None;
    }

    let question_type = if lower.contains("más alto") || lower.contains("mas alto") || lower.contains("mayor estatura") {
        RelationalQuestionType::Maximal("altura_mayor".to_string())
    } else {
        return None;
    };

    Some(RelationalQuerySpec { graph, question_type, unparsed_clauses })
}
