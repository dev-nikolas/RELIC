import json
from pathlib import Path
from typing import Any, Tuple

import torch
import torch.nn as nn
import torch.nn.functional as F

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

SLOT2ID = {s: i for i, s in enumerate(SLOT_LABELS)}
ID2SLOT = {i: s for i, s in enumerate(SLOT_LABELS)}
INTENT2ID = {it: i for i, it in enumerate(INTENT_LABELS)}
ID2INTENT = {i: it for i, it in enumerate(INTENT_LABELS)}


class RelicSlotFiller(nn.Module):
    """Modelo neuro-simbólico compacto para extracción de slots e intención a RIR."""
    def __init__(
        self,
        vocab_size: int = 8192,
        embed_dim: int = 128,
        hidden_dim: int = 128,
        num_layers: int = 2,
        num_intents: int = len(INTENT_LABELS),
        num_slots: int = len(SLOT_LABELS),
    ):
        super().__init__()
        self.embedding = nn.Embedding(vocab_size, embed_dim, padding_idx=2)
        self.gru = nn.GRU(
            embed_dim,
            hidden_dim // 2,
            num_layers=num_layers,
            batch_first=True,
            bidirectional=True,
        )
        self.intent_head = nn.Sequential(
            nn.Linear(hidden_dim, 64),
            nn.ReLU(),
            nn.Linear(64, num_intents),
        )
        self.slot_head = nn.Sequential(
            nn.Linear(hidden_dim, 64),
            nn.ReLU(),
            nn.Linear(64, num_slots),
        )

    def forward(self, input_ids: torch.Tensor, mask: torch.Tensor | None = None) -> Tuple[torch.Tensor, torch.Tensor]:
        # input_ids: (batch, seq_len)
        embeds = self.embedding(input_ids)  # (batch, seq_len, embed_dim)
        h, _ = self.gru(embeds)            # (batch, seq_len, hidden_dim)

        # Slot logits per token
        slot_logits = self.slot_head(h)     # (batch, seq_len, num_slots)

        # Masked average pooling for Intent
        if mask is not None:
            mask_expanded = mask.unsqueeze(-1).float()  # (batch, seq_len, 1)
            sum_h = torch.sum(h * mask_expanded, dim=1)
            lengths = torch.clamp(mask_expanded.sum(dim=1), min=1.0)
            pooled = sum_h / lengths
        else:
            pooled = torch.mean(h, dim=1)

        intent_logits = self.intent_head(pooled)  # (batch, num_intents)
        return intent_logits, slot_logits


def extract_spans_from_predictions(token_offsets: list[tuple[int, int]], pred_slot_ids: list[int], text: str) -> list[dict[str, Any]]:
    """Convierte predicciones de tokens a spans de texto con sus valores."""
    spans = []
    current_label = None
    span_start = 0
    span_end = 0

    for idx, slot_id in enumerate(pred_slot_ids):
        if idx >= len(token_offsets):
            break
        tag = ID2SLOT.get(slot_id, "O")
        t_start, t_end = token_offsets[idx]
        if t_start >= t_end:
            continue

        slot_name = tag[2:] if tag.startswith(("B-", "I-")) else None
        if slot_name is not None:
            if current_label == slot_name and (span_end == t_start or tag.startswith("I-")):
                span_end = t_end
            else:
                if current_label is not None:
                    spans.append({
                        "slot": current_label,
                        "text": text[span_start:span_end].strip(),
                        "start": span_start,
                        "end": span_end,
                    })
                current_label = slot_name
                span_start = t_start
                span_end = t_end
        else:
            if current_label is not None:
                spans.append({
                    "slot": current_label,
                    "text": text[span_start:span_end].strip(),
                    "start": span_start,
                    "end": span_end,
                })
                current_label = None

    if current_label is not None:
        spans.append({
            "slot": current_label,
            "text": text[span_start:span_end].strip(),
            "start": span_start,
            "end": span_end,
        })

    return spans


def assemble_rir_from_slots(intent_str: str, spans: list[dict[str, Any]], raw_text: str) -> dict[str, Any]:
    """Ensambla el AST RIR canónico sin hardcoding de palabras."""
    if intent_str == "INCOMPLETE":
        return {
            "schema_version": 1,
            "intent": "unknown",
            "domain": "unsupported",
            "status": "INCOMPLETE",
            "uncertainty": {
                "is_rejected": True,
                "status": "INCOMPLETE",
                "missing_fields": ["initial_quantity_or_operation"],
                "rejection_reason": "Consulta incompleta o ambigua detectada por el modelo.",
            },
        }
    elif intent_str == "UNSUPPORTED":
        return {
            "schema_version": 1,
            "intent": "unknown",
            "domain": "unsupported",
            "status": "UNSUPPORTED",
            "uncertainty": {
                "is_rejected": True,
                "status": "UNSUPPORTED",
                "missing_fields": [],
                "rejection_reason": "Consulta fuera del dominio soportado.",
            },
        }
    elif intent_str == "GRAPH_QUERY":
        origin = None
        target = None
        via = None
        avoid = None
        is_negated = False

        for s in spans:
            slot = s["slot"]
            val = s["text"]
            if slot == "ORIGIN" and not origin:
                origin = val
            elif slot == "TARGET" and not target:
                target = val
            elif slot == "VIA" and not via:
                via = val
            elif slot == "AVOID" and not avoid:
                avoid = val
            elif slot == "NEGATION":
                is_negated = True

        if not origin or not target:
            return {
                "schema_version": 1,
                "intent": "graph_query",
                "domain": "spatial_graph",
                "status": "INCOMPLETE",
                "uncertainty": {
                    "is_rejected": True,
                    "status": "INCOMPLETE",
                    "missing_fields": ["origin" if not origin else "target"],
                    "rejection_reason": "No se identificaron los extremos de la ruta.",
                },
            }

        return {
            "schema_version": 1,
            "intent": "graph_query",
            "domain": "spatial_graph",
            "status": "VALID",
            "graph_spec": {
                "start_node": origin,
                "target_node": target,
                "via_node": via,
                "avoid_node": avoid,
                "constraints": [],
                "is_negated": is_negated,
            },
        }

    elif intent_str == "INVENTORY_UPDATE":
        initial_quantity = None
        entity = None
        operations = []

        for s in spans:
            slot = s["slot"]
            val_str = s["text"]

            if slot == "INIT_QTY":
                try:
                    initial_quantity = float(val_str)
                except ValueError:
                    pass
            elif slot == "ENTITY" and not entity:
                entity = val_str
            elif slot in ("OP_ADD", "OP_SUB"):
                try:
                    num_val = float(val_str)
                    op_name = "add" if slot == "OP_ADD" else "subtract"
                    operations.append({"operator": op_name, "operand": num_val})
                except ValueError:
                    pass

        if initial_quantity is None or not operations:
            return {
                "schema_version": 1,
                "intent": "inventory_update",
                "domain": "inventory",
                "status": "INCOMPLETE",
                "uncertainty": {
                    "is_rejected": True,
                    "status": "INCOMPLETE",
                    "missing_fields": ["initial_quantity" if initial_quantity is None else "operations"],
                    "rejection_reason": "Problema de inventario sin cantidad inicial u operaciones claras.",
                },
            }

        return {
            "schema_version": 1,
            "intent": "inventory_update",
            "domain": "inventory",
            "status": "VALID",
            "entity": entity,
            "initial_quantity": initial_quantity,
            "operations": operations,
        }

    return {
        "schema_version": 1,
        "intent": "unknown",
        "domain": "unsupported",
        "status": "UNSUPPORTED",
    }
