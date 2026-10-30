
use relic_core::TemporalGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    Greater,
    Lesser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpretationStatus {
    Valid,
    Negated,
    AmbiguousDimension,
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
    pub fn to_normalized_edge(&self) -> Option<(String, String, String)> {
        if self.status != InterpretationStatus::Valid || self.is_negated { return None; }
        let rel_name = format!("{}_mayor", self.dimension.as_deref()?);
        match self.polarity {
            Polarity::Greater => Some((self.left_entity.clone(), rel_name, self.right_entity.clone())),
            Polarity::Lesser => Some((self.right_entity.clone(), rel_name, self.left_entity.clone())),
        }
    }
}

pub fn extract_semantic_role(clause: &str) -> Option<SemanticComparison> {
    let clean = clause.trim_matches(|c: char| !c.is_alphanumeric() && !matches!(c, ' ' | 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ñ'));
    let lower = clean.to_lowercase();
    let is_negated = lower.contains("no es ") || lower.contains("no supera") || lower.contains("no aventaja");

    let dimension = if ["alto", "alta", "bajo", "baja", "altura", "estatura"].iter().any(|w| lower.contains(w)) {
        Some("altura".to_string())
    } else if ["rápido", "rápida", "lento", "lenta", "velocidad"].iter().any(|w| lower.contains(w)) {
        Some("velocidad".to_string())
    } else if ["pesado", "pesada", "ligero", "ligera", "peso"].iter().any(|w| lower.contains(w)) {
        Some("peso".to_string())
    } else if ["fuerte", "fuerza", "débil", "debil"].iter().any(|w| lower.contains(w)) {
        Some("fuerza".to_string())
    } else {
        None
    };

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

    let (left, right, polarity) = if let Some(idx) = lower.find(" es más alto que ").or_else(|| lower.find(" es más alta que ")) {
        (clean[..idx].trim(), clean[idx + 17..].trim(), Polarity::Greater)
    } else if let Some(idx) = lower.find(" es más bajo que ").or_else(|| lower.find(" es más baja que ")) {
        (clean[..idx].trim(), clean[idx + 17..].trim(), Polarity::Lesser)
    } else if let Some(idx) = lower.find(" tiene menor altura que ").or_else(|| lower.find(" tiene menos altura que ")) {
        (clean[..idx].trim(), clean[idx + 24..].trim(), Polarity::Lesser)
    } else if let Some(idx) = lower.find(" supera a ") {
        let after = clean[idx + 10..].trim();
        let target_part = after.to_lowercase().find(" en ").map_or(after, |i| after[..i].trim());
        (clean[..idx].trim(), target_part, Polarity::Greater)
    } else if let Some(idx) = lower.find(" aventaja a ") {
        let after = clean[idx + 12..].trim();
        let target_part = after.to_lowercase().find(" en ").map_or(after, |i| after[..i].trim());
        (clean[..idx].trim(), target_part, Polarity::Greater)
    } else if lower.starts_with("más alto que ") {
        let rest = &clean[13..];
        let es_idx = rest.to_lowercase().find(" es ")?;
        (rest[es_idx + 4..].trim(), rest[..es_idx].trim(), Polarity::Greater)
    } else {
        return None;
    };

    let left_clean = left.trim_start_matches("Si ").trim_start_matches("si ")
        .split_whitespace().last().unwrap_or("").trim_matches(|c: char| !c.is_alphanumeric()).to_string();
    let right_clean = right.split_whitespace().next().unwrap_or("").trim_matches(|c: char| !c.is_alphanumeric()).to_string();
    if left_clean.is_empty() || right_clean.is_empty() { return None; }

    Some(SemanticComparison {
        left_entity: left_clean,
        right_entity: right_clean,
        dimension,
        polarity,
        is_negated,
        status: if is_negated { InterpretationStatus::Negated } else { InterpretationStatus::Valid },
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

pub fn parse_relational_query(text: &str) -> Option<RelationalQuerySpec> {
    let clean = text.trim_matches(|c| matches!(c, '¿' | '?' | '¡' | '!' | '.'));
    let lower = clean.to_lowercase();
    let mut graph = TemporalGraph::new();
    let mut unparsed_clauses = Vec::new();
    let normalized = clean
        .replace(", ¿quién", "; ¿quién")
        .replace(", ¿quien", "; ¿quien")
        .replace(", ¿cuál", "; ¿cuál")
        .replace(", ¿cual", "; ¿cual")
        .replace(';', ".")
        .replace(" y ", ". ");

    for sentence in normalized.split('.') {
        let s_trim = sentence.trim().trim_matches(|c| matches!(c, '¿' | '?' | '¡' | '!'));
        let s_lower = s_trim.to_lowercase();
        if s_trim.is_empty() || ["quién ", "quien ", "cuál ", "cual "].iter().any(|p| s_lower.starts_with(p)) {
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

    if graph.node_count() < 2 { return None; }
    let question_type = if ["más alto", "mas alto", "mayor estatura"].iter().any(|k| lower.contains(k)) {
        RelationalQuestionType::Maximal("altura_mayor".to_string())
    } else {
        return None;
    };
    Some(RelationalQuerySpec { graph, question_type, unparsed_clauses })
}
