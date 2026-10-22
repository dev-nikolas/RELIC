#!/usr/bin/env python3
"""Experimento OPCIONAL y aislado: Afinado con pares Pregunta-Respuesta en español.
Compara la precisión antes y después del afinado sin contaminar el entrenamiento base.
Menos de 180 líneas.
"""

import json
import sys
from pathlib import Path
import torch
import torch.nn.functional as F
from tokenizers import Tokenizer

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))
sys.path.insert(0, str(ROOT_DIR / "tools"))

from common import REPORTS_DIR, TOKENIZER_DIR
from relic import RelicConfig, RelicForCausalLM

SPANISH_QA_SAMPLES = [
    ("Pregunta: ¿Cuánto es 5 + 3?\nRespuesta: El resultado es 8.", "8"),
    ("Pregunta: ¿Cuánto es 10 - 4?\nRespuesta: El resultado es 6.", "6"),
    ("Pregunta: Tengo 3 manzanas y recibo 2, ¿cuántas tengo?\nRespuesta: Ahora tienes 5 manzanas.", "5"),
    ("Pregunta: Tengo 10 lápices y regalo 4, ¿cuántos tengo?\nRespuesta: Ahora tienes 6 lápices.", "6"),
    ("Pregunta: ¿Cuánto es 7 * 6?\nRespuesta: El resultado es 42.", "42"),
    ("Pregunta: ¿Cuánto es 12 / 3?\nRespuesta: El resultado es 4.", "4"),
    ("Pregunta: ¿Cuánto es 15 + 25?\nRespuesta: El resultado es 40.", "40"),
    ("Pregunta: Tengo 20 panes y vendo 8, ¿cuántos quedan?\nRespuesta: Ahora tienes 12 panes.", "12"),
]


def evaluate_qa_accuracy(model, tokenizer, device) -> float:
    """Evalúa cuántas respuestas contienen la respuesta esperada."""
    model.eval()
    correct = 0
    for text, target in SPANISH_QA_SAMPLES:
        prompt = text.split("Respuesta:")[0] + "Respuesta:"
        enc = tokenizer.encode(prompt)
        input_ids = torch.tensor([enc.ids], dtype=torch.long, device=device)

        with torch.no_grad():
            for _ in range(16):
                logits = model(input_ids)
                next_tok = torch.argmax(logits[:, -1, :], dim=-1, keepdim=True)
                input_ids = torch.cat([input_ids, next_tok], dim=1)
                if next_tok.item() in (tokenizer.token_to_id("<|eos|>"), 315):
                    break

        gen_text = tokenizer.decode(input_ids[0].tolist())
        if target in gen_text:
            correct += 1
    return correct / len(SPANISH_QA_SAMPLES)


def main() -> None:
    print("=" * 60)
    print("  RELIC — Experimento Aislado QA en Español")
    print("=" * 60)

    tok_path = TOKENIZER_DIR / "tokenizer.json"
    if not tok_path.exists():
        sys.exit(f"Falta tokenizer en {tok_path}")
    tokenizer = Tokenizer.from_file(str(tok_path))

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    config = RelicConfig()
    model = RelicForCausalLM(config).to(device)

    # Cargar checkpoint base si existe
    base_ckpt = ROOT_DIR / "artifacts" / "checkpoints" / "relic_10m_latest.pt"
    if base_ckpt.exists():
        ckpt = torch.load(base_ckpt, map_location=device)
        model.load_state_dict(ckpt.get("model", ckpt))
        print(f"[INFO] Checkpoint base cargado desde {base_ckpt}")

    acc_before = evaluate_qa_accuracy(model, tokenizer, device)
    print(f"Precisión antes del afinado: {acc_before * 100:.1f}%")

    # Preparar lotes de entrenamiento para afinado ligero
    encoded_texts = [tokenizer.encode(t[0] + "\n<|eos|>\n").ids for t in SPANISH_QA_SAMPLES]
    max_len = max(len(ids) for ids in encoded_texts)
    pad_id = tokenizer.token_to_id("<|pad|>") or 0

    batch_ids = []
    for ids in encoded_texts:
        padded = ids + [pad_id] * (max_len - len(ids))
        batch_ids.append(padded)

    x_tensor = torch.tensor([b[:-1] for b in batch_ids], dtype=torch.long, device=device)
    y_tensor = torch.tensor([b[1:] for b in batch_ids], dtype=torch.long, device=device)

    optimizer = torch.optim.AdamW(model.parameters(), lr=1e-4, weight_decay=0.01)
    model.train()

    print("Ejecutando 15 pasos de afinado en CPU/GPU...")
    for step in range(15):
        optimizer.zero_grad()
        logits = model(x_tensor)
        loss = F.cross_entropy(logits.view(-1, config.vocab_size), y_tensor.view(-1), ignore_index=pad_id)
        loss.backward()
        optimizer.step()
        if step % 5 == 0 or step == 14:
            print(f"  Paso {step:02d} | Loss QA: {loss.item():.4f}")

    acc_after = evaluate_qa_accuracy(model, tokenizer, device)
    print(f"Precisión después del afinado: {acc_after * 100:.1f}%")

    out_report = REPORTS_DIR / "spanish_qa_experiment_metrics.json"
    out_report.parent.mkdir(parents=True, exist_ok=True)
    with open(out_report, "w", encoding="utf-8") as f:
        json.dump({
            "accuracy_before": acc_before,
            "accuracy_after": acc_after,
            "samples_count": len(SPANISH_QA_SAMPLES),
            "steps": 15,
            "isolated": True,
        }, f, indent=2)

    print(f"[REPORTE] Resultados guardados aisladamente en {out_report}")
    print("=" * 60)


if __name__ == "__main__":
    main()
