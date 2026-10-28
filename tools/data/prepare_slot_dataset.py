import json
import random
import re
from pathlib import Path
from tokenizers import Tokenizer

ROOT = Path(__file__).resolve().parent.parent.parent
DATA_DIR = ROOT / "data" / "semantic_compiler"
TOKENIZER_PATH = ROOT / "artifacts" / "tokenizer" / "tokenizer.json"

SLOT_LABELS = [
    "O",
    "B-INIT_QTY", "I-INIT_QTY",
    "B-ENTITY", "I-ENTITY",
    "B-OP_ADD", "I-OP_ADD",
    "B-OP_SUB", "I-OP_SUB",
    "B-ORIGIN", "I-ORIGIN",
    "B-TARGET", "I-TARGET",
    "B-VIA", "I-VIA",
    "B-AVOID", "I-AVOID",
    "B-NEGATION", "I-NEGATION",
]

INTENT_LABELS = [
    "INVENTORY_UPDATE",
    "GRAPH_QUERY",
    "ARITHMETIC_QUERY",
    "RELATIONAL_QUERY",
    "INCOMPLETE",
    "UNSUPPORTED",
]

def align_char_spans_to_tokens(text, spans, token_offsets):
    """spans: list of (start_char, end_char, slot_name)"""
    tags = ["O"] * len(token_offsets)
    for s_start, s_end, slot_name in sorted(spans, key=lambda x: x[0]):
        first = True
        for idx, (t_start, t_end) in enumerate(token_offsets):
            if t_start >= t_end:
                continue
            if max(s_start, t_start) < min(s_end, t_end):
                tag = f"B-{slot_name}" if first else f"I-{slot_name}"
                tags[idx] = tag
                first = False
    return tags

def annotate_sample(item, tokenizer):
    text = item["text"]
    rir = item.get("rir") or item.get("gold_rir", {})
    intent_raw = rir.get("intent", "").upper()
    status_raw = rir.get("status", "").upper()

    if status_raw in ("INCOMPLETE", "AMBIGUOUS"):
        intent = "INCOMPLETE"
    elif status_raw == "UNSUPPORTED" or intent_raw in ("UNKNOWN", "UNSUPPORTED"):
        intent = "UNSUPPORTED"
    elif intent_raw == "INVENTORY_UPDATE":
        intent = "INVENTORY_UPDATE"
    elif intent_raw in ("GRAPH_QUERY", "REACHABILITY"):
        intent = "GRAPH_QUERY"
    elif intent_raw == "ARITHMETIC_QUERY":
        intent = "ARITHMETIC_QUERY"
    else:
        intent = "UNSUPPORTED"

    encoding = tokenizer.encode(text)
    token_offsets = encoding.offsets
    spans = []

    # Inventory slots
    if intent == "INVENTORY_UPDATE" and status_raw == "VALID":
        init_qty = rir.get("initial_quantity")
        entity = rir.get("entity")
        ops = rir.get("operations", [])

        # Find entity span
        if entity:
            idx = text.lower().find(entity.lower())
            if idx != -1:
                spans.append((idx, idx + len(entity), "ENTITY"))

        # Match numbers in text to operations
        num_matches = list(re.finditer(r"\b\d+(?:\.\d+)?\b", text))
        matched_indices = set()

        # Initial quantity
        if init_qty is not None:
            for i, m in enumerate(num_matches):
                if i not in matched_indices and abs(float(m.group(0)) - init_qty) < 1e-5:
                    spans.append((m.start(), m.end(), "INIT_QTY"))
                    matched_indices.add(i)
                    break

        # Operations
        for op in ops:
            op_kind = op.get("operator", "add")
            slot_name = "OP_ADD" if op_kind in ("add", "+") else "OP_SUB"
            operand_val = op.get("operand", 0.0)

            for i, m in enumerate(num_matches):
                if i not in matched_indices and abs(float(m.group(0)) - operand_val) < 1e-5:
                    spans.append((m.start(), m.end(), slot_name))
                    matched_indices.add(i)
                    break

    # Graph slots
    elif intent == "GRAPH_QUERY" and status_raw == "VALID":
        gspec = rir.get("graph_spec", {})
        start_node = gspec.get("start_node")
        target_node = gspec.get("target_node")
        via = gspec.get("via_node")
        avoid = gspec.get("avoid_node")
        is_neg = gspec.get("is_negated", False)

        for node_val, slot_type in [(start_node, "ORIGIN"), (target_node, "TARGET"), (via, "VIA"), (avoid, "AVOID")]:
            if node_val:
                idx = text.lower().find(node_val.lower())
                if idx != -1:
                    spans.append((idx, idx + len(node_val), slot_type))

        if is_neg:
            for neg_kw in ["no es posible", "es imposible", "no se puede", "sin ruta", "no es factible"]:
                idx = text.lower().find(neg_kw)
                if idx != -1:
                    spans.append((idx, idx + len(neg_kw), "NEGATION"))
                    break

    tags = align_char_spans_to_tokens(text, spans, token_offsets)
    return {
        "text": text,
        "token_ids": encoding.ids,
        "tokens": encoding.tokens,
        "intent": intent,
        "slot_tags": tags,
        "family": item.get("family", ""),
        "gold_rir": rir,
    }

def generate_synthetic_corpus():
    """Genera ejemplos variados sintéticos para balancear intenciones y slots."""
    cities = ["Bogotá", "Lima", "Quito", "Caracas", "Medellín", "Cali", "Madrid", "Barcelona", "París", "Roma", "Tokio", "Berlín", "Buenos Aires", "Santiago", "Marta", "Cartagena"]
    entities = ["manzanas", "caramelos", "libros", "lápices", "monedas", "galletas", "chocolates", "naranjas", "peras", "juguetes"]
    
    samples = []
    random.seed(42)

    # 1. Graph Queries (~150)
    for _ in range(160):
        c1, c2 = random.sample(cities, 2)
        has_via = random.random() < 0.25
        has_avoid = random.random() < 0.25
        is_neg = random.random() < 0.2
        via_city = random.choice([c for c in cities if c not in (c1, c2)]) if has_via else None
        avoid_city = random.choice([c for c in cities if c not in (c1, c2, via_city)]) if has_avoid else None

        templates = [
            f"¿Se puede ir de {c1} a {c2}?",
            f"¿Existe una ruta directa de {c1} hacia {c2}?",
            f"¿Es posible llegar de {c1} a {c2}?",
            f"¿Cómo llegar desde {c1} hasta {c2}?",
            f"¿Hay camino partiendo de {c1} y alcanzando {c2}?",
            f"Quiero viajar de {c1} a {c2}.",
            f"¿Conecta {c1} con {c2}?",
            f"¿Podemos ir de {c1} a {c2}?",
        ]
        text = random.choice(templates)
        if has_via:
            text = text.replace("?", f" pasando por {via_city}?")
        if has_avoid:
            text = text.replace("?", f" sin pasar por {avoid_city}?")
        if is_neg:
            text = text.replace("¿Se puede", "¿Es imposible").replace("¿Es posible", "¿No es posible").replace("¿Existe", "¿No existe")

        rir = {
            "schema_version": 1,
            "intent": "graph_query",
            "domain": "spatial_graph",
            "status": "VALID",
            "graph_spec": {
                "start_node": c1,
                "target_node": c2,
                "via_node": via_city,
                "avoid_node": avoid_city,
                "constraints": [],
                "is_negated": is_neg,
            }
        }
        samples.append({"text": text, "rir": rir, "family": "synthetic_graph"})

    # 2. Rich Inventory Queries with diverse verbs & distractors (~180)
    add_verbs = ["me regalan", "me regalaron", "me dan", "me dieron", "compro", "compré", "adquiero", "recibo", "agrego"]
    sub_verbs = ["regalo", "regalé", "comí", "como", "me comí", "di", "le di", "vendo", "vendí", "gasto", "gasté", "pierdo", "perdí"]
    distractors = ["de las más rojas", "de las más dulces", "de los más grandes", "más maduras", "más brillantes", ""]

    for _ in range(180):
        ent = random.choice(entities)
        q0 = random.randint(5, 50)
        n_ops = random.choice([1, 2])
        ops = []
        clause_parts = []
        q_curr = q0

        # Start clause
        start_templates = [
            f"Tengo {q0} {ent}",
            f"Cuento con {q0} {ent}",
            f"Poseo {q0} {ent}",
            f"Hay {q0} {ent}",
            f"Tenía {q0} {ent}",
        ]
        text_accum = random.choice(start_templates)

        for _ in range(n_ops):
            is_add = random.random() < 0.5
            op_q = random.randint(1, min(10, q_curr if not is_add else 20))
            dist = random.choice(distractors)
            dist_str = f" {dist}" if dist else ""

            if is_add:
                verb = random.choice(add_verbs)
                clause_parts.append(f"{verb} {op_q}{dist_str}")
                ops.append({"operator": "add", "operand": float(op_q)})
                q_curr += op_q
            else:
                verb = random.choice(sub_verbs)
                clause_parts.append(f"{verb} {op_q}{dist_str}")
                ops.append({"operator": "subtract", "operand": float(op_q)})
                q_curr = max(0, q_curr - op_q)

        text = text_accum + ", " + " y ".join(clause_parts) + "."
        rir = {
            "schema_version": 1,
            "intent": "inventory_update",
            "domain": "inventory",
            "status": "VALID",
            "entity": ent,
            "initial_quantity": float(q0),
            "operations": ops,
        }
        samples.append({"text": text, "rir": rir, "family": "synthetic_inventory"})

    # 3. Incomplete / Ambiguous Queries (~60)
    incomplete_texts = [
        ("Tengo algunas manzanas y compro 3.", ["initial_quantity"]),
        ("Hay varios caramelos y regalo 2.", ["initial_quantity"]),
        ("Poseo ciertos libros y me dan 5.", ["initial_quantity"]),
        ("Tengo 10 lápices.", ["operations"]),
        ("¿Se puede ir desde Bogotá?", ["target"]),
        ("¿Cómo llegar hasta París?", ["origin"]),
        ("¿Existe camino partiendo de Madrid?", ["target"]),
        ("¿Hay ruta hasta Lima?", ["origin"]),
        ("5 + ?", ["operand"]),
        ("10 - ?", ["operand"]),
        ("8 * ?", ["operand"]),
    ]
    for txt, missing in incomplete_texts:
        for _ in range(5):
            samples.append({
                "text": txt,
                "rir": {
                    "schema_version": 1,
                    "intent": "unknown",
                    "domain": "unsupported",
                    "status": "INCOMPLETE",
                    "uncertainty": {
                        "is_rejected": True,
                        "status": "INCOMPLETE",
                        "missing_fields": missing,
                        "rejection_reason": "Faltan elementos requeridos.",
                    }
                },
                "family": "synthetic_incomplete"
            })

    # 4. Unsupported Queries (~40)
    unsupported_texts = [
        "¿Quién descubrió América?",
        "Cuéntame una historia sobre dragones.",
        "¿Cuál es el sentido de la vida?",
        "Escribe un poema en verso libre.",
        "Dime la receta de una tarta de manzana.",
        "¿Qué hora es en Tokio en este momento?",
        "Traduce esta frase al francés.",
        "Explica la teoría de la relatividad general.",
    ]
    for txt in unsupported_texts:
        for _ in range(5):
            samples.append({
                "text": txt,
                "rir": {
                    "schema_version": 1,
                    "intent": "unknown",
                    "domain": "unsupported",
                    "status": "UNSUPPORTED",
                    "uncertainty": {
                        "is_rejected": True,
                        "status": "UNSUPPORTED",
                        "missing_fields": [],
                        "rejection_reason": "Fuera del dominio soportado.",
                    }
                },
                "family": "synthetic_unsupported"
            })

    return samples

def main():
    tokenizer = Tokenizer.from_file(str(TOKENIZER_PATH))
    out_dir = DATA_DIR / "slot_tagged"
    out_dir.mkdir(parents=True, exist_ok=True)

    # Load existing samples
    existing_samples = {"train": [], "val": [], "test_id": [], "test_ood": []}
    for split in ["train", "val", "test_id", "test_ood"]:
        inp_file = DATA_DIR / f"{split}.jsonl"
        if inp_file.exists():
            with open(inp_file, "r", encoding="utf-8") as fin:
                for line in fin:
                    if line.strip():
                        existing_samples[split].append(json.loads(line))

    # Generate synthetic samples and partition
    synthetic = generate_synthetic_corpus()
    random.shuffle(synthetic)

    # 70% train, 15% val, 15% ood
    n = len(synthetic)
    n_train = int(n * 0.70)
    n_val = int(n * 0.15)
    
    syn_train = synthetic[:n_train]
    syn_val = synthetic[n_train:n_train + n_val]
    syn_ood = synthetic[n_train + n_val:]

    all_data = {
        "train": existing_samples["train"] + syn_train,
        "val": existing_samples["val"] + syn_val,
        "test_id": existing_samples["test_id"],
        "test_ood": existing_samples["test_ood"] + syn_ood,
    }

    for split, items in all_data.items():
        out_file = out_dir / f"{split}_slots.jsonl"
        count = 0
        with open(out_file, "w", encoding="utf-8") as fout:
            for item in items:
                annotated = annotate_sample(item, tokenizer)
                fout.write(json.dumps(annotated, ensure_ascii=False) + "\n")
                count += 1
        print(f"Processed {split}: {count} samples -> {out_file.name}")

if __name__ == "__main__":
    main()
