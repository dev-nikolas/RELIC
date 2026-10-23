#!/usr/bin/env python3
"""Reconstruccion del nucleo H1-shape en Dyck (protocolo docs/PROTOCOL_H1_RIR.md, etapa 2).

Los pesos congelados originales NO existen en el repo [MEDIDO: solo hay spec+metricas en
artifacts/checkpoints/relic_core_h1]. Entonces entrenamos aqui UNA COPIA con la misma
arquitectura (102,720 params) y la MISMA receta certificada (tools/
train_ablation_h1_recurrent.py: 30 epocas, AdamW lr 3e-3 wd 0.01, batch 32, seed 42)
sobre data/benchmarks/dyck. Todo resultado que dependa de estos pesos se etiqueta
[RECONSTRUIDO] y no puede citarse como "H1 congelado".
Salida: reports/eval/prot_h1rir_h1recon_dyck.pt + sha256 en prot_h1rir_intentos.jsonl.
"""
from __future__ import annotations

import hashlib
import json
import operator
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import torch
import torch.nn.functional as F

import train_ablation_h1_recurrent as ABL

VOCAB = ABL.VOCAB_SIZE
sal_tass = Path("__file__").parent
REPORT = Path("reports/eval")
PT = REPORT / "prot_h1rir_h1recon_dyck.pt"


def receta(train=1000, val=200, ood=300):
    with open(f"data/benchmarks/dyck/train.json", encoding="utf-8") as f:
        tr = json.load(f)[:train]
    with open("data/benchmarks/dyck/val_id.json", encoding="utf-8") as f:
        vd = json.load(f)[:val]
    with open("data/benchmarks/dyck/test_ood.json", encoding="utf-8") as f:
        od = json.load(f)[:ood]
    return tr, vd, od


def epoca(modelo, tok, tr):
    import random
    random.seed(42)
    modelo.train()
    total, n = 0.0, 0
    op = torch.optim.AdamW(modelo.parameters(), lr=3e-3, weight_decay=0.01)
    idx = list(range(len(tr)))
    random.shuffle(idx)
    for s0 in range(0, len(idx), 32):
        batch = [tr[i] for i in idx[s0:s0 + 32]]
        full = [tok.encode(f"{s['prompt']} {s['target']}") for s in batch]
        W = max(len(ids) for ids in full)
        xs, ys = [], []
        for s, ids in zip(batch, full):
            pid = tok.encode(f"{s['prompt']} ")
            ids = ids + [tok.pad_id] * (W - len(ids))
            tgt = [-100] * (len(pid) - 1) + ids[len(pid) - 1:] + [-100] * (W - len(ids))
            xs.append(ids[:-1])
            ys.append(tgt[1:])
        inp = torch.tensor(xs, dtype=torch.long)
        obj = torch.tensor(ys, dtype=torch.long)
        op.zero_grad()
        logits = modelo(inp)
        loss = F.cross_entropy(logits.view(-1, VOCAB), obj.view(-1), ignore_index=-100)
        loss.backward()
        op.step()
        total += loss.item()
        n += 1
    return total / max(1, n)


def acc(sector, tok, model):
    model.eval()
    ok = 0
    with torch.no_grad():
        for s in sector:
            tgt = tok.encode(s["target"])
            pid = tok.encode(s["prompt"] + " ")
            if not tgt:
                continue
            ids = tok.encode(s["prompt"]) + [-1]  # se genera autoregresivamente
            good = 0
            seq = pid
            for t in tgt:
                inp = torch.tensor([seq], dtype=torch.long)
                logits = model(inp)[0, -1]
                pred = int(logits.argmax())
                good += int(pred == t)
                seq = seq + [t]
            ok += int(good == len(tgt))
    return round(100.0 * ok / len(sector), 2)


def main() -> None:
    tr, vd, od = receta()
    tok = ABL.CharTokenizer()
    for ronda, maxep in [(1, 30), (2, 60), (3, 100)]:
        torch.manual_seed(42)
        model = ABL.CalibratedRecurrentModel(vocab_size=VOCAB)
        assert model.count_parameters() == 102720, "La paridad 102,720 params es contractual"
        hist = []
        for ep in range(1, maxep + 1):
            L = epoca(model, tok, tr)
            hist.append(round(L, 4))
            if ep % 10 == 0 or ep == maxep:
                print(f"[ronda {ronda}] epoch {ep}/{maxep} loss={L:.4f}", flush=True)
        a_id, a_ood = acc(vd, tok, model), acc(od, tok, model)
        print(f"  dyck val_id={a_id}% ood={a_ood}%")
        if a_id >= 99.0 and a_ood >= 95.0:
            REPORT.mkdir(exist_ok=True)
            torch.save(model.state_dict(), PT)
            sha = hashlib.sha256(PT.read_bytes()).hexdigest()
            reg = {"evento": "reconstruccion_h1", "epocas": maxep, "ronda": ronda,
                   "loss_hist": hist[-6:], "dyck_val_id": a_id, "dyck_ood": a_ood,
                   "pt_sha256": sha, "etiqueta": "[RECONSTRUIDO] peso no citable como congelado",
                   "arquitectura": ABL.CalibratedRecurrentModel(vocab_size=VOCAB).count_parameters()}
            with open(REPORT / "prot_h1rir_intentos.jsonl", "a", encoding="utf-8") as f:
                f.write(json.dumps(reg, ensure_ascii=False) + "\n")
            print(json.dumps(reg, ensure_ascii=False))
            return
    raise SystemExit("[FALLO] La reconstruccion no alcanza 99/95 en dyck; revisar sin tunear")


if __name__ == "__main__":
    main()
