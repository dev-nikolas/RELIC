
use relic_lexicon::{LexiconPayload, LexiconView};
use thiserror::Error;

pub const RESERVED_CORE_TOKENS: &[&str] = &[
    "+", "-", "*", "/", "%", "=", "?", "¿", "!", "¡",
    "no", "sin", "sí", "si", "algunas", "algunos", "ningún", "ninguna",
    "y", "e", "o", "u", "pero", "luego", "después", "despues",
    "de", "del", "a", "al", "desde", "hasta", "hacia", "por", "entre", "en", "con", "que",
    "pasando", "evitando", "vía", "via",
    "el", "la", "los", "las", "un", "una",
    "más", "mas", "menos", "suma", "sumar", "resta", "restar",
    "multiplica", "multiplicar", "divide", "dividir", "dividido",
    "quita", "quitar", "quítale", "quitale",
    "mitad", "tercio", "cuarto", "doble", "triple",
    "ciento", "porciento", "repartir", "reparte",
    "alto", "alta", "bajo", "baja", "mayor", "menor",
    "tengo", "hay", "poseo", "tenía", "tenia", "cuento", "contaba", "gano", "gané", "gane",
    "recibo", "recibe", "compro", "compré", "compre", "agrego", "sumo",
    "regalo", "regala", "doy", "di", "pierdo", "perdí", "perdi", "vendo", "gasto",
    "puede", "podría", "podria", "logra", "consigue", "depende",
    "llegar", "alcanzar", "ir", "salir", "partiendo", "camino", "ruta",
    "es", "son", "cuánto", "cuanto", "calcula", "resultado",
    "cero", "uno", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve", "diez",
    "once", "doce", "trece", "catorce", "quince", "veinte", "treinta", "cuarenta", "cincuenta",
    "sesenta", "setenta", "ochenta", "noventa", "cien",
];

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OracleViolation {
    #[error("Violación del Oráculo: el token '{token}' es una palabra reservada del núcleo inmutable y no puede redefinirse")]
    ReservedTokenOverwrite { token: String },
    #[error("Violación del Oráculo: el campo '{field}' no puede estar vacío ni contener solo espacios")]
    EmptyIdentifier { field: String },
    #[error("Violación del Oráculo: auto-ciclo trivial '{node} -> {node}' prohibido en hechos relacionales")]
    SelfLoopNotAllowed { node: String },
    #[error("Violación del Oráculo: peso de activación inválido ({weight}); debe ser finito y >= 0.0")]
    InvalidWeight { weight: String },
    #[error("Violación del Oráculo: los rollbacks o consolidaciones deben ejecutarse mediante sus métodos transaccionales dedicados")]
    DirectControlPayloadForbidden,
}

fn is_reserved_or_numeric(token: &str) -> bool {
    let lower = token.trim().to_lowercase();
    !lower.is_empty() && (lower.chars().all(|c| c.is_ascii_digit() || c == '.') || RESERVED_CORE_TOKENS.contains(&lower.as_str()))
}

fn check_non_empty(val: &str, field: &str) -> Result<String, OracleViolation> {
    let trimmed = val.trim();
    if trimmed.is_empty() { Err(OracleViolation::EmptyIdentifier { field: field.to_string() }) } else { Ok(trimmed.to_string()) }
}

fn check_weight(w: f64) -> Result<(), OracleViolation> {
    if !w.is_finite() || w < 0.0 { Err(OracleViolation::InvalidWeight { weight: format!("{w}") }) } else { Ok(()) }
}

pub fn validate_proposal_oracle(proposal: &LexiconPayload, _current_view: &LexiconView) -> Result<(), OracleViolation> {
    match proposal {
        LexiconPayload::OpAlias { alias_word, weight, .. } => {
            let trimmed = check_non_empty(alias_word, "alias_word")?;
            check_weight(*weight)?;
            if is_reserved_or_numeric(&trimmed) { return Err(OracleViolation::ReservedTokenOverwrite { token: trimmed }); }
            Ok(())
        }
        LexiconPayload::QuantumPolysemousAlias { alias_word, amplitude_r, phase_theta, context_word, .. } => {
            let trimmed = check_non_empty(alias_word, "alias_word")?;
            if !amplitude_r.is_finite() || *amplitude_r <= 0.0 || !phase_theta.is_finite() {
                return Err(OracleViolation::InvalidWeight { weight: format!("r={amplitude_r}, theta={phase_theta}") });
            }
            if is_reserved_or_numeric(&trimmed) { return Err(OracleViolation::ReservedTokenOverwrite { token: trimmed }); }
            if let Some(cw) = context_word { check_non_empty(cw, "context_word")?; }
            Ok(())
        }
        LexiconPayload::PhraseMacro { phrase, expansion, weight, .. } => {
            let p_trim = check_non_empty(phrase, "phrase")?;
            check_non_empty(expansion, "expansion")?;
            check_weight(*weight)?;
            if is_reserved_or_numeric(&p_trim) { return Err(OracleViolation::ReservedTokenOverwrite { token: p_trim }); }
            Ok(())
        }
        LexiconPayload::EntityAlias { alias, canonical, weight, .. } => {
            let alias_trim = check_non_empty(alias, "alias")?;
            check_non_empty(canonical, "canonical")?;
            check_weight(*weight)?;
            if is_reserved_or_numeric(&alias_trim) { return Err(OracleViolation::ReservedTokenOverwrite { token: alias_trim }); }
            Ok(())
        }
        LexiconPayload::GraphFact { from, relation, to, weight, .. } => {
            let f_trim = check_non_empty(from, "from")?;
            check_non_empty(relation, "relation")?;
            let t_trim = check_non_empty(to, "to")?;
            check_weight(*weight)?;
            if f_trim.eq_ignore_ascii_case(&t_trim) { return Err(OracleViolation::SelfLoopNotAllowed { node: f_trim }); }
            Ok(())
        }
        LexiconPayload::EdgeReinforcement { from, to, new_weight, .. } => {
            if from.trim().is_empty() || to.trim().is_empty() {
                return Err(OracleViolation::EmptyIdentifier { field: "from/to".to_string() });
            }
            check_weight(*new_weight)
        }
        LexiconPayload::Rollback { .. } | LexiconPayload::ConsolidationCheckpoint { .. } => {
            Err(OracleViolation::DirectControlPayloadForbidden)
        }
    }
}
