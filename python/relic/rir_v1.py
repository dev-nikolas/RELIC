"""Contrato RIR v1 en Python con tipado estricto y validación semántica idéntica a Rust.

Especificación canónica interoperable entre el compilador semántico y el runtime en Rust.
"""

from __future__ import annotations
from dataclasses import dataclass, field, asdict
from enum import Enum
import math
import json
from typing import Any


class ValidityStatus(str, Enum):
    VALID = "VALID"
    INCOMPLETE = "INCOMPLETE"
    AMBIGUOUS = "AMBIGUOUS"
    UNSUPPORTED = "UNSUPPORTED"


class IntentKind(str, Enum):
    INVENTORY_UPDATE = "inventory_update"
    ARITHMETIC_QUERY = "arithmetic_query"
    GRAPH_QUERY = "graph_query"
    RULE_TRANSFORM = "rule_transform"
    UNKNOWN = "unknown"


class DomainKind(str, Enum):
    INVENTORY = "inventory"
    ABSTRACT_MATH = "abstract_math"
    SOFTWARE_DEPENDENCY = "software_dependency"
    SPATIAL_GRAPH = "spatial_graph"
    RULE_STRING = "rule_string"
    UNSUPPORTED = "unsupported"


class RirArithmeticOp(str, Enum):
    ADD = "add"
    SUBTRACT = "subtract"
    MULTIPLY = "multiply"
    DIVIDE = "divide"


@dataclass
class ArithmeticStep:
    operator: RirArithmeticOp
    operand: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "operator": self.operator.value if isinstance(self.operator, RirArithmeticOp) else self.operator,
            "operand": float(self.operand),
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ArithmeticStep:
        return cls(
            operator=RirArithmeticOp(data["operator"]),
            operand=float(data["operand"]),
        )


@dataclass
class GraphQuerySpec:
    start_node: str
    target_node: str
    via_node: str | None = None
    avoid_node: str | None = None
    is_negated: bool = False

    def to_dict(self) -> dict[str, Any]:
        return {
            "start_node": self.start_node,
            "target_node": self.target_node,
            "via_node": self.via_node,
            "avoid_node": self.avoid_node,
            "is_negated": self.is_negated,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> GraphQuerySpec:
        return cls(
            start_node=data["start_node"],
            target_node=data["target_node"],
            via_node=data.get("via_node"),
            avoid_node=data.get("avoid_node"),
            is_negated=bool(data.get("is_negated", False)),
        )


@dataclass
class UncertaintyReport:
    is_rejected: bool = False
    status: ValidityStatus = ValidityStatus.VALID
    missing_fields: list[str] = field(default_factory=list)
    rejection_reason: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "is_rejected": self.is_rejected,
            "status": self.status.value if isinstance(self.status, ValidityStatus) else self.status,
            "missing_fields": list(self.missing_fields),
            "rejection_reason": self.rejection_reason,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> UncertaintyReport:
        return cls(
            is_rejected=bool(data.get("is_rejected", False)),
            status=ValidityStatus(data.get("status", ValidityStatus.VALID)),
            missing_fields=list(data.get("missing_fields", [])),
            rejection_reason=data.get("rejection_reason"),
        )


@dataclass
class RirDocumentV1:
    schema_version: int = 1
    intent: IntentKind = IntentKind.UNKNOWN
    domain: DomainKind = DomainKind.UNSUPPORTED
    status: ValidityStatus = ValidityStatus.VALID
    entity: str | None = None
    initial_quantity: float | None = None
    operations: list[ArithmeticStep] = field(default_factory=list)
    graph_spec: GraphQuerySpec | None = None
    uncertainty: UncertaintyReport = field(default_factory=UncertaintyReport)

    def validate_semantic(self) -> None:
        """Valida invariantes semánticos. Lanza ValueError si no es ejecutable."""
        if self.schema_version != 1:
            raise ValueError(f"Versión de esquema no compatible: {self.schema_version}")

        if self.status != ValidityStatus.VALID:
            if not self.uncertainty.is_rejected:
                raise ValueError("Estado no válido pero incertidumbre no marcada como rechazada")
            raise ValueError(f"Documento no ejecutable por estado: {self.status.value}")

        if self.uncertainty.is_rejected:
            raise ValueError("Estado marcado como válido pero incertidumbre tiene is_rejected=True")

        if self.intent in (IntentKind.INVENTORY_UPDATE, IntentKind.ARITHMETIC_QUERY):
            if self.initial_quantity is None:
                raise ValueError("Campo obligatorio ausente: initial_quantity")
            if not math.isfinite(self.initial_quantity):
                raise ValueError("Valor numérico no finito en initial_quantity")
            if not self.operations:
                raise ValueError("Operación aritmética sin pasos a ejecutar")

            for i, step in enumerate(self.operations):
                if not math.isfinite(step.operand):
                    raise ValueError(f"Operando no finito en step {i}")
                if step.operator == RirArithmeticOp.DIVIDE and abs(step.operand) < 1e-12:
                    raise ValueError("División por cero no permitida")

        elif self.intent == IntentKind.GRAPH_QUERY:
            if not self.graph_spec:
                raise ValueError("Campo obligatorio ausente: graph_spec")
            if not self.graph_spec.start_node.strip() or not self.graph_spec.target_node.strip():
                raise ValueError("start_node y target_node no pueden estar vacíos")

        elif self.intent == IntentKind.UNKNOWN:
            raise ValueError("Intención desconocida no ejecutable")

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {
            "schema_version": self.schema_version,
            "intent": self.intent.value if isinstance(self.intent, IntentKind) else self.intent,
            "domain": self.domain.value if isinstance(self.domain, DomainKind) else self.domain,
            "status": self.status.value if isinstance(self.status, ValidityStatus) else self.status,
            "uncertainty": self.uncertainty.to_dict(),
        }
        if self.entity is not None:
            d["entity"] = self.entity
        if self.initial_quantity is not None:
            d["initial_quantity"] = float(self.initial_quantity)
        if self.operations:
            d["operations"] = [op.to_dict() for op in self.operations]
        if self.graph_spec is not None:
            d["graph_spec"] = self.graph_spec.to_dict()
        return d

    def to_json(self, indent: int = 2) -> str:
        return json.dumps(self.to_dict(), indent=indent, ensure_ascii=False)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> RirDocumentV1:
        ops = [ArithmeticStep.from_dict(step) for step in data.get("operations", [])]
        graph = GraphQuerySpec.from_dict(data["graph_spec"]) if data.get("graph_spec") else None
        unc = UncertaintyReport.from_dict(data["uncertainty"]) if data.get("uncertainty") else UncertaintyReport()

        return cls(
            schema_version=int(data.get("schema_version", 1)),
            intent=IntentKind(data.get("intent", IntentKind.UNKNOWN)),
            domain=DomainKind(data.get("domain", DomainKind.UNSUPPORTED)),
            status=ValidityStatus(data.get("status", ValidityStatus.VALID)),
            entity=data.get("entity"),
            initial_quantity=float(data["initial_quantity"]) if data.get("initial_quantity") is not None else None,
            operations=ops,
            graph_spec=graph,
            uncertainty=unc,
        )

    @classmethod
    def from_json(cls, json_str: str) -> RirDocumentV1:
        return cls.from_dict(json.loads(json_str))
