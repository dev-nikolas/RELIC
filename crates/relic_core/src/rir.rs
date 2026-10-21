//! # RIR v1 (RELIC Intermediate Representation Version 1)
//!
//! Contrato formal, tipado y versionado entre la interpretación lingüística y la
//! ejecución determinista en RELIC.
//!
//! # Principio Fundamental:
//! La validez sintáctica de un JSON NO implica validez semántica en RIR.
//! Para ser ejecutable por `ReasoningEngine`, una especificación debe pasar la frontera
//! de validación estricta (`validate_semantic`), garantizando ausencia de valores NaN/Inf,
//! presencia de campos obligatorios por dominio y un estado inequívoco `ValidityStatus::Valid`.

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

/// Especificación para consultas sobre grafos (espaciales o dependencias).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GraphQuerySpec {
    pub start_node: String,
    pub target_node: String,
    pub via_node: Option<String>,
    pub avoid_node: Option<String>,
    pub is_negated: bool,
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

impl RirDocumentV1 {
    /// Constructor canónico para operaciones de inventario o aritméticas válidas.
    pub fn new_inventory(
        entity: Option<String>,
        initial_quantity: f64,
        operations: Vec<ArithmeticStep>,
    ) -> Self {
        Self {
            schema_version: 1,
            intent: IntentKind::InventoryUpdate,
            domain: DomainKind::Inventory,
            status: ValidityStatus::Valid,
            entity,
            initial_quantity: Some(initial_quantity),
            operations,
            graph_spec: None,
            uncertainty: UncertaintyReport::default(),
        }
    }

    /// Constructor para rechazos por incompletitud o ambigüedad.
    pub fn new_rejected(
        status: ValidityStatus,
        reason: &str,
        missing: Vec<String>,
    ) -> Self {
        Self {
            schema_version: 1,
            intent: IntentKind::Unknown,
            domain: DomainKind::Unsupported,
            status,
            entity: None,
            initial_quantity: None,
            operations: Vec::new(),
            graph_spec: None,
            uncertainty: UncertaintyReport {
                is_rejected: true,
                status,
                missing_fields: missing,
                rejection_reason: Some(reason.to_string()),
            },
        }
    }

    /// Constructor para consultas de grafos válidas.
    pub fn new_graph(
        domain: DomainKind,
        start_node: String,
        target_node: String,
        via_node: Option<String>,
        avoid_node: Option<String>,
        is_negated: bool,
    ) -> Self {
        Self {
            schema_version: 1,
            intent: IntentKind::GraphQuery,
            domain,
            status: ValidityStatus::Valid,
            entity: None,
            initial_quantity: None,
            operations: Vec::new(),
            graph_spec: Some(GraphQuerySpec {
                start_node,
                target_node,
                via_node,
                avoid_node,
                is_negated,
            }),
            uncertainty: UncertaintyReport::default(),
        }
    }

    /// Validación semántica estricta.
    /// Garantiza que el documento cumple todos los invariantes de seguridad matemática y lógica
    /// antes de delegar cualquier cálculo a `ReasoningEngine`.
    pub fn validate_semantic(&self) -> Result<(), RirValidationError> {
        // 1. Versión de esquema
        if self.schema_version != 1 {
            return Err(RirValidationError::UnsupportedSchemaVersion(self.schema_version));
        }

        // 2. Coherencia de estado de rechazo
        if self.status != ValidityStatus::Valid {
            if !self.uncertainty.is_rejected {
                return Err(RirValidationError::ContradictoryRejectionState);
            }
            return Err(RirValidationError::NonExecutableStatus(self.status));
        }

        if self.uncertainty.is_rejected {
            return Err(RirValidationError::ContradictoryRejectionState);
        }

        // 3. Validación según intención
        match self.intent {
            IntentKind::InventoryUpdate | IntentKind::ArithmeticQuery => {
                let initial = match self.initial_quantity {
                    Some(v) => {
                        if !v.is_finite() {
                            return Err(RirValidationError::NonFiniteNumber("initial_quantity".to_string()));
                        }
                        v
                    }
                    None => {
                        return Err(RirValidationError::MissingRequiredFields(vec!["initial_quantity".to_string()]));
                    }
                };

                if self.operations.is_empty() {
                    return Err(RirValidationError::InvalidInventorySpec);
                }

                for (idx, step) in self.operations.iter().enumerate() {
                    if !step.operand.is_finite() {
                        return Err(RirValidationError::NonFiniteNumber(format!("operations[{}].operand", idx)));
                    }
                    if step.operator == RirArithmeticOp::Divide && step.operand.abs() < 1e-12 {
                        return Err(RirValidationError::DivisionByZero);
                    }
                }

                let _ = initial;
            }

            IntentKind::GraphQuery => {
                let spec = match self.graph_spec {
                    Some(ref s) => s,
                    None => {
                        return Err(RirValidationError::MissingRequiredFields(vec!["graph_spec".to_string()]));
                    }
                };

                if spec.start_node.trim().is_empty() || spec.target_node.trim().is_empty() {
                    return Err(RirValidationError::InvalidGraphSpec);
                }
            }

            IntentKind::RuleTransform => {
                // Dominio de transformación de reglas
            }

            IntentKind::Unknown => {
                return Err(RirValidationError::NonExecutableStatus(ValidityStatus::Unsupported));
            }
        }

        Ok(())
    }

    /// Serializa a JSON canónico formateado.
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializa desde JSON canónico.
    pub fn from_json_str(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

