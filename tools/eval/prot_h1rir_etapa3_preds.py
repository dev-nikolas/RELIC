#!/usr/bin/env python3
"""ETAPA 3 (protocolo H1->RIR): predice intent/status para el split congelado con
3 semillas del candidato A2 congelado en etapa 2 (H1 [RECONSTRUIDO]+char-bigrama).

POR DISPLIN: no hay tuning, ni semillas nuevas, ni lookahead de resultados. La salida
(alimentador del binario puente) preserva la asignacion caso->estados predichos.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import torch
import torch.nn as nn

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "eval"))
import train_ablation_h1_recurrent as ABL
import prot_h1rir as P1
import prot_h1rir_experA as EA

SEMILLAS = [42, 123, 999]  # congeladas por protocolo
SPLIT = "reports/eval/prot_h1rir_test_split.json"


def entrenar(X, Ys, semilla):
    torch.manual_seed(semilla)
    model = EA.Candidato(X.shape[1], {"intent": CLASES["intent"], "domain": CLASES["domain"], "status": CLASES["status"]})
    opt = torch.optim.Adam(model.parameters(), lr=0.05)
    for _ in range(80):
        opt.zero_grad()
        outs = model(X)
        loss = sum(nn.functional.cross_entropy(outs[k], Ys[k]) for k in Ys)
        loss.backward()
        opt.step()
    return model


CLASES = {"intent": ["arithmetic_query", "graph_query", "inventory_update", "unknown"],
          "domain": ["abstract_math", "software_dependency", "spatial_graph", "unsupported", "inventory"],
          "status": ["VALID", "INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]}


def main() -> None:
    frozen, dev, dev_sin_leaks, leaks = P1.static_frozen()
    Xcb = P1.tensor_txt([d["text"] for d in dev_sin_leaks], P1.vocab_feats([d["text"] for d in dev_sin_leaks]))
    lat_dev = EA.latentes_h1([d["text"] for d in dev_sin_leaks], None)
    X = torch.cat([lat_dev, Xcb], dim=1)
    Ys = {k: torch.tensor([CLASES[k].index(d["rir"][k]) for d in dev_sin_leaks]) for k in CLASES}
    lat_fr = EA.latentes_h1([g["text"] for g in frozen], None)
    Xfr = torch.cat([lat_fr, P1.tensor_txt([g["text"] for g in frozen], P1.vocab_feats([d["text"] for d in dev_sin_leaks]))], dim=1)

    preds = {}
    for s in SEMILLAS:
        m = entrenar(X, Ys, s)
        with torch.no_grad():
            pr = m(Xfr)
        casos = []
        for i, g in enumerate(frozen):
            casos.append({"id": f"h1rir_{i}", "pred_intent": CLASES["intent"][pr["intent"][i].argmax().item()],
                          "pred_status": CLASES["status"][pr["status"][i].argmax().item()]})
        preds[str(s)] = casos
        from collections import Counter
        print(f"semilla {s}: disp={Counter((c['pred_intent'], c['pred_status']) for c in casos)}")
    with open("reports/eval/prot_h1rir_bridge_preds.json", "w", encoding="utf-8") as f:
        json.dump(preds, f, ensure_ascii=False, indent=1)
    json.dump([{"input": g["text"], "id": f"h1rir_{i}"} for i, g in enumerate(frozen)],
              open("reports/eval/prot_h1rir_bridge_cases.json", "w", encoding="utf-8"), ensure_ascii=False, indent=0)
    print("preds + cases escritos para el binario puente")


if __name__ == "__main__":
    main()
