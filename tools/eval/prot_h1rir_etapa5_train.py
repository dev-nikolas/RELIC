#!/usr/bin/env python3
"""ETAPA 5 entrenador de cabeza v2 (protocolo congelado e4d06d7).
Arquitectura IDENTICA a A2 (etapa 3): latente H1 [RECONSTRUIDO, sha
7448feab] + char-bigram. Cambios SOLO-DATA respecto de etapa 2/3 (limpio
de seal, vocab excl de textos selados), arquitectura sin cofundido.
Corrida unica x semilla {42,123,999}; hiperparametros fijos (hidden 64,
lr 0.05, 80 epocas, Adam); SIN early stopping, SIN tuning. Pesos firmados
sha256 (archivo no versionado por politica *.pt) + intentos append-only."""
import json
import hashlib
import sys
from pathlib import Path

import torch
import torch.nn as nn

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "eval"))
import prot_h1rir as P1
import prot_h1rir_experA as EA
from prot_h1rir_etapa3_preds import CLASES, SEMILLAS


def main() -> None:
    train = [json.loads(l) for l in open(ROOT / "reports/eval/prot_h1rir_etapa5_train.jsonl", encoding="utf-8") if l.strip()]
    val = [json.loads(l) for l in open(ROOT / "reports/eval/prot_h1rir_etapa5_val.jsonl", encoding="utf-8") if l.strip()]
    frozen, _, _, _ = P1.static_frozen()
    corpus = [r["text"] for r in train + val]
    vocab = P1.vocab_feats(corpus)
    Xtr = torch.cat([EA.latentes_h1(corpus, None), P1.tensor_txt(corpus, vocab)], dim=1)
    Ys = {k: torch.tensor([CLASES[k].index(r["rir"][k]) for r in train + val]) for k in CLASES}
    lat_fr = EA.latentes_h1([g["text"] for g in frozen], None)
    Xfr = torch.cat([lat_fr, P1.tensor_txt([g["text"] for g in frozen], vocab)], dim=1)
    preds, metricas, shas = {}, {}, {}
    for s in SEMILLAS:
        torch.manual_seed(s)
        model = EA.Candidato(Xtr.shape[1], CLASES)
        opt = torch.optim.Adam(model.parameters(), lr=0.05)
        for _ in range(80):
            opt.zero_grad()
            outs = model(Xtr)
            loss = sum(nn.functional.cross_entropy(outs[k], Ys[k]) for k in Ys)
            loss.backward()
            opt.step()
        torch.save(model.state_dict(), ROOT / f"reports/eval/prot_h1rir_headv2_seed{s}.pt")
        shas[str(s)] = hashlib.sha256(open(ROOT / f"reports/eval/prot_h1rir_headv2_seed{s}.pt", "rb").read()).hexdigest()
        with torch.no_grad():
            pr = model(Xfr)
        casos = []
        aciertos = {"intent": 0, "domain": 0, "status": 0, "tupla": 0}
        for i, g in enumerate(frozen):
            pi = CLASES["intent"][pr["intent"][i].argmax().item()]
            pd = CLASES["domain"][pr["domain"][i].argmax().item()]
            ps = CLASES["status"][pr["status"][i].argmax().item()]
            casos.append({"id": f"h1rir_{i}", "pred_intent": pi, "pred_status": ps})
            aciertos["intent"] += pi == g["intent"]
            aciertos["domain"] += pd == g["domain"]
            aciertos["status"] += ps == g["status"]
            aciertos["tupla"] += (pi == g["intent"] and pd == g["domain"] and ps == g["status"])
        preds[str(s)] = casos
        metricas[str(s)] = {k: round(100 * v / len(frozen), 1) for k, v in aciertos.items()}
        from collections import Counter
        print(f"semilla {s}: disp={Counter((c['pred_intent'], c['pred_status']) for c in casos)}")
    json.dump(preds, open(ROOT / "reports/eval/prot_h1rir_bridge_preds_v2.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    json.dump({"metricas": metricas, "pesos_sha256": shas, "hiperparams": {"hidden": 64, "lr": 0.05, "epochs": 80, "seeds": SEMILLAS},
               "n_train": len(train) + len(val), "vocab_excluido": "clusters sealed", "arquitectura": "A2 identica"},
              open(ROOT / "reports/eval/prot_h1rir_etapa5_head_metrics.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    with open(ROOT / "reports/eval/prot_h1rir_intentos.jsonl", "a", encoding="utf-8") as f:
        f.write(json.dumps({"fase": "etapa5-headv2", "arquitectura": "A2", "split_sealed": True,
                            "pesos_sha256": shas, "metricas_frozen": metricas}, ensure_ascii=False) + "\n")
    print(json.dumps(metricas, indent=1))
    print("pesos_sha256:", json.dumps(shas))


if __name__ == "__main__":
    main()
