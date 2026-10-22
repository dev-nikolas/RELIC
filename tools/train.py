#!/usr/bin/env python3
"""Motor de entrenamiento para RELIC con AdamW, Cosine Decay, AMP y validación periódica.
Menos de 180 líneas.
"""

import argparse
import json
import math
import sys
import time
from pathlib import Path
import numpy as np
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
from train_helpers import append_metrics_csv, evaluate_model, generate_sample


def get_lr(step: int, warmup_steps: int, max_steps: int, max_lr: float, min_lr: float) -> float:
    if step < warmup_steps:
        return max_lr * (step + 1) / max(1, warmup_steps)
    if step > max_steps:
        return min_lr
    ratio = (step - warmup_steps) / max(1, max_steps - warmup_steps)
    return min_lr + 0.5 * (1.0 + math.cos(math.pi * ratio)) * (max_lr - min_lr)


def train() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--config", default=str(ROOT_DIR / "configs" / "train_tinystories_10m.json"))
    ap.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    ap.add_argument("--resume", default=None, help="Ruta a checkpoint .pt para reanudar")
    ap.add_argument("--dry-run", action="store_true", help="Ejecutar sólo 10 pasos para validación")
    args = ap.parse_args()

    with open(args.config, "r", encoding="utf-8") as f:
        cfg_dict = json.load(f)

    m_cfg = cfg_dict["model"]
    t_cfg = cfg_dict["training"]
    seed = t_cfg.get("seed", 1337)
    torch.manual_seed(seed)
    np.random.seed(seed)

    device = torch.device(args.device)
    tok_path = TOKENIZER_DIR / "tokenizer.json"
    if not tok_path.exists():
        sys.exit(f"Falta tokenizer en {tok_path}. Ejecuta tools/train_bpe.py")
    tokenizer = Tokenizer.from_file(str(tok_path))

    seq_len = t_cfg.get("seq_len", 256)
    train_dataset = StoryDataset(DATA_DIR / "train.txt", tokenizer, seq_len=seq_len)
    val_dataset = StoryDataset(DATA_DIR / "val.txt", tokenizer, seq_len=seq_len)

    train_loader = DataLoader(train_dataset, batch_size=t_cfg.get("batch_size", 16), shuffle=True)
    val_loader = DataLoader(val_dataset, batch_size=t_cfg.get("batch_size", 16), shuffle=False)

    config = RelicConfig(**m_cfg)
    model = RelicForCausalLM(config).to(device)

    start_step = 0
    if args.resume and Path(args.resume).exists():
        print(f"Reanudando desde {args.resume}...")
        ckpt = torch.load(args.resume, map_location=device)
        model.load_state_dict(ckpt.get("model", ckpt))
        start_step = ckpt.get("step", 0) if isinstance(ckpt, dict) else 0

    optimizer = torch.optim.AdamW(model.parameters(), lr=t_cfg["min_lr"], betas=(0.9, 0.95), weight_decay=t_cfg.get("weight_decay", 0.1))
    use_amp = (device.type == "cuda") and t_cfg.get("mixed_precision", True)
    scaler = torch.amp.GradScaler("cuda", enabled=use_amp)

    max_steps = 10 if args.dry_run else t_cfg.get("max_steps", 5000)
    accum_steps = t_cfg.get("gradient_accumulation_steps", 1)
    eval_interval = 5 if args.dry_run else t_cfg.get("eval_interval", 250)

    print(f"Entrenamiento RELIC ({model.count_parameters():,} params) | Device: {device} | Pasos: {max_steps} | AMP: {use_amp}")
    csv_log = REPORTS_DIR / "training_metrics.csv"
    train_iter = iter(train_loader)

    for step in range(start_step, max_steps):
        t0 = time.time()
        optimizer.zero_grad(set_to_none=True)
        accum_loss = 0.0

        for _ in range(accum_steps):
            try:
                x, y = next(train_iter)
            except StopIteration:
                train_iter = iter(train_loader)
                x, y = next(train_iter)
            x, y = x.to(device), y.to(device)

            with torch.amp.autocast(device_type=device.type, enabled=use_amp):
                logits = model(x)
                loss = F.cross_entropy(logits.view(-1, config.vocab_size), y.view(-1)) / accum_steps

            scaler.scale(loss).backward()
            accum_loss += loss.item() * accum_steps

        lr = get_lr(step, t_cfg["warmup_steps"], max_steps, t_cfg["max_lr"], t_cfg["min_lr"])
        for p in optimizer.param_groups:
            p["lr"] = lr

        scaler.unscale_(optimizer)
        torch.nn.utils.clip_grad_norm_(model.parameters(), t_cfg.get("grad_clip", 1.0))
        scaler.step(optimizer)
        scaler.update()

        dt = time.time() - t0
        tok_sec = (t_cfg.get("batch_size", 16) * seq_len * accum_steps) / max(1e-6, dt)

        if step % 5 == 0 or step == max_steps - 1:
            print(f"Paso {step:04d}/{max_steps} | Loss: {accum_loss:.4f} | LR: {lr:.2e} | {tok_sec:.0f} tok/s")

        if (step > 0 and step % eval_interval == 0) or step == max_steps - 1:
            val_loss, ppl = evaluate_model(model, val_loader, device, t_cfg.get("eval_steps", 10), config.vocab_size)
            sample = generate_sample(model, tokenizer, "Once upon a time", device)
            print(f"  [EVAL] Val Loss: {val_loss:.4f} | Perplejidad: {ppl:.2f}\n  [MUESTRA]: \"{sample[:80]}...\"")
            append_metrics_csv(csv_log, {"step": step, "train_loss": round(accum_loss, 4), "val_loss": val_loss, "perplexity": ppl, "lr": lr})

    # Guardar checkpoint y exportar
    ckpt_dir = ROOT_DIR / "artifacts" / "checkpoints"
    ckpt_dir.mkdir(parents=True, exist_ok=True)
    out_pt = ckpt_dir / "relic_10m_latest.pt"
    torch.save({"model": model.state_dict(), "config": m_cfg, "step": max_steps}, out_pt)
    print(f"Checkpoint final guardado en {out_pt}")


if __name__ == "__main__":
    train()
