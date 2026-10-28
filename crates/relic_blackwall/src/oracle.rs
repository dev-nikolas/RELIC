//! Paso 1 del Blackwall: Oráculo de Invariantes Inmutables.
//! Bloquea sobrescritura de operadores/palabras reservadas del núcleo, identificadores vacíos
//! y auto-ciclos inválidos antes de clonar el Sandbox.
//! Menos de 180 líneas.

use relic_lexicon::{LexiconPayload, LexiconView};
use thiserror::Error;

/// Conjunto inmutable de palabras y símbolos reservados del núcleo de RELIC.
/// Ninguna propuesta externa puede redefinir ni secuestrar estos tokens.
pub const RESERVED_CORE_TOKENS: &[&str] = &[
    // Símbolos aritméticos y lógicos
    "+", "-", "*", "/", "%", "=", "?", "¿", "!", "¡",
    // Negaciones y cuantificadores epistémicos
    "no", "sin", "sí", "si", "algunas", "algunos", "ningún", "ninguna",
    // Conectores y preposiciones estructurales
    "y", "e", "o", "u", "pero", "luego", "después", "despues",
    "de", "del", "a", "al", "desde", "hasta", "hacia", "por", "entre", "en", "con", "que",
    "pasando", "evitando", "vía", "via",
    // Artículos
    "el", "la", "los", "las", "un", "una",
    // Operadores verbales, proporcionales y comparativos nucleares
    "más", "mas", "menos", "suma", "sumar", "resta", "restar",
    "multiplica", "multiplicar", "divide", "dividir", "dividido",
    "quita", "quitar", "quítale", "quitale",
    "mitad", "tercio", "cuarto", "doble", "triple",
    "ciento", "porciento", "repartir", "reparte",
    "alto", "alta", "bajo", "baja", "mayor", "menor",
    // Verbos nucleares de inventario y grafos
    "tengo", "hay", "poseo", "tenía", "tenia", "cuento", "contaba", "gano", "gané", "gane",
    "recibo", "recibe", "compro", "compré", "compre", "agrego", "sumo",
    "regalo", "regala", "doy", "di", "pierdo", "perdí", "perdi", "vendo", "gasto",
    "puede", "podría", "podria", "logra", "consigue", "depende",
    "llegar", "alcanzar", "ir", "salir", "partiendo", "camino", "ruta",
    "es", "son", "cuánto", "cuanto", "calcula", "resultado",
    // Numerales base
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
    if lower.is_empty() {
        return false;
    }
    if lower.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return true;
    }
    RESERVED_CORE_TOKENS.contains(&lower.as_str())
}

/// Valida que una propuesta cumpla estrictamente los invariantes del núcleo antes del Sandbox.
pub fn validate_proposal_oracle(
    proposal: &LexiconPayload,
    _current_view: &LexiconView,
) -> Result<(), OracleViolation> {
    match proposal {
        LexiconPayload::OpAlias {
            alias_word, weight, ..
        } => {
            let trimmed = alias_word.trim();
            if trimmed.is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "alias_word".to_string(),
                });
            }
            if !weight.is_finite() || *weight < 0.0 {
                return Err(OracleViolation::InvalidWeight {
                    weight: format!("{weight}"),
                });
            }
            if is_reserved_or_numeric(trimmed) {
                return Err(OracleViolation::ReservedTokenOverwrite {
                    token: trimmed.to_string(),
                });
            }
            Ok(())
        }
        LexiconPayload::PhraseMacro {
            phrase,
            expansion,
            weight,
            ..
        } => {
            let p_trim = phrase.trim();
            let e_trim = expansion.trim();
            if p_trim.is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "phrase".to_string(),
                });
            }
            if e_trim.is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "expansion".to_string(),
                });
            }
            if !weight.is_finite() || *weight < 0.0 {
                return Err(OracleViolation::InvalidWeight {
                    weight: format!("{weight}"),
                });
            }
            if is_reserved_or_numeric(p_trim) {
                return Err(OracleViolation::ReservedTokenOverwrite {
                    token: p_trim.to_string(),
                });
            }
            Ok(())
        }
        LexiconPayload::EntityAlias {
            alias,
            canonical,
            weight,
            ..
        } => {
            let alias_trim = alias.trim();
            let canon_trim = canonical.trim();
            if alias_trim.is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "alias".to_string(),
                });
            }
            if canon_trim.is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "canonical".to_string(),
                });
            }
            if !weight.is_finite() || *weight < 0.0 {
                return Err(OracleViolation::InvalidWeight {
                    weight: format!("{weight}"),
                });
            }
            if is_reserved_or_numeric(alias_trim) {
                return Err(OracleViolation::ReservedTokenOverwrite {
                    token: alias_trim.to_string(),
                });
            }
            Ok(())
        }
        LexiconPayload::GraphFact {
            from,
            relation,
            to,
            weight,
            ..
        } => {
            let f_trim = from.trim();
            let r_trim = relation.trim();
            let t_trim = to.trim();
            if f_trim.is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "from".to_string(),
                });
            }
            if r_trim.is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "relation".to_string(),
                });
            }
            if t_trim.is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "to".to_string(),
                });
            }
            if !weight.is_finite() || *weight < 0.0 {
                return Err(OracleViolation::InvalidWeight {
                    weight: format!("{weight}"),
                });
            }
            if f_trim.eq_ignore_ascii_case(t_trim) {
                return Err(OracleViolation::SelfLoopNotAllowed {
                    node: f_trim.to_string(),
                });
            }
            Ok(())
        }
        LexiconPayload::EdgeReinforcement {
            from,
            to,
            new_weight,
            ..
        } => {
            if from.trim().is_empty() || to.trim().is_empty() {
                return Err(OracleViolation::EmptyIdentifier {
                    field: "from/to".to_string(),
                });
            }
            if !new_weight.is_finite() || *new_weight < 0.0 {
                return Err(OracleViolation::InvalidWeight {
                    weight: format!("{new_weight}"),
                });
            }
            Ok(())
        }
        LexiconPayload::Rollback { .. } | LexiconPayload::ConsolidationCheckpoint { .. } => {
            Err(OracleViolation::DirectControlPayloadForbidden)
        }
    }
}
