//! Constructores y validación semántica de RIR v1.
use super::types::*;

impl RirDocumentV1 {
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

    pub fn new_rejected(status: ValidityStatus, reason: &str, missing: Vec<String>) -> Self {
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
                constraints: Vec::new(),
                is_negated,
            }),
            uncertainty: UncertaintyReport::default(),
        }
    }

    pub fn validate_semantic(&self) -> Result<(), RirValidationError> {
        if self.schema_version != 1 {
            return Err(RirValidationError::UnsupportedSchemaVersion(self.schema_version));
        }

        if self.status != ValidityStatus::Valid {
            if !self.uncertainty.is_rejected {
                return Err(RirValidationError::ContradictoryRejectionState);
            }
            return Err(RirValidationError::NonExecutableStatus(self.status));
        }

        if self.uncertainty.is_rejected {
            return Err(RirValidationError::ContradictoryRejectionState);
        }

        match self.intent {
            IntentKind::InventoryUpdate | IntentKind::ArithmeticQuery => {
                let _initial = match self.initial_quantity {
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

                let avoid_nodes = spec.all_avoid_nodes();
                if avoid_nodes.contains(&spec.start_node.as_str()) || avoid_nodes.contains(&spec.target_node.as_str()) {
                    return Err(RirValidationError::InvalidGraphSpec);
                }
            }

            IntentKind::RuleTransform => {}

            IntentKind::Unknown => {
                return Err(RirValidationError::NonExecutableStatus(ValidityStatus::Unsupported));
            }
        }

        Ok(())
    }

    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}
