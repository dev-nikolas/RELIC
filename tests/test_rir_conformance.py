"""Pruebas de conformidad y casos adversariales para RIR v1 (Fase 4).

Ubicado fuera de los paquetes principales de producción.
Utiliza exclusivamente la biblioteca estándar de Python (unittest).
"""

import math
import json
import os
import sys
import unittest

# Importar el paquete relic desde el directorio python/
ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, os.path.join(ROOT_DIR, "python"))

from relic.rir_v1 import (
    RirDocumentV1,
    ValidityStatus,
    IntentKind,
    DomainKind,
    RirArithmeticOp,
    ArithmeticStep,
    GraphQuerySpec,
    UncertaintyReport,
)


class TestRirConformance(unittest.TestCase):
    def test_adversarial_unsupported_version(self):
        doc = RirDocumentV1(
            schema_version=99,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            initial_quantity=5.0,
            operations=[ArithmeticStep(RirArithmeticOp.ADD, 1.0)],
        )
        with self.assertRaises(ValueError) as ctx:
            doc.validate_semantic()
        self.assertIn("Versión de esquema no compatible", str(ctx.exception))

    def test_adversarial_contradictory_valid_but_rejected(self):
        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            initial_quantity=5.0,
            operations=[ArithmeticStep(RirArithmeticOp.ADD, 1.0)],
            uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.VALID),
        )
        with self.assertRaises(ValueError) as ctx:
            doc.validate_semantic()
        self.assertIn("incertidumbre tiene is_rejected=True", str(ctx.exception))

    def test_adversarial_contradictory_invalid_but_not_rejected(self):
        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.UNKNOWN,
            domain=DomainKind.UNSUPPORTED,
            status=ValidityStatus.INCOMPLETE,
            uncertainty=UncertaintyReport(is_rejected=False, status=ValidityStatus.INCOMPLETE),
        )
        with self.assertRaises(ValueError) as ctx:
            doc.validate_semantic()
        self.assertIn("incertidumbre no marcada como rechazada", str(ctx.exception))

    def test_adversarial_infinity_and_nan_operands(self):
        # NaN en initial_quantity
        doc_nan = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            initial_quantity=float("nan"),
            operations=[ArithmeticStep(RirArithmeticOp.ADD, 1.0)],
        )
        with self.assertRaises(ValueError) as ctx:
            doc_nan.validate_semantic()
        self.assertIn("Valor numérico no finito", str(ctx.exception))

        # Infinito en operando
        doc_inf = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            initial_quantity=5.0,
            operations=[ArithmeticStep(RirArithmeticOp.ADD, float("inf"))],
        )
        with self.assertRaises(ValueError) as ctx:
            doc_inf.validate_semantic()
        self.assertIn("Operando no finito", str(ctx.exception))

    def test_adversarial_division_by_zero(self):
        doc_div = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.ARITHMETIC_QUERY,
            domain=DomainKind.ABSTRACT_MATH,
            status=ValidityStatus.VALID,
            initial_quantity=10.0,
            operations=[ArithmeticStep(RirArithmeticOp.DIVIDE, 0.0)],
        )
        with self.assertRaises(ValueError) as ctx:
            doc_div.validate_semantic()
        self.assertIn("División por cero no permitida", str(ctx.exception))

    def test_adversarial_empty_graph_nodes(self):
        doc_graph = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.GRAPH_QUERY,
            domain=DomainKind.SPATIAL_GRAPH,
            status=ValidityStatus.VALID,
            graph_spec=GraphQuerySpec(start_node="   ", target_node="Barcelona"),
        )
        with self.assertRaises(ValueError) as ctx:
            doc_graph.validate_semantic()
        self.assertIn("start_node y target_node no pueden estar vacíos", str(ctx.exception))

    def test_cross_language_json_roundtrip(self):
        canonical_payload = {
            "schema_version": 1,
            "intent": "inventory_update",
            "domain": "inventory",
            "status": "VALID",
            "entity": "cuadernos",
            "initial_quantity": 7.0,
            "operations": [
                {"operator": "add", "operand": 3.0},
                {"operator": "subtract", "operand": 2.0},
            ],
            "uncertainty": {
                "is_rejected": False,
                "status": "VALID",
                "missing_fields": [],
                "rejection_reason": None,
            },
        }
        json_str = json.dumps(canonical_payload)
        doc = RirDocumentV1.from_json(json_str)
        doc.validate_semantic()

        self.assertEqual(doc.entity, "cuadernos")
        self.assertEqual(doc.initial_quantity, 7.0)
        self.assertEqual(len(doc.operations), 2)
        self.assertEqual(doc.operations[0].operator, RirArithmeticOp.ADD)
        self.assertEqual(doc.operations[1].operator, RirArithmeticOp.SUBTRACT)


if __name__ == "__main__":
    unittest.main()
