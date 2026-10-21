"""RELIC Intermediate Representation (RIR).

Representación formal, explícita, fuertemente tipada y verificable
para desacoplar la resolución de problemas del lenguaje natural humano.
Permite transferencia isomórfica de procedimientos entre dominios dispares
(grafos espaciales, dependencias de software, transformaciones algebraicas).
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class DomainKind(str, Enum):
    SPATIAL_GRAPH = "SpatialGraph"
    SOFTWARE_DEPENDENCY = "SoftwareDependency"
    ALGEBRAIC_RULE = "AlgebraicRule"
    UNKNOWN = "Unknown"


class ObjectiveKind(str, Enum):
    REACHABILITY = "reach"
    SHORTEST_PATH = "shortest_path"
    TRANSFORM = "transform"
    INCOMPLETE = "incomplete"


@dataclass
class Constraints:
    via: str | None = None          # Restricción de paso obligatorio
    avoid: str | None = None        # Restricción de exclusión estricta
    max_hops: int = 10              # Cota de profundidad máxima

    def to_dict(self) -> dict[str, Any]:
        return {k: v for k, v in {"via": self.via, "avoid": self.avoid, "max_hops": self.max_hops}.items() if v is not None}


@dataclass
class UncertaintyReport:
    is_actionable: bool
    missing_fields: list[str] = field(default_factory=list)
    unknown_entities: list[str] = field(default_factory=list)
    recommended_action: str = "EXECUTE"  # "EXECUTE", "ASK_CLARIFICATION", "REJECT_UNKNOWN"


@dataclass
class RIRSpec:
    """Especificación canónica abstracta de un problema en RELIC."""
    domain: DomainKind
    objective: ObjectiveKind
    initial_state: str | None
    target_state: str | None
    constraints: Constraints = field(default_factory=Constraints)
    is_negated: bool = False
    raw_query: str = ""

    def to_canonical_text(self) -> str:
        """Serializa a DSL canónico de tareas de RELIC."""
        lines = [
            f"DOMAIN: {self.domain.value}",
            f"GOAL: {self.objective.value}({self.target_state or '?'})",
            f"KNOWN:",
            f"    initial_state = {self.initial_state or '?'}",
        ]
        c_dict = self.constraints.to_dict()
        if c_dict:
            lines.append("CONSTRAINTS:")
            for k, v in c_dict.items():
                lines.append(f"    {k}({v})")
        if self.is_negated:
            lines.append("POLARITY: negated")
        return "\n".join(lines)


@dataclass
class RIRPlan:
    """Plan de acción verificable derivado de la especificación RIR."""
    path: list[str] = field(default_factory=list)
    cost: int = 0
    all_edges_exist: bool = False
    constraints_satisfied: bool = False
    is_successful: bool = False
    failure_reason: str | None = None

    def to_canonical_text(self) -> str:
        lines = [
            "PLAN:",
            f"    {' -> '.join(self.path) if self.path else 'None'}",
            "VERIFY:",
            f"    all_edges_exist = {str(self.all_edges_exist).lower()}",
            f"    constraints_satisfied = {str(self.constraints_satisfied).lower()}",
            f"    is_successful = {str(self.is_successful).lower()}",
        ]
        if self.failure_reason:
            lines.append(f"    failure_reason = '{self.failure_reason}'")
        return "\n".join(lines)
