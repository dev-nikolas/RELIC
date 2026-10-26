"""Funciones auxiliares de evaluación, generación periódica y métricas para train.py.
Menos de 180 líneas.
"""

import csv
import math
from pathlib import Path
import torch
import torch.nn.functional as F


@torch.no_grad()
def evaluate_model(model, val_loader, device, max_steps: int, vocab_size: int) -> tuple[float, float]:
    """Calcula pérdida media y perplejidad sobre el conjunto de validación."""
    model.eval()
    total_loss = 0.0
    steps = 0
    for i, (x, y) in enumerate(val_loader):
        if i >= max_steps:
            break
        x, y = x.to(device), y.to(device)
        logits = model(x)
        loss = F.cross_entropy(logits.view(-1, vocab_size), y.view(-1))
        total_loss += loss.item()
        steps += 1

    model.train()
    if steps == 0:
        return 0.0, 0.0
    avg_loss = total_loss / steps
    perplexity = math.exp(min(avg_loss, 20.0))
    return round(avg_loss, 4), round(perplexity, 2)


@torch.no_grad()
def generate_sample(model, tokenizer, prompt: str, device, max_tokens: int = 32) -> str:
    """Genera texto causal con muestreo greedy para monitoreo en vivo."""
    model.eval()
    enc = tokenizer.encode(prompt)
    input_ids = torch.tensor([enc.ids], dtype=torch.long, device=device)

    for _ in range(max_tokens):
        logits = model(input_ids)
        next_token = torch.argmax(logits[:, -1, :], dim=-1, keepdim=True)
        input_ids = torch.cat([input_ids, next_token], dim=1)
        if next_token.item() in (tokenizer.token_to_id("<|eos|>"), 315):
            break

    gen_ids = input_ids[0].tolist()
    decoded = tokenizer.decode(gen_ids)
    model.train()
    return decoded


def append_metrics_csv(csv_path: Path, metrics: dict) -> None:
    """Registra una fila de métricas en formato CSV."""
    csv_path.parent.mkdir(parents=True, exist_ok=True)
    file_exists = csv_path.exists()
    with open(csv_path, "a", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=list(metrics.keys()))
        if not file_exists:
            writer.writeheader()
        writer.writerow(metrics)
