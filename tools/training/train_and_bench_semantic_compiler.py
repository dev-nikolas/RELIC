#!/usr/bin/env python3
"""Fases 3 y 4: Entrenamiento y Benchmark del Compilador Semántico RIR v1 vs Baseline Heurístico.

Métricas Evaluadas:
1. Coincidencia exacta del RIR (exact_match)
2. Exactitud de intención (intent_accuracy)
3. Exactitud de extracción de slots (cantidades, operadores, unidades)
4. Exactitud de ejecución final con ReasoningEngine/Arithmetic
5. Tasa de rechazo en casos incompletos/ambiguos/no soportados
6. Tasa de falsos positivos
7. Latencia por consulta (µs) y Memoria RSS (MB)
8. Generalización a familias OOD no vistas
"""

import argparse
import ctypes
import ctypes.wintypes
import json
import math
import os
import random
import sys
import time
from pathlib import Path
from typing import Any

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import Dataset, DataLoader
from tokenizers import Tokenizer

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))
sys.path.insert(0, str(ROOT_DIR / "tools"))

from common import REPORTS_DIR, TOKENIZER_DIR
from relic.config import RelicConfig
from relic.model import RelicForCausalLM
from relic.rir_v1 import (
    RirDocumentV1,
    IntentKind,
    DomainKind,
    ValidityStatus,
    RirArithmeticOp,
    ArithmeticStep,
)

DATASET_DIR = ROOT_DIR / "data" / "semantic_compiler"


# ==============================================================================
# MONITOR NATIVO DE MEMORIA FÍSICA RSS (WINDOWS KERNEL32 / PSAPI)
# ==============================================================================
def get_native_rss_mb() -> float:
    try:
        class ProcessMemoryCounters(ctypes.Structure):
            _fields_ = [
                ("cb", ctypes.wintypes.DWORD),
                ("PageFaultCount", ctypes.wintypes.DWORD),
                ("PeakWorkingSetSize", ctypes.c_size_t),
                ("WorkingSetSize", ctypes.c_size_t),
                ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
                ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
                ("PagefileUsage", ctypes.c_size_t),
                ("PeakPagefileUsage", ctypes.c_size_t),
            ]

        fn = ctypes.windll.kernel32.K32GetProcessMemoryInfo
        fn.argtypes = [ctypes.c_void_p, ctypes.POINTER(ProcessMemoryCounters), ctypes.wintypes.DWORD]
        fn.restype = ctypes.wintypes.BOOL
        pmc = ProcessMemoryCounters()
        pmc.cb = ctypes.sizeof(ProcessMemoryCounters)
        if fn(ctypes.c_void_p(-1), ctypes.byref(pmc), pmc.cb):
            return round(pmc.WorkingSetSize / (1024 * 1024), 2)
    except Exception:
        pass
    return 0.0


# ==============================================================================
# DATASET CON TARGET-MASKING PARA COMPILACIÓN SEMÁNTICA
# ==============================================================================
class RirCompilerDataset(Dataset):
    def __init__(self, jsonl_path: Path, tokenizer: Tokenizer, max_len: int = 160):
        self.samples = []
        eos_id = tokenizer.token_to_id("<|eos|>")
        if eos_id is None:
            eos_id = 1

        with open(jsonl_path, "r", encoding="utf-8") as f:
            for line in f:
                item = json.loads(line)
                text = item["text"]
                rir_str = json.dumps(item["rir"], separators=(",", ":"), ensure_ascii=False)

                prompt_str = f"Entrada: {text}\nRIR: "
                prompt_ids = tokenizer.encode(prompt_str).ids
                target_ids = tokenizer.encode(rir_str).ids + [eos_id]

                input_ids = prompt_ids + target_ids
                labels = [-100] * len(prompt_ids) + target_ids

                if len(input_ids) > max_len:
                    input_ids = input_ids[:max_len]
                    labels = labels[:max_len]
                else:
                    pad_len = max_len - len(input_ids)
                    input_ids = input_ids + [2] * pad_len
                    labels = labels + [-100] * pad_len

                self.samples.append({
                    "raw": item,
                    "prompt_str": prompt_str,
                    "input_ids": torch.tensor(input_ids, dtype=torch.long),
                    "labels": torch.tensor(labels, dtype=torch.long),
                })

    def __len__(self):
        return len(self.samples)

    def __getitem__(self, idx):
        return self.samples[idx]["input_ids"], self.samples[idx]["labels"]


# ==============================================================================
# BASELINE HEURÍSTICO CONGELADO (MÉTODO REGLAS EXISTENTE EN RUST / PYTHON)
# ==============================================================================
def heuristic_baseline_parse(text: str) -> dict[str, Any]:
    clean = text.strip()
    lower = clean.lower()

    # Incompleto explícito
    if "+ ?" in lower or "- ?" in lower or "* ?" in lower or "/ ?" in lower:
        return RirDocumentV1(
            schema_version=1,
            intent=IntentKind.ARITHMETIC_QUERY,
            domain=DomainKind.ABSTRACT_MATH,
            status=ValidityStatus.INCOMPLETE,
        ).to_dict()

    if "algunas" in lower or "algunos" in lower:
        return RirDocumentV1(
            schema_version=1,
            intent=IntentKind.INVENTORY_UPDATE,
            domain=DomainKind.INVENTORY,
            status=ValidityStatus.INCOMPLETE,
        ).to_dict()

    # Inventario básico ("tengo X Y y recibo Z más")
    if "tengo " in lower or "hay " in lower or "poseo " in lower:
        idx = lower.find("tengo ")
        if idx == -1:
            idx = lower.find("hay ")
        if idx == -1:
            idx = lower.find("poseo ")

        prefix_len = 6 if "tengo " in lower or "poseo " in lower else 4
        after = lower[idx + prefix_len:].strip()
        parts = after.split()
        if parts:
            try:
                init_val = float(parts[0].replace(".", ""))
                unit = parts[1].strip(".,?!") if len(parts) > 1 and parts[1] not in ["y", "pero", "luego"] else None
                ops = []
                # Búsqueda de verbos fijados en el baseline
                if "compro" in lower or "recibo" in lower or "más" in lower:
                    for w in parts:
                        clean_w = "".join(c for c in w if c.isdigit() or c == ".")
                        if clean_w and float(clean_w) != init_val:
                            ops.append(ArithmeticStep(RirArithmeticOp.ADD, float(clean_w)))
                            break
                if "regalo" in lower or "dono" in lower or "vendo" in lower:
                    for w in parts:
                        clean_w = "".join(c for c in w if c.isdigit() or c == ".")
                        if clean_w and float(clean_w) != init_val and not any(op.operand == float(clean_w) for op in ops):
                            ops.append(ArithmeticStep(RirArithmeticOp.SUBTRACT, float(clean_w)))
                            break

                if ops:
                    return RirDocumentV1(
                        schema_version=1,
                        intent=IntentKind.INVENTORY_UPDATE,
                        domain=DomainKind.INVENTORY,
                        status=ValidityStatus.VALID,
                        entity=unit,
                        initial_quantity=init_val,
                        operations=ops,
                    ).to_dict()
            except Exception:
                pass

    # Aritmética abstracta ("¿Cuánto es X + Y?")
    for op_sym, op_enum in [("+", RirArithmeticOp.ADD), ("-", RirArithmeticOp.SUBTRACT)]:
        if op_sym in clean:
            parts = clean.split(op_sym)
            try:
                num1 = float("".join(c for c in parts[0] if c.isdigit() or c == "."))
                num2 = float("".join(c for c in parts[1] if c.isdigit() or c == "."))
                return RirDocumentV1(
                    schema_version=1,
                    intent=IntentKind.ARITHMETIC_QUERY,
                    domain=DomainKind.ABSTRACT_MATH,
                    status=ValidityStatus.VALID,
                    initial_quantity=num1,
                    operations=[ArithmeticStep(op_enum, num2)],
                ).to_dict()
            except Exception:
                pass

    # No soportado por defecto
    return RirDocumentV1(
        schema_version=1,
        intent=IntentKind.UNKNOWN,
        domain=DomainKind.UNSUPPORTED,
        status=ValidityStatus.UNSUPPORTED,
    ).to_dict()


# ==============================================================================
# MOTOR DETERMINISTA DE EJECUCIÓN PARA VALIDAR RIR
# ==============================================================================
def execute_rir(rir_dict: dict[str, Any]) -> tuple[bool, Any]:
    try:
        doc = RirDocumentV1.from_dict(rir_dict)
        doc.validate_semantic()

        if doc.intent in (IntentKind.INVENTORY_UPDATE, IntentKind.ARITHMETIC_QUERY):
            val = doc.initial_quantity
            for op in doc.operations:
                if op.operator == RirArithmeticOp.ADD:
                    val += op.operand
                elif op.operator == RirArithmeticOp.SUBTRACT:
                    val -= op.operand
                elif op.operator == RirArithmeticOp.MULTIPLY:
                    val *= op.operand
                elif op.operator == RirArithmeticOp.DIVIDE:
                    val /= op.operand
            return True, val
        elif doc.intent == IntentKind.GRAPH_QUERY:
            return True, "GRAPH_OK"
        return False, "NON_EXECUTABLE"
    except Exception as e:
        return False, str(e)


# ==============================================================================
# DECODIFICACIÓN Y GENERACIÓN DEL COMPILADOR SEMÁNTICO
# ==============================================================================
def compile_text_to_rir(
    model: nn.Module,
    tokenizer: Tokenizer,
    text: str,
    device: torch.device,
    max_gen_tokens: int = 120,
) -> tuple[dict[str, Any] | None, str, float]:
    model.eval()
    prompt_str = f"Entrada: {text}\nRIR: "
    input_ids = tokenizer.encode(prompt_str).ids
    curr_ids = list(input_ids)

    eos_id = tokenizer.token_to_id("<|eos|>") or 1

    t0 = time.perf_counter()
    with torch.no_grad():
        for _ in range(max_gen_tokens):
            inp = torch.tensor([curr_ids], dtype=torch.long, device=device)
            logits = model(inp)
            next_token = torch.argmax(logits[0, -1], dim=-1).item()
            if next_token == eos_id or next_token == 2:  # EOS o PAD
                break
            curr_ids.append(next_token)

    elapsed_us = (time.perf_counter() - t0) * 1_000_000
    gen_text = tokenizer.decode(curr_ids[len(input_ids):])

    # Parsear JSON de salida
    try:
        # Extraer substring entre primera llave y última llave
        start = gen_text.find("{")
        end = gen_text.rfind("}")
        if start != -1 and end != -1 and end > start:
            json_str = gen_text[start:end + 1]
            data = json.loads(json_str)
            return data, gen_text, elapsed_us
        return None, gen_text, elapsed_us
    except Exception:
        return None, gen_text, elapsed_us


# ==============================================================================
# PIPELINE DE EVALUACIÓN
# ==============================================================================
def evaluate_dataset_partition(
    model: nn.Module | None,
    tokenizer: Tokenizer,
    dataset_path: Path,
    use_heuristic: bool,
    device: torch.device,
) -> dict[str, Any]:
    samples = []
    with open(dataset_path, "r", encoding="utf-8") as f:
        for line in f:
            samples.append(json.loads(line))

    exact_matches = 0
    intent_matches = 0
    slot_matches = 0
    exec_matches = 0
    rejections_correct = 0
    false_positives = 0
    latencies = []
    failures = []

    for s in samples:
        query = s["text"]
        gold_rir = s["rir"]
        gold_ok, gold_exec = execute_rir(gold_rir)

        if use_heuristic:
            t0 = time.perf_counter()
            pred_rir = heuristic_baseline_parse(query)
            lat = (time.perf_counter() - t0) * 1_000_000
            raw_gen = json.dumps(pred_rir)
        else:
            pred_rir, raw_gen, lat = compile_text_to_rir(model, tokenizer, query, device)

        latencies.append(lat)

        # 1. Exact Match
        is_exact = (pred_rir == gold_rir)
        if is_exact:
            exact_matches += 1

        # 2. Intent Match
        pred_intent = pred_rir.get("intent") if pred_rir else None
        gold_intent = gold_rir.get("intent")
        if pred_intent == gold_intent:
            intent_matches += 1

        # 3. Slot Match
        pred_init = pred_rir.get("initial_quantity") if pred_rir else None
        gold_init = gold_rir.get("initial_quantity")
        pred_ops = pred_rir.get("operations") if pred_rir else None
        gold_ops = gold_rir.get("operations")
        if pred_init == gold_init and pred_ops == gold_ops:
            slot_matches += 1

        # 4. Exec Match
        if pred_rir:
            pred_ok, pred_exec = execute_rir(pred_rir)
            if pred_ok == gold_ok and pred_exec == gold_exec:
                exec_matches += 1
            elif pred_ok and not gold_ok:
                false_positives += 1
        else:
            if not gold_ok:
                exec_matches += 1  # Rechazo correcto

        # 5. Rejection Rate
        if gold_rir.get("status") in ["INCOMPLETE", "AMBIGUOUS", "UNSUPPORTED"]:
            if pred_rir and pred_rir.get("status") == gold_rir.get("status"):
                rejections_correct += 1
            elif pred_rir is None:
                rejections_correct += 1

        if not is_exact:
            failures.append({
                "query": query,
                "family": s.get("family"),
                "gold": gold_rir,
                "pred": pred_rir,
                "raw_output": raw_gen,
            })

    total = len(samples)
    return {
        "total_samples": total,
        "exact_match_rate": round(exact_matches / total, 4),
        "intent_accuracy": round(intent_matches / total, 4),
        "slot_accuracy": round(slot_matches / total, 4),
        "execution_accuracy": round(exec_matches / total, 4),
        "rejection_accuracy": round(rejections_correct / max(1, sum(1 for s in samples if s["rir"].get("status") != "VALID")), 4),
        "false_positive_count": false_positives,
        "avg_latency_us": round(sum(latencies) / len(latencies), 1),
        "failures": failures[:5],  # Guardar los 5 primeros fallos ilustrativos
    }


# ==============================================================================
# ENTRENAMIENTO EXPERIMENTAL DEL COMPILADOR SEMÁNTICO
# ==============================================================================
def train_semantic_compiler(
    train_path: Path,
    val_path: Path,
    tokenizer: Tokenizer,
    epochs: int,
    batch_size: int,
    lr: float,
    seed: int,
    device: torch.device,
) -> tuple[RelicForCausalLM, list[float]]:
    random.seed(seed)
    torch.manual_seed(seed)

    train_ds = RirCompilerDataset(train_path, tokenizer)
    val_ds = RirCompilerDataset(val_path, tokenizer)

    train_loader = DataLoader(train_ds, batch_size=batch_size, shuffle=True)
    val_loader = DataLoader(val_ds, batch_size=batch_size, shuffle=False)

    cfg = RelicConfig(
        vocab_size=tokenizer.get_vocab_size(),
        hidden_size=288,
        intermediate_size=768,
        num_hidden_layers=6,
        num_attention_heads=6,
        num_key_value_heads=2,
        head_dim=48,
        max_position_embeddings=512,
        max_train_seq_len=160,
    )
    model = RelicForCausalLM(cfg).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=0.01)

    print(f"\n[ENTRENAMIENTO] Compilador Semántico RIR ({model.count_parameters():,} params, semilla {seed})...")
    model.train()
    loss_curve = []

    for epoch in range(epochs):
        epoch_loss = 0.0
        for x, y in train_loader:
            x, y = x.to(device), y.to(device)
            optimizer.zero_grad()
            logits = model(x)
            loss = F.cross_entropy(logits.view(-1, cfg.vocab_size), y.view(-1), ignore_index=-100)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            epoch_loss += loss.item()

        avg_loss = epoch_loss / len(train_loader)
        loss_curve.append(round(avg_loss, 4))
        if epoch % 5 == 0 or epoch == epochs - 1:
            print(f"  Época {epoch+1:02d}/{epochs:02d} | Train Loss (RIR): {avg_loss:.4f}")

    return model, loss_curve


# ==============================================================================
# EXPERIMENTO Y REPORTE INTEGRAL
# ==============================================================================
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--epochs", type=int, default=15)
    parser.add_argument("--batch-size", type=int, default=8)
    parser.add_argument("--lr", type=float, default=5e-4)
    parser.add_argument("--seeds", nargs="+", type=int, default=[42, 123, 999])
    parser.add_argument("--device", default="cpu")
    args = parser.parse_args()

    device = torch.device(args.device)
    tok_path = TOKENIZER_DIR / "tokenizer.json"
    tokenizer = Tokenizer.from_file(str(tok_path))

    train_path = DATASET_DIR / "train.jsonl"
    val_path = DATASET_DIR / "val.jsonl"
    test_id_path = DATASET_DIR / "test_id.jsonl"
    test_ood_path = DATASET_DIR / "test_ood.jsonl"

    print("======================================================================")
    print("  RELIC — Benchmark Oficial: Baseline Heurístico vs Compilador RIR")
    print("======================================================================")
    print(f"Semillas evaluadas: {args.seeds}")
    print(f"Dispositivo:        {device}")
    print(f"Memoria RSS Inicial: {get_native_rss_mb():.2f} MB")
    print("----------------------------------------------------------------------")

    # 1. EVALUAR BASELINE HEURÍSTICO CONGELADO (Reglas existentes)
    print("\n[1/3] Evaluando Baseline Heurístico Congelado...")
    heur_val = evaluate_dataset_partition(None, tokenizer, val_path, use_heuristic=True, device=device)
    heur_id = evaluate_dataset_partition(None, tokenizer, test_id_path, use_heuristic=True, device=device)
    heur_ood = evaluate_dataset_partition(None, tokenizer, test_ood_path, use_heuristic=True, device=device)

    print(f"  Heurístico - Val Exact Match:    {heur_val['exact_match_rate'] * 100:.2f}% | Exec Acc: {heur_val['execution_accuracy'] * 100:.2f}%")
    print(f"  Heurístico - ID Test Exact:      {heur_id['exact_match_rate'] * 100:.2f}% | Exec Acc: {heur_id['execution_accuracy'] * 100:.2f}%")
    print(f"  Heurístico - OOD Test Exact:     {heur_ood['exact_match_rate'] * 100:.2f}% | Exec Acc: {heur_ood['execution_accuracy'] * 100:.2f}%")
    print(f"  Heurístico - Latencia Promedio:  {heur_val['avg_latency_us']:.1f} µs")

    # 2. ENTRENAR Y EVALUAR COMPILADOR SEMÁNTICO EN 3 SEMILLAS
    learned_results = []
    last_model = None

    for seed in args.seeds:
        model, loss_curve = train_semantic_compiler(
            train_path=train_path,
            val_path=val_path,
            tokenizer=tokenizer,
            epochs=args.epochs,
            batch_size=args.batch_size,
            lr=args.lr,
            seed=seed,
            device=device,
        )
        last_model = model

        print(f"  Evaluando Compilador (Semilla {seed})...")
        c_val = evaluate_dataset_partition(model, tokenizer, val_path, use_heuristic=False, device=device)
        c_id = evaluate_dataset_partition(model, tokenizer, test_id_path, use_heuristic=False, device=device)
        c_ood = evaluate_dataset_partition(model, tokenizer, test_ood_path, use_heuristic=False, device=device)

        print(f"    Val Exact:    {c_val['exact_match_rate'] * 100:.2f}% | Exec: {c_val['execution_accuracy'] * 100:.2f}%")
        print(f"    ID Test:      {c_id['exact_match_rate'] * 100:.2f}% | Exec: {c_id['execution_accuracy'] * 100:.2f}%")
        print(f"    OOD Test:     {c_ood['exact_match_rate'] * 100:.2f}% | Exec: {c_ood['execution_accuracy'] * 100:.2f}%")

        learned_results.append({
            "seed": seed,
            "loss_curve": loss_curve,
            "val": c_val,
            "test_id": c_id,
            "test_ood": c_ood,
        })

    # Resumen y reporte JSON
    rss_peak = get_native_rss_mb()
    report = {
        "metadata": {
            "epochs": args.epochs,
            "seeds": args.seeds,
            "learning_rate": args.lr,
            "device": str(device),
            "peak_rss_mb": rss_peak,
        },
        "heuristic_baseline": {
            "val": heur_val,
            "test_id": heur_id,
            "test_ood": heur_ood,
        },
        "semantic_compiler": {
            "runs": learned_results,
            "averages": {
                "val_exact_match": round(sum(r["val"]["exact_match_rate"] for r in learned_results) / len(args.seeds), 4),
                "val_exec_accuracy": round(sum(r["val"]["execution_accuracy"] for r in learned_results) / len(args.seeds), 4),
                "id_exact_match": round(sum(r["test_id"]["exact_match_rate"] for r in learned_results) / len(args.seeds), 4),
                "id_exec_accuracy": round(sum(r["test_id"]["execution_accuracy"] for r in learned_results) / len(args.seeds), 4),
                "ood_exact_match": round(sum(r["test_ood"]["exact_match_rate"] for r in learned_results) / len(args.seeds), 4),
                "ood_exec_accuracy": round(sum(r["test_ood"]["execution_accuracy"] for r in learned_results) / len(args.seeds), 4),
                "avg_latency_us": round(sum(r["val"]["avg_latency_us"] for r in learned_results) / len(args.seeds), 1),
            },
        },
    }

    REPORTS_DIR.mkdir(parents=True, exist_ok=True)
    report_file = REPORTS_DIR / "semantic_compiler_report.json"
    with open(report_file, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    print(f"\n[ÉXITO] Reporte reproducible guardado en: {report_file}")

    # Tabla resumen
    b_val = heur_val["exact_match_rate"] * 100
    b_ood = heur_ood["exact_match_rate"] * 100
    c_val = report["semantic_compiler"]["averages"]["val_exact_match"] * 100
    c_ood = report["semantic_compiler"]["averages"]["ood_exact_match"] * 100

    print("\n" + "=" * 76)
    print(f"{'MÉTRICA':<32} | {'BASELINE HEURÍSTICO':<18} | {'COMPILADOR SEMÁNTICO':<18}")
    print("-" * 76)
    print(f"{'Exact Match RIR (Val)':<32} | {b_val:<18.2f}% | {c_val:<18.2f}%")
    print(f"{'Exact Match RIR (OOD)':<32} | {b_ood:<18.2f}% | {c_ood:<18.2f}%")
    print(f"{'Exec Acc (OOD No Visto)':<32} | {heur_ood['execution_accuracy'] * 100:<18.2f}% | {report['semantic_compiler']['averages']['ood_exec_accuracy'] * 100:<18.2f}%")
    print(f"{'Latencia por Consulta':<32} | {heur_val['avg_latency_us']:<18.1f} µs | {report['semantic_compiler']['averages']['avg_latency_us']:<18.1f} µs")
    print(f"{'Pico Memoria RSS':<32} | {'~44 MB (Rust)':<18} | {rss_peak:<18.2f} MB")
    print("=" * 76)


if __name__ == "__main__":
    main()
