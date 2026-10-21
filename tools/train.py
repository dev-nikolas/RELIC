#!/usr/bin/env python3
"""Fase 2: Motor de entrenamiento para RELIC-10M con AdamW, Cosine Decay y Warmup.
Soporta dry-run de 100 pasos y entrenamiento completo. Menos de 180 lineas.
"""
import argparse
import json
import math
import sys
import time
from pathlib import Path
import torch
import torch.nn.functional as F
from torch.utils.data import DataLoader
from tokenizers import Tokenizer

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))
sys.path.insert(0, str(ROOT_DIR / "tools"))

from common import DATA_DIR, REPORTS_DIR, TOKENIZER_DIR
from dataset import StoryDataset
from relic import RelicConfig, RelicForCausalLM


def get_lr(step: int, warmup_steps: int, max_steps: int, max_lr: float, min_lr: float) -> float:
    """Calcula la tasa de aprendizaje con warmup lineal y decaimiento coseno."""
    if step < warmup_steps:
        return max_lr * (step + 1) / warmup_steps
    if step > max_steps:
        return min_lr
    decay_ratio = (step - warmup_steps) / (max_steps - warmup_steps)
    coeff = 0.5 * (1.0 + math.cos(math.pi * decay_ratio))
    return min_lr + coeff * (max_lr - min_lr)


def train() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--batch-size", type=int, default=16)
    ap.add_argument("--seq-len", type=int, default=256)
    ap.add_argument("--max-steps", type=int, default=100)
    ap.add_argument("--warmup-steps", type=int, default=20)
    ap.add_argument("--max-lr", type=float, default=5e-4)
    ap.add_argument("--min-lr", type=float, default=5e-5)
    ap.add_argument("--device", default="cpu")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    device = torch.device(args.device)
    tok_path = TOKENIZER_DIR / "tokenizer.json"
    if not tok_path.exists():
        sys.exit(f"Falta tokenizer en {tok_path}. Ejecuta primero tools/train_bpe.py")

    tokenizer = Tokenizer.from_file(str(tok_path))
    dataset = StoryDataset(DATA_DIR / "train.txt", tokenizer, seq_len=args.seq_len)
    loader = DataLoader(dataset, batch_size=args.batch_size, shuffle=True)

    config = RelicConfig(max_position_embeddings=512, max_train_seq_len=args.seq_len)
    model = RelicForCausalLM(config).to(device)
    model.train()

    optimizer = torch.optim.AdamW(model.parameters(), lr=args.min_lr, betas=(0.9, 0.95), weight_decay=0.1)

    print(f"Iniciando entrenamiento RELIC-10M ({model.count_parameters():,} params) en {device}...")
    step = 0
    start_time = time.time()
    metrics_history = []
    data_iter = iter(loader)

    while step < args.max_steps:
        try:
            x, y = next(data_iter)
        except StopIteration:
            data_iter = iter(loader)
            x, y = next(data_iter)

        x, y = x.to(device), y.to(device)
        lr = get_lr(step, args.warmup_steps, args.max_steps, args.max_lr, args.min_lr)
        for param_group in optimizer.param_groups:
            param_group["lr"] = lr

        step_start = time.time()
        optimizer.zero_grad()
        logits = model(x)
        loss = F.cross_entropy(logits.view(-1, config.vocab_size), y.view(-1))
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)
        optimizer.step()
        step_elapsed = time.time() - step_start

        tokens_in_step = x.numel()
        tok_sec = tokens_in_step / max(1e-6, step_elapsed)
        loss_val = round(loss.item(), 4)

        if step % 10 == 0 or step == args.max_steps - 1:
            print(f"Paso {step:03d}/{args.max_steps} | Loss: {loss_val:.4f} | LR: {lr:.2e} | {tok_sec:.0f} tok/s")

        metrics_history.append({"step": step, "loss": loss_val, "tok_sec": round(tok_sec, 1), "lr": lr})
        step += 1

    total_time = time.time() - start_time
    print(f"Finalizado en {total_time:.1f}s ({total_time / args.max_steps:.3f} s/paso)")

    REPORTS_DIR.mkdir(parents=True, exist_ok=True)
    report_file = REPORTS_DIR / ("dry_run_metrics.json" if args.dry_run else "training_metrics.json")
    with open(report_file, "w", encoding="utf-8") as f:
        json.dump({
            "total_steps": args.max_steps,
            "total_time_seconds": round(total_time, 2),
            "final_loss": loss_val,
            "initial_loss": metrics_history[0]["loss"],
            "avg_tok_sec": round(sum(m["tok_sec"] for m in metrics_history) / len(metrics_history), 1),
            "history": metrics_history,
        }, f, indent=2)

    # Guardar checkpoint inicial
    ckpt_dir = ROOT_DIR / "artifacts" / "checkpoints"
    ckpt_dir.mkdir(parents=True, exist_ok=True)
    torch.save(model.state_dict(), ckpt_dir / "relic_10m_latest.pt")
    print(f"Reporte guardado en {report_file}")
    print(f"Checkpoint guardado en {ckpt_dir / 'relic_10m_latest.pt'}")


if __name__ == "__main__":
    train()
