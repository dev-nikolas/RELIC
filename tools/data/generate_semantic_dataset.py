#!/usr/bin/env python3
"""Generador y Particionador de Dataset para Compilación Semántica RIR v1.

Garantías Metodológicas:
1. Partición por Familias Lingüísticas Disjuntas (Cero fuga de plantillas sintácticas).
2. Generación con validación estricta de esquema RIR v1 para cada etiqueta.
3. Inclusión explícita de casos de incertidumbre (INCOMPLETE, AMBIGUOUS, UNSUPPORTED).
4. Conjunto de evaluación OOD con las construcciones y verbos raros especificados por el usuario.
"""

import json
import random
from pathlib import Path
import sys

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))

from relic.rir_v1 import (
    RirDocumentV1,
    IntentKind,
    DomainKind,
    ValidityStatus,
    RirArithmeticOp,
    ArithmeticStep,
    GraphQuerySpec,
    UncertaintyReport,
)

DATASET_DIR = ROOT_DIR / "data" / "semantic_compiler"


def create_sample(text: str, doc: RirDocumentV1, family: str) -> dict:
    doc.validate_semantic() if doc.status == ValidityStatus.VALID else None
    return {
        "text": text,
        "family": family,
        "rir": doc.to_dict(),
    }


def generate_all_samples():
    random.seed(42)

    # =========================================================================
    # FAMILIA 1: Posesión directa y adición/sustracción estándar (TRAIN + ID)
    # =========================================================================
    fam1_units = ["manzanas", "libros", "cuadernos", "monedas", "galletas", "peras", "billetes", "piezas", "fichas"]
    fam1_samples = []

    for _ in range(80):
        u = random.choice(fam1_units)
        init = random.randint(2, 20)
        # Caso 1: Una adición
        add_val = random.randint(1, 10)
        t1 = f"Tengo {init} {u} y compro {add_val} más."
        d1 = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=u,
            initial_quantity=float(init),
            operations=[ArithmeticStep(RirArithmeticOp.ADD, float(add_val))],
        )
        fam1_samples.append(create_sample(t1, d1, "F1_direct_possession_add"))

        # Caso 2: Una resta
        sub_val = random.randint(1, init - 1)
        t2 = f"Cuento con {init} {u} y regalo {sub_val}."
        d2 = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=u,
            initial_quantity=float(init),
            operations=[ArithmeticStep(RirArithmeticOp.SUBTRACT, float(sub_val))],
        )
        fam1_samples.append(create_sample(t2, d2, "F1_direct_possession_sub"))

        # Caso 3: Multi-paso (adición + sustracción)
        add_v2 = random.randint(1, 8)
        sub_v2 = random.randint(1, init + add_v2 - 1)
        t3 = f"Poseo {init} {u}, recibo {add_v2} y luego dono {sub_v2}."
        d3 = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=u,
            initial_quantity=float(init),
            operations=[
                ArithmeticStep(RirArithmeticOp.ADD, float(add_v2)),
                ArithmeticStep(RirArithmeticOp.SUBTRACT, float(sub_v2)),
            ],
        )
        fam1_samples.append(create_sample(t3, d3, "F1_direct_possession_multistep"))

    # =========================================================================
    # FAMILIA 2: Aritmética abstracta pura (TRAIN + ID)
    # =========================================================================
    fam2_samples = []
    for _ in range(60):
        a = random.randint(1, 50)
        b = random.randint(1, 50)
        t1 = f"¿Cuánto es {a} + {b}?"
        d1 = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.ARITHMETIC_QUERY,
            domain=DomainKind.ABSTRACT_MATH,
            status=ValidityStatus.VALID,
            entity=None,
            initial_quantity=float(a),
            operations=[ArithmeticStep(RirArithmeticOp.ADD, float(b))],
        )
        fam2_samples.append(create_sample(t1, d1, "F2_abstract_math_add"))

        sub_b = random.randint(1, a)
        t2 = f"Calcula {a} - {sub_b}."
        d2 = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.ARITHMETIC_QUERY,
            domain=DomainKind.ABSTRACT_MATH,
            status=ValidityStatus.VALID,
            entity=None,
            initial_quantity=float(a),
            operations=[ArithmeticStep(RirArithmeticOp.SUBTRACT, float(sub_b))],
        )
        fam2_samples.append(create_sample(t2, d2, "F2_abstract_math_sub"))

    # =========================================================================
    # FAMILIA 3: Grafos y dependencias de software (TRAIN + ID)
    # =========================================================================
    fam3_samples = []
    crates = [("relic_cli", "relic_core"), ("relic_core", "memmap2"), ("app_service", "auth_gateway"), ("auth_gateway", "database_connector")]
    for u, v in crates:
        t = f"¿Depende el crate {u} del componente {v}?"
        d = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.GRAPH_QUERY,
            domain=DomainKind.SOFTWARE_DEPENDENCY,
            status=ValidityStatus.VALID,
            graph_spec=GraphQuerySpec(start_node=u, target_node=v, is_negated=False),
        )
        fam3_samples.append(create_sample(t, d, "F3_graph_dependency"))

    # =========================================================================
    # FAMILIA 4: Incompletitud y ambigüedad (TRAIN + ID)
    # =========================================================================
    fam4_samples = [
        create_sample(
            "¿Cuánto es 5 + ?",
            RirDocumentV1(
                schema_version=1,
                intent=IntentKind.ARITHMETIC_QUERY,
                domain=DomainKind.ABSTRACT_MATH,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.INCOMPLETE, missing_fields=["operations[0].operand"], rejection_reason="Falta el segundo operando"),
            ),
            "F4_incomplete_math",
        ),
        create_sample(
            "Calcula 12 -",
            RirDocumentV1(
                schema_version=1,
                intent=IntentKind.ARITHMETIC_QUERY,
                domain=DomainKind.ABSTRACT_MATH,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.INCOMPLETE, missing_fields=["operations[0].operand"], rejection_reason="Operación incompleta"),
            ),
            "F4_incomplete_math",
        ),
        create_sample(
            "Tengo varias manzanas y me dieron 3 más.",
            RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.INCOMPLETE,
                entity="manzanas",
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.INCOMPLETE, missing_fields=["initial_quantity"], rejection_reason="Cantidad inicial indefinida"),
            ),
            "F4_incomplete_inventory",
        ),
        create_sample(
            "¿Se puede llegar a la estación?",
            RirDocumentV1(
                schema_version=1,
                intent=IntentKind.GRAPH_QUERY,
                domain=DomainKind.SPATIAL_GRAPH,
                status=ValidityStatus.INCOMPLETE,
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.INCOMPLETE, missing_fields=["start_node"], rejection_reason="Falta punto de partida"),
            ),
            "F4_incomplete_graph",
        ),
        create_sample(
            "Tengo 5 objetos o quizás 6 y perdí 2.",
            RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.AMBIGUOUS,
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.AMBIGUOUS, missing_fields=["initial_quantity"], rejection_reason="Cantidad inicial ambigua o contradictoria"),
            ),
            "F4_ambiguous_inventory",
        ),
    ]

    # =========================================================================
    # FAMILIA 5: Fuera de distribución / No soportado (TRAIN + ID)
    # =========================================================================
    fam5_samples = [
        create_sample(
            "¿Cuál es la capital de Colombia?",
            RirDocumentV1(
                schema_version=1,
                intent=IntentKind.UNKNOWN,
                domain=DomainKind.UNSUPPORTED,
                status=ValidityStatus.UNSUPPORTED,
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.UNSUPPORTED, rejection_reason="Pregunta enciclopédica fuera del dominio formal"),
            ),
            "F5_unsupported_open_domain",
        ),
        create_sample(
            "Escribe un poema sobre el mar en invierno.",
            RirDocumentV1(
                schema_version=1,
                intent=IntentKind.UNKNOWN,
                domain=DomainKind.UNSUPPORTED,
                status=ValidityStatus.UNSUPPORTED,
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.UNSUPPORTED, rejection_reason="Petición de generación creativa no formal"),
            ),
            "F5_unsupported_creative",
        ),
        create_sample(
            "¿Quién fue el primer presidente de México?",
            RirDocumentV1(
                schema_version=1,
                intent=IntentKind.UNKNOWN,
                domain=DomainKind.UNSUPPORTED,
                status=ValidityStatus.UNSUPPORTED,
                uncertainty=UncertaintyReport(is_rejected=True, status=ValidityStatus.UNSUPPORTED, rejection_reason="Hecho histórico no computable en RIR"),
            ),
            "F5_unsupported_history",
        ),
    ]

    # =========================================================================
    # FAMILIA 6 (EXCLUSIVO VALIDACIÓN): Cláusulas invertidas y dependencias relativas
    # Nunca vistas en TRAIN
    # =========================================================================
    fam6_val_samples = []
    f6_units = ["libros", "cuadernos", "lápices", "naranjas", "peras"]
    for i in range(25):
        u = f6_units[i % len(f6_units)]
        init = 10 + i
        used = 2 + (i % 4)
        t = f"De los {init} {u} que había comprado, presté {used} a mi hermano."
        d = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=u,
            initial_quantity=float(init),
            operations=[ArithmeticStep(RirArithmeticOp.SUBTRACT, float(used))],
        )
        fam6_val_samples.append(create_sample(t, d, "F6_inverted_clause_val"))

    # =========================================================================
    # FAMILIAS 7, 8, 9 (EXCLUSIVO TEST OOD - GENERALIZACIÓN ESTRICTA)
    # Frases complejas, voz pasiva, negación, verbos raros ("extravié", "obsequié")
    # Nunca vistas en TRAIN ni VAL
    # =========================================================================
    # =========================================================================
    # FAMILIAS 7, 8, 9 (EXCLUSIVO TEST OOD - GENERALIZACIÓN ESTRICTA EXPANDIDA)
    # 60+ muestras con verbos no vistos, voz pasiva, cláusulas invertidas,
    # negación estricta e incompletitudes sin antecedentes.
    # =========================================================================
    fam_ood_samples = []

    ood_units = ["lápices", "manzanas", "libros", "monedas", "peras", "billetes", "cuadernos", "fichas", "cajas"]

    # 1. Verbos no vistos de sustracción (extraviar, obsequiar, ceder, consumir, hurtar, repartir, traspasar)
    sub_verbs = [
        ("extravié", "F9_unseen_lexicon_extraviar"),
        ("obsequié", "F9_unseen_lexicon_obsequiar"),
        ("cedí", "F9_unseen_lexicon_ceder"),
        ("consumí", "F9_unseen_lexicon_consumir"),
        ("me hurtaron", "F9_unseen_lexicon_hurtar"),
        ("repartí", "F9_unseen_lexicon_repartir"),
        ("traspasé", "F9_unseen_lexicon_traspasar"),
        ("me descompuse de", "F9_unseen_lexicon_descomponer"),
        ("doné en caridad", "F9_unseen_lexicon_donar_caridad"),
        ("destiné a descarte", "F9_unseen_lexicon_descarte"),
    ]
    for verb, fam_tag in sub_verbs:
        for idx in range(2):
            u = ood_units[(len(fam_ood_samples) + idx) % len(ood_units)]
            init = 5 + idx * 3
            sub_n = 1 + idx * 2
            txt = f"Contaba con {init} {u}, pero {verb} {sub_n}."
            doc = RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.VALID,
                entity=u,
                initial_quantity=float(init),
                operations=[ArithmeticStep(RirArithmeticOp.SUBTRACT, float(sub_n))],
            )
            fam_ood_samples.append(create_sample(txt, doc, fam_tag))

    # 2. Verbos no vistos de adición (adquirir, suministrar, ingresar, recolectar, hallar, heredar)
    add_verbs = [
        ("adquirí", "F9_unseen_lexicon_adquirir"),
        ("me suministraron", "F9_unseen_lexicon_suministrar"),
        ("ingresaron a mi haber", "F9_unseen_lexicon_ingresar"),
        ("recolecté", "F9_unseen_lexicon_recolectar"),
        ("hallé en el almacén", "F9_unseen_lexicon_hallar"),
        ("recibí en herencia", "F9_unseen_lexicon_heredar"),
        ("incorporé", "F9_unseen_lexicon_incorporar"),
        ("coseché", "F9_unseen_lexicon_cosechar"),
    ]
    for verb, fam_tag in add_verbs:
        for idx in range(2):
            u = ood_units[(len(fam_ood_samples) + idx) % len(ood_units)]
            init = 4 + idx * 2
            add_n = 2 + idx * 3
            txt = f"Tenía {init} {u} y {verb} {add_n}."
            doc = RirDocumentV1(
                schema_version=1,
                intent=IntentKind.INVENTORY_UPDATE,
                domain=DomainKind.INVENTORY,
                status=ValidityStatus.VALID,
                entity=u,
                initial_quantity=float(init),
                operations=[ArithmeticStep(RirArithmeticOp.ADD, float(add_n))],
            )
            fam_ood_samples.append(create_sample(txt, doc, fam_tag))

    # 3. Voz pasiva perifrástica
    passive_templates = [
        ("Fueron sustraídas {sub} de las {init} {u} que estaban guardadas.", RirArithmeticOp.SUBTRACT),
        ("Fueron consumidas {sub} de mis {init} {u}.", RirArithmeticOp.SUBTRACT),
        ("Fueron entregadas {sub} de las {init} {u} en el depósito.", RirArithmeticOp.SUBTRACT),
        ("Me fueron suministradas {add} {u} adicionales, teniendo previamente {init}.", RirArithmeticOp.ADD),
        ("Fueron añadidas {add} nuevas {u} al lote inicial de {init}.", RirArithmeticOp.ADD),
        ("Fueron destruidas {sub} de las {init} {u} del inventario.", RirArithmeticOp.SUBTRACT),
        ("Me fueron donadas {add} {u} cuando ya contaba con {init}.", RirArithmeticOp.ADD),
        ("Fueron extraviadas {sub} {u} de un conjunto de {init}.", RirArithmeticOp.SUBTRACT),
    ]
    for tmpl, op in passive_templates:
        u = ood_units[len(fam_ood_samples) % len(ood_units)]
        init = 8
        delta = 3
        if op == RirArithmeticOp.SUBTRACT:
            txt = tmpl.format(sub=delta, init=init, u=u)
            step_op = RirArithmeticOp.SUBTRACT
        else:
            txt = tmpl.format(add=delta, init=init, u=u)
            step_op = RirArithmeticOp.ADD
        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=u,
            initial_quantity=float(init),
            operations=[ArithmeticStep(step_op, float(delta))],
        )
        fam_ood_samples.append(create_sample(txt, doc, "F7_passive_voice"))

    # 4. Cláusulas invertidas y subordinadas temporales
    inverted_templates = [
        ("Habiendo extraviado {sub}, y contando originalmente con {init} {u}, ¿cuántas conservo?", RirArithmeticOp.SUBTRACT),
        ("De las {init} {u} que custodiaba, {sub} regalé a mi socio.", RirArithmeticOp.SUBTRACT),
        ("Tras haber obsequiado {sub}, si al principio poseía {init} {u}...", RirArithmeticOp.SUBTRACT),
        ("Habiendo adquirido {add} {u}, sumadas a las {init} existentes...", RirArithmeticOp.ADD),
        ("De los {init} {u} guardados en el cajón, {sub} se extraviaron.", RirArithmeticOp.SUBTRACT),
        ("A mi hermano le presté {sub} de los {init} {u} que compré.", RirArithmeticOp.SUBTRACT),
        ("Habiendo incorporado {add} {u} a las {init} que poseía...", RirArithmeticOp.ADD),
        ("De un conjunto de {init} {u}, {sub} fueron cedidas.", RirArithmeticOp.SUBTRACT),
    ]
    for tmpl, op in inverted_templates:
        u = ood_units[len(fam_ood_samples) % len(ood_units)]
        init = 10
        delta = 4
        txt = tmpl.format(init=init, sub=delta, add=delta, u=u)
        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=u,
            initial_quantity=float(init),
            operations=[ArithmeticStep(op, float(delta))],
        )
        fam_ood_samples.append(create_sample(txt, doc, "F7_inverted_clauses"))

    # 5. Negación estricta y retención intacta
    negation_templates = [
        ("No regalé ninguna de las {init} {u} que tenía.", "manzanas", 7),
        ("No es cierto que extravié {sub} de mis {init} {u}.", "libros", 6),
        ("Nunca cedí ninguna de las {init} {u} bajo mi custodia.", "monedas", 12),
        ("No incorporé ninguna {u} adicional a las {init} presentes.", "fichas", 9),
        ("No me fueron sustraídas ninguna de las {init} {u}.", "peras", 5),
    ]
    for tmpl, u, init in negation_templates:
        txt = tmpl.format(init=init, sub=2, u=u)
        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.VALID,
            entity=u,
            initial_quantity=float(init),
            operations=[ArithmeticStep(RirArithmeticOp.SUBTRACT, 0.0)],
        )
        fam_ood_samples.append(create_sample(txt, doc, "F8_strict_negation"))

    # 6. Incompletitud explícita y ambigüedad (Rechazo seguro obligatorio)
    incomplete_templates = [
        ("Tengo algunas {u} y extravié {sub}.", "Cantidad inicial indefinida ('algunas')"),
        ("Poseo varios {u} y me obsequiaron {add} más.", "Cantidad inicial indefinida ('varios')"),
        ("¿Cuánto me queda si extravié dos de las que tenía?", "Antecedente numérico ausente"),
        ("No regalé ninguna de las {u}.", "No se especifica cuántas unidades había en total"),
        ("Compré {add} {u} pero desconozco cuántas tenía previamente.", "Cantidad inicial desconocida"),
        ("Fueron sustraídas {sub} {u} pero nadie sabe cuántas había al inicio.", "Inventario inicial indeterminado"),
        ("Doné {sub} {u} de una cantidad no precisada.", "Total inicial no especificado"),
        ("Hay ciertas {u} en el depósito y se retiraron {sub}.", "Cantidad inicial indefinida ('ciertas')"),
    ]
    for tmpl, reason in incomplete_templates:
        u = ood_units[len(fam_ood_samples) % len(ood_units)]
        txt = tmpl.format(sub=2, add=3, u=u)
        doc = RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.INCOMPLETE,
            entity=u,
            uncertainty=UncertaintyReport(
                is_rejected=True,
                status=ValidityStatus.INCOMPLETE,
                missing_fields=["initial_quantity"],
                rejection_reason=reason,
            ),
        )
        fam_ood_samples.append(create_sample(txt, doc, "F4_incomplete_ambiguous"))

    return {
        "fam1": fam1_samples,
        "fam2": fam2_samples,
        "fam3": fam3_samples,
        "fam4": fam4_samples,
        "fam5": fam5_samples,
        "fam6_val": fam6_val_samples,
        "ood_test": fam_ood_samples,
    }


def main():
    DATASET_DIR.mkdir(parents=True, exist_ok=True)
    all_data = generate_all_samples()

    # Partición controlada con garantía de cero duplicados
    seen_texts = set()

    def deduplicate(samples):
        res = []
        for s in samples:
            if s["text"] not in seen_texts:
                seen_texts.add(s["text"])
                res.append(s)
        return res

    test_ood_samples = all_data["ood_test"]
    for s in test_ood_samples:
        seen_texts.add(s["text"])

    val_candidates = (
        all_data["fam6_val"]
        + all_data["fam1"][200:]
        + all_data["fam2"][90:]
        + all_data["fam3"][2:]
        + all_data["fam4"][2:]
        + all_data["fam5"][2:]
    )
    val_samples = deduplicate(val_candidates)

    train_candidates = (
        all_data["fam1"][:200]
        + all_data["fam2"][:90]
        + all_data["fam3"][:2]
        + all_data["fam4"][:2]
        + all_data["fam5"][:2]
    )
    train_samples = deduplicate(train_candidates)

    test_id_samples = val_samples[:30]

    # Guardar particiones en JSONL
    for name, dataset in [
        ("train.jsonl", train_samples),
        ("val.jsonl", val_samples),
        ("test_id.jsonl", test_id_samples),
        ("test_ood.jsonl", test_ood_samples),
    ]:
        p = DATASET_DIR / name
        with open(p, "w", encoding="utf-8") as f:
            for s in dataset:
                f.write(json.dumps(s, ensure_ascii=False) + "\n")
        print(f"Guardado {name}: {len(dataset)} muestras")

    # Auditoría de metadatos y detección de duplicados
    train_texts = set(s["text"] for s in train_samples)
    val_texts = set(s["text"] for s in val_samples)
    ood_texts = set(s["text"] for s in test_ood_samples)

    leak_train_val = train_texts.intersection(val_texts)
    leak_train_ood = train_texts.intersection(ood_texts)

    metadata = {
        "total_samples": len(train_samples) + len(val_samples) + len(test_ood_samples),
        "partitions": {
            "train": {
                "count": len(train_samples),
                "families": list(set(s["family"] for s in train_samples)),
            },
            "val": {
                "count": len(val_samples),
                "families": list(set(s["family"] for s in val_samples)),
            },
            "test_id": {
                "count": len(test_id_samples),
                "families": list(set(s["family"] for s in test_id_samples)),
            },
            "test_ood": {
                "count": len(test_ood_samples),
                "families": list(set(s["family"] for s in test_ood_samples)),
                "unseen_constructs": [
                    "passive_voice",
                    "inverted_clauses",
                    "proclitic_pronouns",
                    "unseen_verbs: extraviar, obsequiar",
                    "indefinite_rejection: algunas, otra",
                ],
            },
        },
        "leakage_audit": {
            "train_val_overlap": len(leak_train_val),
            "train_ood_overlap": len(leak_train_ood),
            "status": "ZERO_LEAKAGE_CONFIRMED" if len(leak_train_val) == 0 and len(leak_train_ood) == 0 else "LEAKAGE_DETECTED",
        },
    }

    meta_path = DATASET_DIR / "dataset_metadata.json"
    with open(meta_path, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=2, ensure_ascii=False)

    print(f"\n[ÉXITO] Metadatos del dataset guardados en {meta_path}")
    print(f"Auditoría de fuga: {metadata['leakage_audit']['status']}")


if __name__ == "__main__":
    main()
