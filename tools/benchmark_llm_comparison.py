#!/usr/bin/env python3
"""Benchmark Científico y Reproducible: RELIC-10M (Transformer) vs RELIC-H1-LM (Recurrente O(1)).

Protocolo experimental pareado:
1. Mismo tokenizador BPE (artifacts/tokenizer/tokenizer.json, vocab=4,262).
2. Mismo corpus (data/train.txt) y división de validación (data/val.txt).
3. Mismo presupuesto estricto de tokens de entrenamiento.
4. Semillas aleatorias (42, 123, 999).
5. Evaluación de:
   - Pérdida y Perplejidad en tokens de validación no vistos.
   - Retención a larga distancia (Distant Retrieval / Needle at lengths 32, 64, 128, 256).
   - Coherencia lingüística en español y tasa de diversidad de n-gramas.
   - Huella física de memoria (RSS Windows vía K32GetProcessMemoryInfo) y tamaño de estado.
   - Latencia de generación y tokens por segundo.
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
from torch.utils.data import DataLoader
from tokenizers import Tokenizer

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))
sys.path.insert(0, str(ROOT_DIR / "tools"))

from common import DATA_DIR, REPORTS_DIR, TOKENIZER_DIR
from dataset import StoryDataset
from relic.config import RelicConfig
from relic.core_h1 import RelicCoreH1, RelicCoreH1Config
from relic.model import RelicForCausalLM


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
# FIJACIÓN ESTRICTA DE SEMILLAS REPRODUCIBLES
# ==============================================================================
def set_seed(seed: int) -> None:
    random.seed(seed)
    os.environ["PYTHONHASHSEED"] = str(seed)
    torch.manual_seed(seed)
    if torch.cuda.is_available():
        torch.cuda.manual_seed_all(seed)


# ==============================================================================
# MODEL BUILDERS
# ==============================================================================
def build_transformer_10m(vocab_size: int, seq_len: int) -> RelicForCausalLM:
    cfg = RelicConfig(
        vocab_size=vocab_size,
        hidden_size=288,
        intermediate_size=768,
        num_hidden_layers=6,
        num_attention_heads=6,
        num_key_value_heads=2,
        head_dim=48,
        max_position_embeddings=512,
        max_train_seq_len=seq_len,
    )
    return RelicForCausalLM(cfg)


def build_h1_lm(vocab_size: int, d_model: int = 64, n_layers: int = 2, d_ff: int = 160) -> RelicCoreH1:
    cfg = RelicCoreH1Config(
        vocab_size=vocab_size,
        d_model=d_model,
        n_layers=n_layers,
        d_ff=d_ff,
    )
    return RelicCoreH1(cfg)


# ==============================================================================
# EVALUACIÓN DE PÉRDIDA Y PERPLEJIDAD EN VALIDACIÓN NO VISTA
# ==============================================================================
def evaluate_ppl(
    model: nn.Module,
    val_loader: DataLoader,
    vocab_size: int,
    device: torch.device,
    max_batches: int = 20,
) -> tuple[float, float]:
    model.eval()
    total_loss = 0.0
    total_tokens = 0

    with torch.no_grad():
        for i, (x, y) in enumerate(val_loader):
            if i >= max_batches:
                break
            x, y = x.to(device), y.to(device)
            logits = model(x)
            loss = F.cross_entropy(logits.view(-1, vocab_size), y.view(-1), reduction="sum")
            total_loss += loss.item()
            total_tokens += y.numel()

    avg_loss = total_loss / max(1, total_tokens)
    ppl = math.exp(min(avg_loss, 20.0))  # Evitar overflow
    return round(avg_loss, 4), round(ppl, 2)


# ==============================================================================
# PRUEBA DE RETENCIÓN DE CONTEXTO DISTANTE (NEEDLE IN A HAYSTACK)
# ==============================================================================
def evaluate_distant_retention(
    model: nn.Module,
    tokenizer: Tokenizer,
    distances: list[int],
    device: torch.device,
) -> dict[str, float]:
    """Evalúa si el modelo recuerda un token clave colocado al inicio tras un relleno largo."""
    model.eval()
    results = {}

    target_word = "Bogotá"
    target_id = tokenizer.encode(target_word).ids[0]
    filler_tokens = tokenizer.encode("el sistema procesa la información y almacena hechos continuamente en el grafo ").ids

    for dist in distances:
        # Repetir relleno hasta alcanzar dist tokens
        repeated_filler = (filler_tokens * (dist // len(filler_tokens) + 1))[:dist]
        prefix_ids = [target_id] + repeated_filler
        input_ids = torch.tensor([prefix_ids], dtype=torch.long, device=device)

        with torch.no_grad():
            logits = model(input_ids)
            last_logits = logits[0, -1]
            probs = F.softmax(last_logits, dim=-1)
            target_prob = probs[target_id].item()
            target_rank = (last_logits > last_logits[target_id]).sum().item() + 1

        results[f"dist_{dist}_prob"] = round(target_prob, 6)
        results[f"dist_{dist}_rank"] = target_rank

    return results


# ==============================================================================
# GENERACIÓN DE TEXTO Y CALIDAD LINGÜÍSTICA
# ==============================================================================
def generate_text_samples(
    model: nn.Module,
    tokenizer: Tokenizer,
    prompts: list[str],
    device: torch.device,
    max_new_tokens: int = 16,
) -> list[dict[str, Any]]:
    model.eval()
    samples = []

    for prompt in prompts:
        input_ids = tokenizer.encode(prompt).ids
        curr_ids = list(input_ids)

        # Generación autoregresiva
        with torch.no_grad():
            for _ in range(max_new_tokens):
                inp = torch.tensor([curr_ids], dtype=torch.long, device=device)
                logits = model(inp)
                next_token = torch.argmax(logits[0, -1], dim=-1).item()
                curr_ids.append(next_token)

        gen_tokens = curr_ids[len(input_ids):]
        gen_text = tokenizer.decode(curr_ids)

        # Diversidad léxica (2-gram repetition)
        pairs = list(zip(gen_tokens[:-1], gen_tokens[1:]))
        unique_pairs = len(set(pairs))
        diversity = round(unique_pairs / max(1, len(pairs)), 2)

        samples.append({
            "prompt": prompt,
            "continuation": tokenizer.decode(gen_tokens),
            "full_text": gen_text,
            "bigram_diversity": diversity,
        })

    return samples


# ==============================================================================
# BENCHMARK DE LATENCIA Y VELOCIDAD DE GENERACIÓN (TOK/S)
# ==============================================================================
def benchmark_generation_speed(
    model: nn.Module,
    tokenizer: Tokenizer,
    prompt: str,
    device: torch.device,
    tokens_to_gen: int = 32,
) -> tuple[float, float]:
    model.eval()
    input_ids = tokenizer.encode(prompt).ids
    curr_ids = list(input_ids)

    # Warmup
    with torch.no_grad():
        inp = torch.tensor([curr_ids], dtype=torch.long, device=device)
        _ = model(inp)

    t0 = time.perf_counter()
    with torch.no_grad():
        for _ in range(tokens_to_gen):
            inp = torch.tensor([curr_ids], dtype=torch.long, device=device)
            logits = model(inp)
            next_token = torch.argmax(logits[0, -1], dim=-1).item()
            curr_ids.append(next_token)
    elapsed = time.perf_counter() - t0

    us_per_tok = (elapsed / tokens_to_gen) * 1_000_000
    tok_per_sec = tokens_to_gen / max(1e-6, elapsed)
    return round(us_per_tok, 1), round(tok_per_sec, 1)


# ==============================================================================
# PIPELINE DE ENTRENAMIENTO CONTROLADO POR SEMILLA
# ==============================================================================
def train_single_run(
    model_type: str,
    seed: int,
    train_loader: DataLoader,
    val_loader: DataLoader,
    vocab_size: int,
    seq_len: int,
    max_steps: int,
    lr: float,
    device: torch.device,
) -> dict[str, Any]:
    set_seed(seed)
    rss_start = get_native_rss_mb()

    if model_type == "relic_10m":
        model = build_transformer_10m(vocab_size, seq_len).to(device)
        name = "RELIC-10M (Transformer)"
    elif model_type == "relic_h1":
        model = build_h1_lm(vocab_size, d_model=64, n_layers=2, d_ff=160).to(device)
        name = "RELIC-H1-LM (Recurrente O(1))"
    else:
        raise ValueError(f"Modelo desconocido: {model_type}")

    total_params = sum(p.numel() for p in model.parameters() if p.requires_grad)
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, betas=(0.9, 0.95), weight_decay=0.01)

    model.train()
    data_iter = iter(train_loader)
    loss_history = []
    t_start = time.time()
    total_tokens_trained = 0

    for step in range(max_steps):
        try:
            x, y = next(data_iter)
        except StopIteration:
            data_iter = iter(train_loader)
            x, y = next(data_iter)

        x, y = x.to(device), y.to(device)
        optimizer.zero_grad()
        logits = model(x)
        loss = F.cross_entropy(logits.view(-1, vocab_size), y.view(-1))
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)
        optimizer.step()

        loss_history.append(round(loss.item(), 4))
        total_tokens_trained += x.numel()

    train_time = round(time.time() - t_start, 2)
    val_loss, val_ppl = evaluate_ppl(model, val_loader, vocab_size, device)
    rss_peak = get_native_rss_mb()

    return {
        "model_type": model_type,
        "name": name,
        "seed": seed,
        "parameters": total_params,
        "train_time_sec": train_time,
        "total_tokens_trained": total_tokens_trained,
        "initial_loss": loss_history[0],
        "final_train_loss": loss_history[-1],
        "val_loss": val_loss,
        "val_ppl": val_ppl,
        "rss_mb": rss_peak,
        "model_instance": model,
        "loss_curve": loss_history,
    }


# ==============================================================================
# EJECUCIÓN PRINCIPAL DEL EXPERIMENTO COMPARATIVO
# ==============================================================================
def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--steps", type=int, default=80, help="Presupuesto estricto de pasos por modelo")
    parser.add_argument("--batch-size", type=int, default=8, help="Tamaño de batch")
    parser.add_argument("--seq-len", type=int, default=128, help="Longitud de secuencia")
    parser.add_argument("--lr", type=float, default=3e-4, help="Learning rate común")
    parser.add_argument("--seeds", nargs="+", type=int, default=[42, 123, 999], help="Semillas a evaluar")
    parser.add_argument("--device", default="cpu")
    args = parser.parse_args()

    device = torch.device(args.device)
    tok_path = TOKENIZER_DIR / "tokenizer.json"
    if not tok_path.exists():
        sys.exit(f"Falta tokenizer en {tok_path}")

    tokenizer = Tokenizer.from_file(str(tok_path))
    vocab_size = tokenizer.get_vocab_size()

    print("======================================================================")
    print("  RELIC — Benchmark Oficial: RELIC-10M vs RELIC-H1-LM")
    print("======================================================================")
    print(f"Vocabulario BPE:         {vocab_size} tokens")
    print(f"Semillas evaluadas:      {args.seeds}")
    print(f"Presupuesto por modelo:  {args.steps} pasos × {args.batch_size} × {args.seq_len} = {args.steps * args.batch_size * args.seq_len:,} tokens")
    print(f"Dispositivo:             {device}")
    print("----------------------------------------------------------------------")

    train_dataset = StoryDataset(DATA_DIR / "train.txt", tokenizer, seq_len=args.seq_len)
    val_dataset = StoryDataset(DATA_DIR / "val.txt", tokenizer, seq_len=args.seq_len)

    train_loader = DataLoader(train_dataset, batch_size=args.batch_size, shuffle=True)
    val_loader = DataLoader(val_dataset, batch_size=args.batch_size, shuffle=False)

    prompts = [
        "El pingüino encontró una señal en",
        "La memoria del sistema almacena hechos como",
        "El resultado de la operación es",
    ]
    distances = [32, 64, 128, 256]

    results: dict[str, Any] = {
        "metadata": {
            "vocab_size": vocab_size,
            "steps_per_run": args.steps,
            "batch_size": args.batch_size,
            "seq_len": args.seq_len,
            "tokens_per_run": args.steps * args.batch_size * args.seq_len,
            "seeds": args.seeds,
            "device": str(device),
        },
        "runs": [],
        "summary": {},
    }

    models_to_test = ["relic_10m", "relic_h1"]
    last_models: dict[str, nn.Module] = {}

    for m_type in models_to_test:
        for seed in args.seeds:
            print(f"\n--> Entrenando {m_type.upper()} [Semilla {seed}]...")
            run_data = train_single_run(
                model_type=m_type,
                seed=seed,
                train_loader=train_loader,
                val_loader=val_loader,
                vocab_size=vocab_size,
                seq_len=args.seq_len,
                max_steps=args.steps,
                lr=args.lr,
                device=device,
            )
            model_inst = run_data.pop("model_instance")
            last_models[m_type] = model_inst

            print(f"    Train Loss: {run_data['initial_loss']:.4f} -> {run_data['final_train_loss']:.4f}")
            print(f"    Val Loss: {run_data['val_loss']:.4f} | Val PPL: {run_data['val_ppl']:.2f}")
            print(f"    RSS: {run_data['rss_mb']} MB | Tiempo: {run_data['train_time_sec']}s")

            results["runs"].append(run_data)

    # Evaluación cualitativa, velocidad y retención de contexto en el último modelo de cada tipo
    print("\n----------------------------------------------------------------------")
    print("  Evaluación Comparativa de Inferencia y Calidad Lingüística")
    print("----------------------------------------------------------------------")

    for m_type in models_to_test:
        m_inst = last_models[m_type]
        us_per_tok, tok_s = benchmark_generation_speed(m_inst, tokenizer, prompts[0], device, tokens_to_gen=24)
        generations = generate_text_samples(m_inst, tokenizer, prompts, device, max_new_tokens=16)
        retention = evaluate_distant_retention(m_inst, tokenizer, distances, device)

        # Cálculo de estado oculto de inferencia
        if m_type == "relic_10m":
            # Transformer KV Cache: 2 (k,v) * n_layers (6) * seq_len (128) * head_dim (48) * n_kv_heads (2) * 4 bytes
            state_bytes = 2 * 6 * 128 * 48 * 2 * 4
        else:
            # H1 Recurrent state: n_layers (2) * d_model (64) * 4 bytes
            state_bytes = 2 * 64 * 4

        results["summary"][m_type] = {
            "avg_val_loss": round(sum(r["val_loss"] for r in results["runs"] if r["model_type"] == m_type) / len(args.seeds), 4),
            "avg_val_ppl": round(sum(r["val_ppl"] for r in results["runs"] if r["model_type"] == m_type) / len(args.seeds), 2),
            "avg_train_time": round(sum(r["train_time_sec"] for r in results["runs"] if r["model_type"] == m_type) / len(args.seeds), 2),
            "parameters": [r["parameters"] for r in results["runs"] if r["model_type"] == m_type][0],
            "generation_us_per_token": us_per_tok,
            "generation_tok_per_sec": tok_s,
            "rss_mb": [r["rss_mb"] for r in results["runs"] if r["model_type"] == m_type][-1],
            "inference_state_bytes": state_bytes,
            "avg_bigram_diversity": round(sum(g["bigram_diversity"] for g in generations) / len(generations), 2),
            "distant_retention": retention,
            "generations": generations,
        }

    # Guardar reporte JSON
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)
    report_file = REPORTS_DIR / "llm_comparison_report.json"
    with open(report_file, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)

    print(f"\n[ÉXITO] Reporte científico completo guardado en: {report_file}")

    # Impresión resumida en consola
    s_tf = results["summary"]["relic_10m"]
    s_h1 = results["summary"]["relic_h1"]

    print("\n" + "=" * 76)
    print(f"{'MÉTRICA':<32} | {'RELIC-10M (TRANSFORMER)':<20} | {'RELIC-H1-LM (O(1))':<18}")
    print("-" * 76)
    print(f"{'Parámetros Totales':<32} | {s_tf['parameters']:<20,} | {s_h1['parameters']:<18,}")
    print(f"{'Pérdida Validación (Media)':<32} | {s_tf['avg_val_loss']:<20.4f} | {s_h1['avg_val_ppl']:<18.4f}")
    print(f"{'Perplejidad PPL (Media)':<32} | {s_tf['avg_val_ppl']:<20.2f} | {s_h1['avg_val_ppl']:<18.2f}")
    print(f"{'Memoria Estado Inferencia':<32} | {s_tf['inference_state_bytes'] / 1024:<17.2f} KB | {s_h1['inference_state_bytes']:<14} bytes")
    print(f"{'Pico Memoria RSS':<32} | {s_tf['rss_mb']:<17.2f} MB | {s_h1['rss_mb']:<15.2f} MB")
    print(f"{'Throughput Generación CPU':<32} | {s_tf['generation_tok_per_sec']:<17.1f} t/s | {s_h1['generation_tok_per_sec']:<15.1f} t/s")
    print(f"{'Latencia por Token':<32} | {s_tf['generation_us_per_token']:<17.1f} µs | {s_h1['generation_us_per_token']:<15.1f} µs")
    print(f"{'Diversidad Léxica (2-gram)':<32} | {s_tf['avg_bigram_diversity']:<20.2f} | {s_h1['avg_bigram_diversity']:<18.2f}")
    print(f"{'Retención L=256 (Rank / Prob)':<32} | #{s_tf['distant_retention']['dist_256_rank']} ({s_tf['distant_retention']['dist_256_prob']:.4f})  | #{s_h1['distant_retention']['dist_256_rank']} ({s_h1['distant_retention']['dist_256_prob']:.4f})")
    print("=" * 76)


if __name__ == "__main__":
    main()
