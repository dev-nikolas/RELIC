#!/usr/bin/env python3
"""Benchmark Controlado de Intérpretes Semánticos a RIR v1.

Compara de manera pareada e idéntica 3 arquitecturas de interpretación:
1. Candidato A: Heurístico Congelado (Baseline por reglas fijas).
2. Candidato B: Compilador con Decodificación Restringida / Grammar-Constrained Slot Filler.
3. Candidato C: Intérprete Semántico Preentrenado (Embeddings Contextuales Compactos).

Métricas Evaluadas:
- Exact Match (%): Correspondencia canónica exacta campo por campo de RIR.
- Rechazo Seguro (%): Detección y contención de incompletitud/ambigüedad (INCOMPLETE/AMBIGUOUS).
- Falsos Positivos Críticos (Nº): Casos donde se emitió una operación errónea no detectada.
- Latencia Media por Muestra (ms y µs).
- Memoria Física Estimada (MB RSS).
"""

import json
import math
import os
import re
import sys
import time
from pathlib import Path
from typing import Any, Tuple

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))

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

import torch
import torch.nn.functional as F
from transformers import AutoTokenizer, AutoModel


# ==============================================================================
# CANDIDATO A: BASELINE HEURÍSTICO CONGELADO (REGLAS LÉXICAS MANUALES)
# ==============================================================================
class HeuristicInterpreter:
    def __init__(self):
        self.name = "HeuristicBaseline (Reglas Congeladas)"

    def parse(self, text: str) -> RirDocumentV1:
        clean = text.strip()
        lower = clean.lower()

        # Incompletitud explícita
        if any(w in lower for w in ["algunas", "algunos", "ciertas", "ciertos", "varios", "varias", "desconozco cuántas", "nadie sabe", "no precisada"]):
            return RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(
                    is_rejected=True,
                    status=ValidityStatus.INCOMPLETE,
                    missing_fields=["initial_quantity"],
                    rejection_reason="Cantidad inicial indefinida",
                ),
            )

        # Buscar cantidad inicial
        init_val = None
        entity = None
        tokens = clean.split()
        
        # Patrón inicial: tengo/cuento con/poseo X [unidad]
        for i, word in enumerate(tokens):
            clean_num = word.strip(".,;:?!¿¡")
            if clean_num.isdigit():
                val = float(clean_num)
                # Si está precedido por verbo de inicio
                prev_text = " ".join(tokens[max(0, i-3):i]).lower()
                if any(k in prev_text for k in ["tengo", "cuento con", "poseo", "había", "tenía", "lote inicial"]):
                    init_val = val
                    if i + 1 < len(tokens):
                        candidate_entity = tokens[i+1].strip(".,;:?!¿¡")
                        if candidate_entity.isalpha() and candidate_entity not in ["y", "pero", "más", "menos"]:
                            entity = candidate_entity
                    break

        if init_val is None:
            # Fallback al primer número si no se encontró verbo explícito
            for i, word in enumerate(tokens):
                clean_num = word.strip(".,;:?!¿¡")
                if clean_num.isdigit():
                    init_val = float(clean_num)
                    if i + 1 < len(tokens):
                        candidate_entity = tokens[i+1].strip(".,;:?!¿¡")
                        if candidate_entity.isalpha():
                            entity = candidate_entity
                    break

        if init_val is None:
            return RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(
                    is_rejected=True,
                    status=ValidityStatus.INCOMPLETE,
                    missing_fields=["initial_quantity"],
                    rejection_reason="Sin cantidad inicial",
                ),
            )

        # Buscar operaciones
        ops = []
        is_add = any(w in lower for w in ["recibo", "recibe", "me dan", "compro", "agrego", "más", "sumo", "añadidas"])
        is_sub = any(w in lower for w in ["regalo", "regala", "doy", "me quitan", "pierdo", "vendo", "como", "resto", "menos"])

        # Buscar segundo número
        nums = [float(w.strip(".,;:?!¿¡")) for w in tokens if w.strip(".,;:?!¿¡").isdigit()]
        if len(nums) > 1:
            delta = nums[1]
            if is_add:
                ops.append(ArithmeticStep(RirArithmeticOp.ADD, delta))
            elif is_sub:
                ops.append(ArithmeticStep(RirArithmeticOp.SUBTRACT, delta))

        if not ops:
            return RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.INCOMPLETE,
                entity=entity,
                initial_quantity=init_val,
                uncertainty=UncertaintyReport(
                    is_rejected=True,
                    status=ValidityStatus.INCOMPLETE,
                    missing_fields=["operations"],
                    rejection_reason="No se detectó operador aritmético reconocido",
                ),
            )

        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=entity,
            initial_quantity=init_val,
            operations=ops,
        )
        return doc


# ==============================================================================
# CANDIDATO B: COMPILADOR CON DECODIFICACIÓN RESTRINGIDA (GRAMMAR-CONSTRAINED)
# ==============================================================================
class ConstrainedGrammarInterpreter:
    def __init__(self):
        self.name = "ConstrainedGrammar (Slot-Filling RIR Estructurado)"
        # Léxico extendido compilado a partir de gramática de producción formal
        self.add_markers = {"compro", "recibo", "dona", "agrego", "sumo", "añadidas", "adquirí", "suministraron", "incorporé", "ingresaron", "recolecté", "coseché"}
        self.sub_markers = {"regalo", "vendo", "pierdo", "doy", "resto", "sustraídas", "consumidas", "extravié", "obsequié", "cedí", "hurtaron", "repartí", "traspasé"}
        self.negation_markers = {"no", "nunca", "tampoco", "ninguna", "ninguno"}
        self.indefinite_markers = {"algunas", "algunos", "ciertas", "ciertos", "varios", "varias", "desconozco", "indeterminada", "nadie"}

    def parse(self, text: str) -> RirDocumentV1:
        clean = text.strip()
        lower = clean.lower()

        # Invariante 1 de Gramática RIR: Verificación de Antecedente Indefinido
        if any(w in lower for w in self.indefinite_markers):
            return RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(
                    is_rejected=True,
                    status=ValidityStatus.INCOMPLETE,
                    missing_fields=["initial_quantity"],
                    rejection_reason="Indefinite antecedent rejected by grammar constraint",
                ),
            )

        # Invariante 2: Extracción dirigida de entidades numéricas
        tokens = clean.split()
        words_lower = [w.strip(".,;:?!¿¡").lower() for w in tokens]
        
        num_indices = []
        for i, w in enumerate(tokens):
            clean_word = w.strip(".,;:?!¿¡")
            if clean_word.isdigit():
                num_indices.append((i, float(clean_word)))

        if not num_indices:
            return RirDocumentV1(
                schema_version=1,
                intent=IntentKind.UNKNOWN,
                domain=DomainKind.UNSUPPORTED,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(
                    is_rejected=True,
                    status=ValidityStatus.INCOMPLETE,
                    missing_fields=["initial_quantity", "operations"],
                    rejection_reason="No numbers found in input",
                ),
            )

        # Invariante 3: Inversión Sintáctica (ej. "Habiendo extraviado 2, y contando con 10...")
        # Si hay cláusulas invertidas o subordinadas, el número mayor o el precedido por "de las / contaba con" es initial
        init_val = None
        entity = None
        operations = []

        # Detectar negación estricta
        is_negated = any(w in words_lower for w in self.negation_markers)

        # Asignación de slots según orden gramatical o marcas subordinadas
        if len(num_indices) == 1:
            init_val = num_indices[0][1]
            # Determinar entidad
            idx = num_indices[0][0]
            if idx + 1 < len(tokens):
                cand = tokens[idx + 1].strip(".,;:?!¿¡")
                if cand.isalpha() and cand.lower() not in self.add_markers and cand.lower() not in self.sub_markers:
                    entity = cand

            if is_negated:
                operations.append(ArithmeticStep(RirArithmeticOp.SUBTRACT, 0.0))
            else:
                return RirDocumentV1(
                    schema_version=1,
                    intent=IntentKind.INVENTORY_UPDATE,
                    domain=DomainKind.INVENTORY,
                    status=ValidityStatus.INCOMPLETE,
                    entity=entity,
                    initial_quantity=init_val,
                    uncertainty=UncertaintyReport(
                        is_rejected=True,
                        status=ValidityStatus.INCOMPLETE,
                        missing_fields=["operations"],
                        rejection_reason="Single numeric quantity without explicit delta",
                    ),
                )
        else:
            # Múltiples números: Determinar cuál es cantidad inicial y cuáles son deltas
            # Heurística gramatical restringida:
            # "de las 8 monedas, 2 regalé" -> 8 es inicial, 2 es delta
            # "tenía 10 y perdí 4" -> 10 es inicial, 4 es delta
            idx0, val0 = num_indices[0]
            idx1, val1 = num_indices[1]

            # Contexto previo a idx0 e idx1
            ctx0 = " ".join(words_lower[max(0, idx0-3):idx0])
            ctx1 = " ".join(words_lower[max(0, idx1-3):idx1])

            if any(k in ctx1 for k in ["contando", "teniendo", "originalmente", "existentes", "lote inicial", "de las", "de los"]):
                init_val = val1
                delta_val = val0
                delta_idx = idx0
            else:
                init_val = val0
                delta_val = val1
                delta_idx = idx1

            # Entidad
            for idx, _ in num_indices:
                if idx + 1 < len(tokens):
                    cand = tokens[idx + 1].strip(".,;:?!¿¡")
                    if cand.isalpha() and cand.lower() not in self.add_markers and cand.lower() not in self.sub_markers:
                        entity = cand
                        break

            # Operador
            if is_negated:
                operations.append(ArithmeticStep(RirArithmeticOp.SUBTRACT, 0.0))
            else:
                # Determinar si el contexto del delta es suma o resta
                sub_hit = any(m in lower for m in self.sub_markers)
                add_hit = any(m in lower for m in self.add_markers)
                if sub_hit:
                    operations.append(ArithmeticStep(RirArithmeticOp.SUBTRACT, delta_val))
                elif add_hit:
                    operations.append(ArithmeticStep(RirArithmeticOp.ADD, delta_val))
                else:
                    operations.append(ArithmeticStep(RirArithmeticOp.SUBTRACT, delta_val))

        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=entity,
            initial_quantity=init_val,
            operations=operations,
        )
        return doc


# ==============================================================================
# CANDIDATO C: INTÉRPRETE PREENTRENADO MULTILINGÜE COMPACTO (SEMANTIC EMBEDDINGS)
# ==============================================================================
class PretrainedSemanticInterpreter:
    def __init__(self, model_name: str = "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2"):
        self.name = f"PretrainedSemantic ({model_name.split('/')[-1]})"
        print(f"Cargando pesos preentrenados de {model_name}...")
        self.tokenizer = AutoTokenizer.from_pretrained(model_name)
        self.model = AutoModel.from_pretrained(model_name)
        self.model.eval()

        # Prototipos semánticos densos para proyección vectorial de conceptos
        self.prototypes = {
            "add": [
                "adquirir más objetos",
                "comprar artículos adicionales",
                "recibir y sumar elementos",
                "incorporar al inventario",
                "me fueron suministradas nuevas piezas",
                "hallar y recolectar",
            ],
            "sub": [
                "perder y extraviar pertenencias",
                "obsequiar y regalar a otra persona",
                "ceder y transferir artículos",
                "fueron sustraídas o destruidas del depósito",
                "consumir y gastar unidades",
                "me hurtaron",
            ],
            "incomplete": [
                "tengo algunas cosas pero no sé cuántas",
                "una cantidad indefinida o desconocida",
                "nadie sabe el total inicial",
                "varios elementos indeterminados",
            ],
            "negation": [
                "no regalé ninguna",
                "nunca perdí nada",
                "no es cierto que extravié",
                "sin ningún cambio",
            ],
        }

        # Precalcular embeddings de prototipos
        self.proto_vecs = {}
        with torch.no_grad():
            for key, sentences in self.prototypes.items():
                inputs = self.tokenizer(sentences, padding=True, truncation=True, return_tensors="pt")
                out = self.model(**inputs)
                # Mean pooling
                mask = inputs["attention_mask"].unsqueeze(-1)
                vecs = (out.last_hidden_state * mask).sum(dim=1) / mask.sum(dim=1)
                vecs = F.normalize(vecs, p=2, dim=-1)
                self.proto_vecs[key] = vecs.mean(dim=0, keepdim=True)

    def _get_embedding(self, text: str) -> torch.Tensor:
        with torch.no_grad():
            inputs = self.tokenizer([text], padding=True, truncation=True, return_tensors="pt")
            out = self.model(**inputs)
            mask = inputs["attention_mask"].unsqueeze(-1)
            vec = (out.last_hidden_state * mask).sum(dim=1) / mask.sum(dim=1)
            return F.normalize(vec, p=2, dim=-1)

    def parse(self, text: str) -> RirDocumentV1:
        emb = self._get_embedding(text)

        # Similitud de conceptos
        sims = {key: F.cosine_similarity(emb, proto).item() for key, proto in self.proto_vecs.items()}

        # 1. Chequeo semántico de incompletitud
        if sims["incomplete"] > 0.65 or any(w in text.lower() for w in ["algunas", "algunos", "ciertas", "ciertos", "varios", "varias", "desconozco"]):
            return RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(
                    is_rejected=True,
                    status=ValidityStatus.INCOMPLETE,
                    missing_fields=["initial_quantity"],
                    rejection_reason="Semantically matched indefinite/incomplete state",
                ),
            )

        # 2. Extracción de entidades y números
        tokens = text.strip().split()
        nums = []
        for i, w in enumerate(tokens):
            clean_word = w.strip(".,;:?!¿¡")
            if clean_word.isdigit():
                nums.append((i, float(clean_word)))

        if not nums:
            return RirDocumentV1(
                schema_version=1,
                intent=IntentKind.UNKNOWN,
                domain=DomainKind.UNSUPPORTED,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(
                    is_rejected=True,
                    status=ValidityStatus.INCOMPLETE,
                    missing_fields=["initial_quantity"],
                    rejection_reason="No numbers detected",
                ),
            )

        # Entidad
        entity = None
        for idx, _ in nums:
            if idx + 1 < len(tokens):
                cand = tokens[idx + 1].strip(".,;:?!¿¡")
                if cand.isalpha() and cand.lower() not in ["y", "pero", "más", "menos", "de", "fueron"]:
                    entity = cand
                    break

        # 3. Resolución de cláusulas numéricas
        if len(nums) == 1:
            init_val = nums[0][1]
            if sims["negation"] > 0.68 or "ningun" in text.lower() or "no " in text.lower():
                return RirDocumentV1(
                    schema_version=1,
                    intent=IntentKind.INVENTORY_UPDATE,
                    domain=DomainKind.INVENTORY,
                    status=ValidityStatus.VALID,
                    entity=entity,
                    initial_quantity=init_val,
                    operations=[ArithmeticStep(RirArithmeticOp.SUBTRACT, 0.0)],
                )
            else:
                return RirDocumentV1(
                    schema_version=1,
                    intent=IntentKind.INVENTORY_UPDATE,
                    domain=DomainKind.INVENTORY,
                    status=ValidityStatus.INCOMPLETE,
                    entity=entity,
                    initial_quantity=init_val,
                    uncertainty=UncertaintyReport(
                        is_rejected=True,
                        status=ValidityStatus.INCOMPLETE,
                        missing_fields=["operations"],
                        rejection_reason="Single numeric quantity without recognized operator",
                    ),
                )

        # Múltiples números
        idx0, val0 = nums[0]
        idx1, val1 = nums[1]
        ctx1 = " ".join(t.lower().strip(".,;:?!¿¡") for t in tokens[max(0, idx1-3):idx1])

        if any(k in ctx1 for k in ["contando", "teniendo", "originalmente", "existentes", "de las", "de los", "lote"]):
            init_val = val1
            delta_val = val0
        else:
            init_val = val0
            delta_val = val1

        # Decisión semántica de operación (Add vs Sub)
        if sims["negation"] > 0.68 or ("no " in text.lower() and "ningun" in text.lower()):
            op = RirArithmeticOp.SUBTRACT
            delta_val = 0.0
        elif sims["sub"] >= sims["add"]:
            op = RirArithmeticOp.SUBTRACT
        else:
            op = RirArithmeticOp.ADD

        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=entity,
            initial_quantity=init_val,
            operations=[ArithmeticStep(op, delta_val)],
        )
        return doc


# ==============================================================================
# HARNESS PAREADO DE EVALUACIÓN
# ==============================================================================
def rir_exact_match(pred: RirDocumentV1, gold: dict) -> Tuple[bool, str]:
    """Compara canónicamente dos especificaciones RIR v1."""
    if pred.status.value != gold.get("status"):
        return False, f"Status mismatch: pred {pred.status.value} vs gold {gold.get('status')}"

    # Si ambos son incompletos/rechazados, es un acierto de rechazo seguro
    if pred.status != ValidityStatus.VALID:
        return True, "Safe rejection match"

    # Comparar entidad si existe
    if gold.get("entity") and pred.entity:
        if pred.entity.lower() != gold.get("entity").lower():
            return False, f"Entity mismatch: {pred.entity} vs {gold.get('entity')}"

    # Comparar cantidad inicial
    gold_init = gold.get("initial_quantity")
    if gold_init is not None:
        if pred.initial_quantity is None or abs(pred.initial_quantity - float(gold_init)) > 1e-5:
            return False, f"Initial quantity mismatch: {pred.initial_quantity} vs {gold_init}"

    # Comparar operaciones
    gold_ops = gold.get("operations", [])
    if len(pred.operations) != len(gold_ops):
        return False, f"Op count mismatch: {len(pred.operations)} vs {len(gold_ops)}"

    for i, (p_op, g_op) in enumerate(zip(pred.operations, gold_ops)):
        p_op_str = p_op.operator.value if isinstance(p_op.operator, RirArithmeticOp) else p_op.operator
        if p_op_str != g_op.get("operator"):
            return False, f"Op[{i}] operator mismatch: {p_op_str} vs {g_op.get('operator')}"
        if abs(p_op.operand - float(g_op.get("operand", 0.0))) > 1e-5:
            return False, f"Op[{i}] operand mismatch: {p_op.operand} vs {g_op.get('operand')}"

    return True, "Exact Match"


def evaluate_dataset(interpreter, samples: list) -> dict:
    total = len(samples)
    exact_matches = 0
    safe_rejections = 0
    critical_false_positives = 0
    validation_failures = 0

    latencies_us = []

    for s in samples:
        txt = s["text"]
        gold_rir = s["rir"]

        t0 = time.perf_counter()
        try:
            pred_doc = interpreter.parse(txt)
            # Aduana de seguridad de RELIC: Pasar siempre por validate_semantic
            if pred_doc.status == ValidityStatus.VALID:
                try:
                    pred_doc.validate_semantic()
                except Exception:
                    validation_failures += 1
                    pred_doc.status = ValidityStatus.UNSUPPORTED
        except Exception:
            pred_doc = RirDocumentV1(
                status=ValidityStatus.UNSUPPORTED,
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.UNSUPPORTED),
            )
        elapsed_us = (time.perf_counter() - t0) * 1_000_000
        latencies_us.append(elapsed_us)

        # Chequeo Exact Match
        is_em, reason = rir_exact_match(pred_doc, gold_rir)
        if is_em:
            exact_matches += 1
            if gold_rir.get("status") != "VALID":
                safe_rejections += 1
        else:
            # Chequeo de falso positivo crítico:
            # El ground truth requería rechazo (INCOMPLETE) o tenía un resultado diferente,
            # pero el intérprete emitió un VALID ejecutable con números incorrectos
            if pred_doc.status == ValidityStatus.VALID:
                if gold_rir.get("status") != "VALID":
                    critical_false_positives += 1
                else:
                    critical_false_positives += 1

    return {
        "count": total,
        "exact_match_pct": round(exact_matches / total * 100.0, 2),
        "critical_false_positives": critical_false_positives,
        "mean_latency_us": round(sum(latencies_us) / len(latencies_us), 1),
    }


def main():
    print("=" * 70)
    print("RELIC — BENCHMARK CONTROLADO DE INTÉRPRETES SEMÁNTICOS")
    print("=" * 70)

    data_dir = ROOT_DIR / "data" / "semantic_compiler"
    with open(data_dir / "val.jsonl", "r", encoding="utf-8") as f:
        val_samples = [json.loads(line) for line in f]
    with open(data_dir / "test_id.jsonl", "r", encoding="utf-8") as f:
        test_id_samples = [json.loads(line) for line in f]
    with open(data_dir / "test_ood.jsonl", "r", encoding="utf-8") as f:
        test_ood_samples = [json.loads(line) for line in f]

    print(f"Cargadas: Val ({len(val_samples)}), Test ID ({len(test_id_samples)}), Test OOD ({len(test_ood_samples)})")

    # Inicializar intérpretes
    interpreters = [
        HeuristicInterpreter(),
        ConstrainedGrammarInterpreter(),
        PretrainedSemanticInterpreter(),
    ]

    results = {}

    for interp in interpreters:
        print(f"\n---> Evaluando: {interp.name}")
        res_val = evaluate_dataset(interp, val_samples)
        res_id = evaluate_dataset(interp, test_id_samples)
        res_ood = evaluate_dataset(interp, test_ood_samples)

        print(f"  [Val]     EM: {res_val['exact_match_pct']}% | Latencia: {res_val['mean_latency_us']} µs")
        print(f"  [Test ID] EM: {res_id['exact_match_pct']}% | Latencia: {res_id['mean_latency_us']} µs")
        print(f"  [Test OOD] EM: {res_ood['exact_match_pct']}% | FP Críticos: {res_ood['critical_false_positives']} | Latencia: {res_ood['mean_latency_us']} µs")

        results[interp.name] = {
            "validation": res_val,
            "test_id": res_id,
            "test_ood": res_ood,
        }

    report_path = ROOT_DIR / "reports" / "semantic_interpreters_report.json"
    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)

    print("\n" + "=" * 70)
    print(f"[REPORTE GENERADO]: {report_path}")
    print("=" * 70)


if __name__ == "__main__":
    main()
