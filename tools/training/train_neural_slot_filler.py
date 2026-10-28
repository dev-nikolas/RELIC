import argparse
import json
import random
import sys
from pathlib import Path

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import DataLoader, Dataset
from tokenizers import Tokenizer
from safetensors.torch import save_file

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))

from relic.neural_slot_filler import (
    ID2INTENT,
    ID2SLOT,
    INTENT2ID,
    INTENT_LABELS,
    SLOT2ID,
    SLOT_LABELS,
    RelicSlotFiller,
    assemble_rir_from_slots,
    extract_spans_from_predictions,
)

DATASET_DIR = ROOT_DIR / "data" / "semantic_compiler" / "slot_tagged"
TOKENIZER_PATH = ROOT_DIR / "artifacts" / "tokenizer" / "tokenizer.json"
WEIGHTS_DIR = ROOT_DIR / "artifacts" / "weights"


class SlotTaggedDataset(Dataset):
    def __init__(self, jsonl_path: Path, max_len: int = 64):
        self.samples = []
        with open(jsonl_path, "r", encoding="utf-8") as f:
            for line in f:
                if not line.strip():
                    continue
                item = json.loads(line)
                token_ids = item["token_ids"][:max_len]
                slot_tags = item["slot_tags"][:max_len]
                intent_str = item["intent"]

                slot_ids = [SLOT2ID.get(t, 0) for t in slot_tags]
                intent_id = INTENT2ID.get(intent_str, INTENT2ID["UNSUPPORTED"])

                # Padding
                pad_len = max_len - len(token_ids)
                padded_input = token_ids + [2] * pad_len
                padded_slots = slot_ids + [-100] * pad_len
                mask = [1] * len(token_ids) + [0] * pad_len

                self.samples.append({
                    "raw": item,
                    "input_ids": torch.tensor(padded_input, dtype=torch.long),
                    "slot_ids": torch.tensor(padded_slots, dtype=torch.long),
                    "intent_id": torch.tensor(intent_id, dtype=torch.long),
                    "mask": torch.tensor(mask, dtype=torch.bool),
                })

    def __len__(self):
        return len(self.samples)

    def __getitem__(self, idx):
        return (
            self.samples[idx]["input_ids"],
            self.samples[idx]["slot_ids"],
            self.samples[idx]["intent_id"],
            self.samples[idx]["mask"],
        )


def evaluate_model(model: RelicSlotFiller, dataset: SlotTaggedDataset, tokenizer: Tokenizer, device: torch.device):
    model.eval()
    intent_correct = 0
    slot_correct = 0
    slot_total = 0
    rir_exact_match = 0

    with torch.no_grad():
        for i in range(len(dataset)):
            sample = dataset.samples[i]
            inp = sample["input_ids"].unsqueeze(0).to(device)
            mask = sample["mask"].unsqueeze(0).to(device)
            gold_intent = sample["intent_id"].item()
            gold_slots = sample["slot_ids"]

            intent_logits, slot_logits = model(inp, mask)
            pred_intent = intent_logits.argmax(dim=-1).item()
            pred_slots = slot_logits.argmax(dim=-1)[0].cpu()

            if pred_intent == gold_intent:
                intent_correct += 1

            valid_mask = gold_slots != -100
            slot_correct += (pred_slots[valid_mask] == gold_slots[valid_mask]).sum().item()
            slot_total += valid_mask.sum().item()

            # End-to-end RIR exact match
            raw = sample["raw"]
            encoding = tokenizer.encode(raw["text"])
            offsets = encoding.offsets
            pred_slot_list = pred_slots[:len(offsets)].tolist()
            spans = extract_spans_from_predictions(offsets, pred_slot_list, raw["text"])
            pred_rir = assemble_rir_from_slots(ID2INTENT[pred_intent], spans, raw["text"])

            gold_rir = raw["gold_rir"]
            # Compare key attributes
            if (
                pred_rir.get("intent") == gold_rir.get("intent")
                and pred_rir.get("status") == gold_rir.get("status")
            ):
                if pred_rir.get("status") == "VALID":
                    if pred_rir.get("intent") == "inventory_update":
                        init_ok = abs(pred_rir.get("initial_quantity", -1) - gold_rir.get("initial_quantity", -2)) < 1e-4
                        ops_ok = pred_rir.get("operations") == gold_rir.get("operations")
                        if init_ok and ops_ok:
                            rir_exact_match += 1
                    elif pred_rir.get("intent") == "graph_query":
                        g_pred = pred_rir.get("graph_spec", {})
                        g_gold = gold_rir.get("graph_spec", {})
                        if (
                            g_pred.get("start_node") == g_gold.get("start_node")
                            and g_pred.get("target_node") == g_gold.get("target_node")
                        ):
                            rir_exact_match += 1
                else:
                    rir_exact_match += 1

    n = len(dataset)
    return {
        "intent_acc": round(100.0 * intent_correct / max(1, n), 2),
        "slot_acc": round(100.0 * slot_correct / max(1, slot_total), 2),
        "rir_em": round(100.0 * rir_exact_match / max(1, n), 2),
    }


def train():
    parser = argparse.ArgumentParser(description="Entrenador del Intérprete Semántico Neuronal")
    parser.add_argument("--epochs", type=int, default=25)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--lr", type=float, default=2e-3)
    parser.add_argument("--seed", type=int, default=1337)
    args = parser.parse_args()

    random.seed(args.seed)
    torch.manual_seed(args.seed)
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    tokenizer = Tokenizer.from_file(str(TOKENIZER_PATH))
    train_ds = SlotTaggedDataset(DATASET_DIR / "train_slots.jsonl")
    val_ds = SlotTaggedDataset(DATASET_DIR / "val_slots.jsonl")
    ood_ds = SlotTaggedDataset(DATASET_DIR / "test_ood_slots.jsonl")

    train_loader = DataLoader(train_ds, batch_size=args.batch_size, shuffle=True)

    model = RelicSlotFiller(vocab_size=tokenizer.get_vocab_size()).to(device)
    total_params = sum(p.numel() for p in model.parameters())
    print("=" * 60)
    print("  RELIC — Entrenamiento del Intérprete Semántico Neuronal")
    print("=" * 60)
    print(f"Parámetros del modelo: {total_params:,}")
    print(f"Dataset de entrenamiento: {len(train_ds)} muestras")
    print(f"Dataset de validación:   {len(val_ds)} muestras")
    print(f"Dataset OOD (no visto):   {len(ood_ds)} muestras")
    print("-" * 60)

    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr, weight_decay=1e-4)
    loss_intent_fn = nn.CrossEntropyLoss()
    loss_slot_fn = nn.CrossEntropyLoss(ignore_index=-100)

    best_val_em = 0.0

    for epoch in range(args.epochs):
        model.train()
        total_loss = 0.0
        for input_ids, slot_ids, intent_id, mask in train_loader:
            input_ids, slot_ids = input_ids.to(device), slot_ids.to(device)
            intent_id, mask = intent_id.to(device), mask.to(device)

            optimizer.zero_grad()
            intent_logits, slot_logits = model(input_ids, mask)

            loss_intent = loss_intent_fn(intent_logits, intent_id)
            loss_slots = loss_slot_fn(slot_logits.view(-1, len(SLOT_LABELS)), slot_ids.view(-1))
            loss = loss_intent + 2.0 * loss_slots

            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            total_loss += loss.item()

        if (epoch + 1) % 5 == 0 or epoch == args.epochs - 1:
            val_metrics = evaluate_model(model, val_ds, tokenizer, device)
            print(f"Época {epoch+1:02d}/{args.epochs:02d} | Loss: {total_loss/len(train_loader):.4f} | Val Intent: {val_metrics['intent_acc']}% | Val Slot: {val_metrics['slot_acc']}% | Val RIR EM: {val_metrics['rir_em']}%")

    # Final Evaluation
    print("-" * 60)
    val_res = evaluate_model(model, val_ds, tokenizer, device)
    ood_res = evaluate_model(model, ood_ds, tokenizer, device)
    print(f"[RESULTADO FINAL] Validación ID : Intent = {val_res['intent_acc']}% | Slots = {val_res['slot_acc']}% | RIR EM = {val_res['rir_em']}%")
    print(f"[RESULTADO FINAL] Generalización OOD: Intent = {ood_res['intent_acc']}% | Slots = {ood_res['slot_acc']}% | RIR EM = {ood_res['rir_em']}%")
    print("=" * 60)

    # Save checkpoint & Safetensors
    WEIGHTS_DIR.mkdir(parents=True, exist_ok=True)
    ckpt_pt = WEIGHTS_DIR / "relic_slot_filler.pt"
    torch.save(model.state_dict(), ckpt_pt)

    # Safetensors export
    ckpt_st = WEIGHTS_DIR / "relic_slot_filler.safetensors"
    tensors_dict = {k: v.contiguous() for k, v in model.state_dict().items()}
    save_file(tensors_dict, str(ckpt_st))
    print(f"Pesos guardados en:\n  - {ckpt_pt}\n  - {ckpt_st}")


if __name__ == "__main__":
    train()
