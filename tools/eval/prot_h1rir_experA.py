#!/usr/bin/env python3
"""Experimento A (etapa 2) del protocolo docs/PROTOCOL_H1_RIR.md — candidatos
A1 (latent H1 [RECONSTRUIDO] 64-d) y A2 (latent + char-bigrams).

Criterios CONGELADOS (commit 05fa461): decision por MEDIANA de tupla exacta
(intent+domain+status) entre semillas {42,123,999};
ZONA VIVA >=59.7%; ZONA GRIS (54.7..59.7) = RECHAZO; <=54.7 RECHAZO inmediato.
TODOS los intentos se registran en prot_h1rir_intentos.jsonl (append-only).
Hiperparametros de candidatos FIJOS por protocolo: hidden 64, lr 0.05, 80 epocas.
"""
from __future__ import annotations

import hashlib
import json
import statistics
import sys
from pathlib import Path

import torch
import torch.nn as nn

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "eval"))
import train_ablation_h1_recurrent as ABL  # noqa: E402
import prot_h1rir as P1  # noqa: E402

SEMILLAS = [42, 123, 999]
ETIQUETA_LAT = "[RECONSTRUIDO]"


def latentes_h1(texts: list[str], vocab_tok: dict) -> torch.Tensor:
    """Codifica cada frase a ids (hash crc32 % 117 + 11) y extrae el estado final
    (64-d) del core H1 reconstruido, capa a capa tal como el forward certificado."""
    torch.manual_seed(42)
    model = ABL.CalibratedRecurrentModel(vocab_size=ABL.VOCAB_SIZE)
    model.load_state_dict(torch.load("reports/eval/prot_h1rir_h1recon_dyck.pt", map_location="cpu"))
    model.eval()
    import zlib
    ids_list = []
    for t in texts:
        ids = [((zlib.crc32(w.encode()) % 117) + 11) for w in P1.nato(t).split()]
        if not ids:
            ids = [0]
        ids_list.append(torch.tensor([ids], dtype=torch.long))
    with torch.no_grad():
        outs = []
        for ids in ids_list:
            x = model.tok_embeddings(ids)
            for layer in model.layers:
                x = layer(x)
            x = model.final_norm(x)
            outs.append(x[0, -1, :])
    return torch.stack(outs)


class Candidato(nn.Module):
    def __init__(self, in_dim: int, clases: dict):
        super().__init__()
        self.keys = list(clases.keys())
        self.hid = nn.Linear(in_dim, 64)
        self.act = nn.ReLU()
        self.out = nn.ModuleDict({k: nn.Linear(64, len(clases[k])) for k in clases})

    def forward(self, x):
        h = self.act(self.hid(x))
        return {k: self.out[k](h) for k in self.keys}


def run_candidato(nombre, Xdev, Ydev, Xf, pf_seed, clases, golds, extra=None):
    torch.manual_seed(pf_seed)
    model = Candidato(Xdev.shape[1], clases)
    opt = torch.optim.Adam(model.parameters(), lr=0.05)
    for _ in range(80):
        opt.zero_grad()
        outs = model(Xdev)
        loss = sum(nn.functional.cross_entropy(outs[k], Ydev[k]) for k in clases)
        loss.backward()
        opt.step()
    with torch.no_grad():
        pr = model(Xf)
    preds = {k: pr[k].argmax(1).tolist() for k in clases}
    accs = {k: round(float(sum(clases[k][preds[k][i]] == golds[i][k] for i in range(len(golds)))) / len(golds), 4) for k in clases}
    tupla = round(100.0 * sum(all(clases[k][preds[k][i]] == golds[i][k] for k in clases) for i in range(len(golds))) / len(golds), 1)
    reg = {"evento": "candidato", "nombre": nombre, "semilla": pf_seed,
           "latente": ETIQUETA_LAT, "accs": accs, "tupla_exacta_pct": tupla,
           "hiperparams": {"hidden": 64, "lr": 0.05, "epochs": 80}, "extra": extra or {}}
    with open("reports/eval/prot_h1rir_intentos.jsonl", "a", encoding="utf-8") as f:
        f.write(json.dumps(reg, ensure_ascii=False) + "\n")
    print(json.dumps(reg, ensure_ascii=False))
    return reg


def main() -> None:
    frozen, dev, dev_sin_leaks, leaks = P1.static_frozen()
    golds = [P1.oro(d) for d in frozen]
    clases = {"intent": ["arithmetic_query", "graph_query", "inventory_update", "unknown"],
              "domain": ["abstract_math", "software_dependency", "spatial_graph", "unsupported", "inventory"],
              "status": ["VALID", "INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]}

    # char-bigram vocab igual que baseline etapa 1 (reproducibilidad)
    vocab_cb = P1.vocab_feats([d["text"] for d in dev_sin_leaks])
    lat_dev = latentes_h1([d["text"] for d in dev_sin_leaks], vocab_cb)
    lat_fr = latentes_h1([g["text"] for g in golds], vocab_cb)

    resultados = {}
    for nombre, Xd, Xf in [("A1_latent_H1", lat_dev, lat_fr)]:
        regs = [run_candidato(nombre, Xd, {k: torch.tensor([clases[k].index(d["rir"][k]) for d in dev_sin_leaks]) for k in clases}, Xf, s, clases, golds) for s in SEMILLAS]
        resultados[nombre] = regs
    Xcb_dev = P1.tensor_txt([d["text"] for d in dev_sin_leaks], vocab_cb)
    Xcb_fr = P1.tensor_txt([g["text"] for g in golds], vocab_cb)
    Xd2 = torch.cat([lat_dev, Xcb_dev], dim=1)
    Xf2 = torch.cat([lat_fr, Xcb_fr], dim=1)
    regs2 = [run_candidato("A2_latent+charbigram", Xd2, {k: torch.tensor([clases[k].index(d["rir"][k]) for d in dev_sin_leaks]) for k in clases}, Xf2, s, clases, golds) for s in SEMILLAS]
    resultados["A2_latent+charbigram"] = regs2

    decision = []
    for nombre, regs in resultados.items():
        meds = statistics.median([r["tupla_exacta_pct"] for r in regs])
        zona = "VIVA" if meds >= 59.7 else ("GRIS=RECHAZO" if meds > 54.7 else "RECHAZO")
        decision.append({"candidato": nombre, "tuplas": [r["tupla_exacta_pct"] for r in regs], "mediana": meds, "zona": zona})
    veredicto = {"protocolo": "docs/PROTOCOL_H1_RIR.md etapa 2", "sha_split": json.load(open("reports/eval/prot_h1rir_test_split.json", encoding="utf-8"))["sha256"],
                 "decisiones": decision, "veredicto_global": "SIGUE" if any(d["zona"] == "VIVA" for d in decision) else "RECHAZO"}
    with open("reports/eval/prot_h1rir_experA_decision.json", "w", encoding="utf-8") as f:
        json.dump(veredicto, f, ensure_ascii=False, indent=1)
    print(json.dumps(veredicto, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
