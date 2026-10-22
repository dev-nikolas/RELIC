//! Tipos fundamentales y especificaciones de datos para RIR v1.
//! Menos de 180 líneas.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errores de validación semántica de RIR v1.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RirValidationError {
    #[error("Versión de esquema no compatible: esperada 1, recibida {0}")]
    UnsupportedSchemaVersion(u32),
    #[error("Estado marcado como válido pero rechazado en incertidumbre")]
    ContradictoryRejectionState,
    #[error("Consulta incompleta: faltan campos obligatorios: {0:?}")]
    MissingRequiredFields(Vec<String>),
    #[error("Valor numérico no válido: se detectó NaN o Infinito en '{0}'")]
    NonFiniteNumber(String),
    #[error("Operación no permitida: división por cero detectada")]
    DivisionByZero,
    #[error("Dominio incompatible con la intención: {0:?} no admite {1:?}")]
    DomainIntentMismatch(DomainKind, IntentKind),
    #[error("Operación de inventario sin pasos o sin cantidad inicial finita")]
    InvalidInventorySpec,
    #[error("Consulta de grafos sin nodo origen o nodo destino")]
    InvalidGraphSpec,
    #[error("Documento RIR no ejecutable debido a estado: {0:?}")]
    NonExecutableStatus(ValidityStatus),
}

/// Estado de certeza y validez epistémica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidityStatus {
    Valid,
    Incomplete,
    Ambiguous,
    Unsupported,
}

/// Intenciones soportadas en el sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    InventoryUpdate,
    ArithmeticQuery,
    GraphQuery,
    RuleTransform,
    Unknown,
}

/// Dominios formales del problema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainKind {
    Inventory,
    AbstractMath,
    SoftwareDependency,
    SpatialGraph,
    RuleString,
    Unsupported,
}

/// Operadores aritméticos tipados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RirArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Paso individual de una operación aritmética con preservación estricta de orden.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArithmeticStep {
    pub operator: RirArithmeticOp,
    pub operand: f64,
}

/// Tipo de restricción aplicable a una exploración de grafos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphConstraint {
    AvoidNode(String),
    RequireVia(String),
}

/// Especificación para consultas sobre grafos (espaciales o dependencias).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GraphQuerySpec {
    pub start_node: String,
    pub target_node: String,
    pub via_node: Option<String>,
    pub avoid_node: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<GraphConstraint>,
    pub is_negated: bool,
}

impl GraphQuerySpec {
    pub fn all_avoid_nodes(&self) -> Vec<&str> {
        let mut list = Vec::new();
        if let Some(ref a) = self.avoid_node {
            list.push(a.as_str());
        }
        for c in &self.constraints {
            if let GraphConstraint::AvoidNode(ref n) = c {
                if !list.contains(&n.as_str()) {
                    list.push(n.as_str());
                }
            }
        }
        list
    }

    pub fn all_via_nodes(&self) -> Vec<&str> {
        let mut list = Vec::new();
        if let Some(ref v) = self.via_node {
            list.push(v.as_str());
        }
        for c in &self.constraints {
            if let GraphConstraint::RequireVia(ref n) = c {
                if !list.contains(&n.as_str()) {
                    list.push(n.as_str());
                }
            }
        }
        list
    }
}

/// Metadatos de incertidumbre y rechazo epistémico.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UncertaintyReport {
    pub is_rejected: bool,
    pub status: ValidityStatus,
    pub missing_fields: Vec<String>,
    pub rejection_reason: Option<String>,
}

impl Default for UncertaintyReport {
    fn default() -> Self {
        Self {
            is_rejected: false,
            status: ValidityStatus::Valid,
            missing_fields: Vec::new(),
            rejection_reason: None,
        }
    }
}

/// Documento canónico RIR v1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RirDocumentV1 {
    pub schema_version: u32,
    pub intent: IntentKind,
    pub domain: DomainKind,
    pub status: ValidityStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_quantity: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<ArithmeticStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_spec: Option<GraphQuerySpec>,
    pub uncertainty: UncertaintyReport,
}
