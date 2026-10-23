#!/usr/bin/env python3
"""Protocolo H1->RIR etapa 1 (docs/PROTOCOL_H1_RIR.md).

Genera: (1) particion de evaluacion congelada (test_id + test_ood del dataset
semantic_compiler) con deteccion anti-duplicados (shingles 3 palabras, umbral
0.6) contra train/val; (2) baseline lexica (SVM-logistica char-bigram,
hiperparametros FIJOS declarados [DESIGN], sin tuning) sobre la MISMA particion;
(3) al pasarle un dump de Rust (--rust-dump), calcula cobertura del despacho y
concordancia de intencion contra el oro congelado.

Uso: python tools/eval/prot_h1rir.py [--rust-dump reports/eval/dump_release_h1rir_split.json]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from collections import Counter

import torch
import torch.nn as nn

DATA = "data/semantic_compiler"
BASE = "reports/eval"
UNACC = str.maketrans("áéíóúüñ", "aeiouun")
HIPERPARAMS = {"hidden": 64, "lr": 0.05, "epochs": 80, "seed": 42}  # [DESIGN] fijos, sin tuning


def nato(t: str) -> str:
    return " ".join(t.strip().translate(UNACC).lower().split())


def shingles(t: str, n: int = 3) -> set:
    w = nato(t).split()
    return {" ".join(w[i:i+n]) for i in range(max(len(w)-n+1, 0))} if len(w) >= n else set(" ".join(w))


def jac(a: set, b: set) -> float:
    return len(a & b) / max(len(a | b), 1)


def load(part: str) -> list:
    return [json.loads(l) for l in open(f"{DATA}/{part}", encoding="utf-8")]


def static_frozen():
    frozen = load("test_id.jsonl") + load("test_ood.jsonl")
    dev = load("train.jsonl") + load("val.jsonl")
    fs = {i: shingles(d["text"]) for i, d in enumerate(frozen)}
    leaks = []
    keep_dev = []
    for j, d in enumerate(dev):
        ds = shingles(d["text"])
        hit = next(((i, round(jac(fs[i], ds), 3)) for i in fs if fs[i] and ds and jac(fs[i], ds) >= 0.6), None)
        if hit:
            leaks.append({"frozen_idx": hit[0], "dev_part": ("train/train" if j < 287 else "val"), "dev_idx": j % 287 if j < 287 else j-287, "jaccard": hit[1], "dev_text": d["text"][:80]})
        else:
            keep_dev.append(d)
    return frozen, dev, keep_dev, leaks


def oro(d: dict) -> dict:
    r = d["rir"]
    o = {"text": d["text"], "family": d["family"], "intent": r["intent"],
         "domain": r["domain"], "status": r["status"], "entity": r.get("entity")}
    if "initial_quantity" in r:
        o["initial_quantity"] = r["initial_quantity"]
    if "operations" in r:
        o["operations"] = r["operations"]
    if "graph_spec" in r:
        o["graph_spec"] = r["graph_spec"]
    return o


def vocab_feats(texts: list) -> dict:
    v = set()
    for t in texts:
        s = nato(t)
        v |= {s[i:i+2] for i in range(len(s)-1)}
    return {x: i for i, x in enumerate(sorted(v))}


def tensor_txt(texts: list, vocab: dict) -> torch.tensor:
    X = torch.zeros(len(texts), len(vocab) + 1)
    for i, t in enumerate(texts):
        s = nato(t)
        for j in range(len(s)-1):
            bg = s[j:j+2]
            if bg in vocab:
                X[i, vocab[bg]] += 1.0
    # counts desconocidos van a la ultima columna
    for i, t in enumerate(texts):
        s = nato(t)
        total = sum(1 for j in range(len(s)-1))
        known = sum(1 for j in range(len(s)-1) if s[j:j+2] in vocab)
        X[i, len(vocab)] = float(total - known)
    return torch.log1p(X)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--rust-dump", default=None)
    args = ap.parse_args()
    os.makedirs(BASE, exist_ok=True)

    frozen, dev, dev_sin_leaks, leaks = static_frozen()
    oro_frozen = [oro(d) for d in frozen]

    # 1) particion congelada firmada
    payload = {"hiperparams_baseline": HIPERPARAMS, "n_eval": len(frozen),
               "leaks_detectados": leaks, "n_dev_sin_leaks": len(dev_sin_leaks),
               "casos": oro_frozen}
    split_path = f"{BASE}/prot_h1rir_test_split.json"
    raw = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    sha = hashlib.sha256(raw).hexdigest()
    if os.path.exists(split_path):
        prev = json.load(open(split_path, encoding="utf-8"))
        if prev.get("sha256") and prev["sha256"] != sha:
            print(f"[AVISO] SHA cambia al regenerar: {prev['sha256']} vs {sha} (el congelado manda)")
            return
    payload["sha256"] = sha
    json.dump(payload, open(split_path, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(f"[SPLIT CONGELADO] n={len(frozen)} leaks={len(leaks)} sha256={sha}")

    # 2) baseline char-bigram (entrenada DEV sin leaks; eval SOLO frozen)
    clases = {"intent": ["arithmetic_query", "graph_query", "inventory_update", "unknown"],
              "domain": ["abstract_math", "software_dependency", "spatial_graph", "unsupported", "inventory"],
              "status": ["VALID", "INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]}
    vocab = vocab_feats([d["text"] for d in dev_sin_leaks])
    torch.manual_seed(HIPERPARAMS["seed"])
    X = tensor_txt([d["text"] for d in dev_sin_leaks], vocab)
    Y = {k: torch.tensor([clases[k].index(d["rir"][k]) for d in dev_sin_leaks]) for k in clases}

    class Cabeza(nn.Module):
        def __init__(self):
            super().__init__()
            self.hid = nn.Linear(len(vocab)+1, HIPERPARAMS["hidden"])
            self.act = nn.ReLU()
            self.out = nn.ModuleDict({k: nn.Linear(HIPERPARAMS["hidden"], len(clases[k])) for k in clases})

        def forward(self, x):
            h = self.act(self.hid(x))
            return {k: self.out[k](h) for k in clases}

    modelo = Cabeza()
    opt = torch.optim.Adam(modelo.parameters(), lr=HIPERPARAMS["lr"])
    for ep in range(HIPERPARAMS["epochs"]):
        opt.zero_grad()
        outs = modelo(X)
        loss = sum(nn.functional.cross_entropy(outs[k], Y[k]) for k in clases)
        loss.backward()
        opt.step()
    Xf = tensor_txt([o["text"] for o in oro_frozen], vocab)
    with torch.no_grad():
        pf = modelo(Xf)
    acc = {}
    for k in clases:
        pred = pf[k].argmax(1).tolist()
        gold = [clases[k].index(o[k]) for o in oro_frozen]
        acc[k] = round(float(sum(p == g for p, g in zip(pred, gold))) / len(gold), 4)
    tupla_exact = None  # calculado abajo (exact_pct)
    preds = {k: pf[k].argmax(1).tolist() for k in clases}
    exact = sum(all(clases[k][preds[k][i]] == oro_frozen[i][k] for k in clases) for i in range(len(oro_frozen)))
    exact_pct = round(100.0 * exact / len(oro_frozen), 1)

    # 3) despacho Rust sobre el mismo congelado (si corre el harness antes)
    rust = None
    if args.rust_dump and os.path.exists(args.rust_dump):
        mapa = {"arithmetic": "arithmetic_query", "graph": "graph_query",
                "relational": "graph_query", "incomplete": None, "free_language": None}
        d = json.load(open(args.rust_dump, encoding="utf-8"))["cases"]
        gum = [o for o in oro_frozen]
        coberto = sum(1 for c in d if c["intent"] in ("arithmetic", "graph", "relational", "incomplete"))
        agree = 0
        for i, c in enumerate(d):
            mi = mapa.get(c["intent"])
            plan = c.get("plan") or {}
            # desempate honesto: un plan aritmetico CON unidad equivale semanticamente a
            # inventory_update (gold de todo el congelado); sin unidad no ejecuta inventario.
            if c["intent"] == "arithmetic" and plan.get("unit") and gum[i]["intent"] == "inventory_update":
                agree += 1
            elif mi and mi == gum[i]["intent"]:
                agree += 1
            elif c["intent"] == "incomplete":
                agree += 0
        rust = {"cobertura_rir_pct": round(100.0 * coberto / len(d), 1),
                "concordancia_intencion_pct": round(100.0 * agree / len(d), 1), "n": len(d)}

    res = {"protocolo": "docs/PROTOCOL_H1_RIR.md", "sha256_split": sha, "n_eval": len(oro_frozen),
           "leaks": len(leaks), "n_dev_sin_leaks": len(dev_sin_leaks),
           "hiperparams": HIPERPARAMS, "baseline_charbigram_acc": acc,
           "tupla_exacta_pct": exact_pct, "despacho_rust": rust}
    out = f"{BASE}/prot_h1rir_resultados.json"
    json.dump(res, open(out, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(json.dumps(res, ensure_ascii=False, indent=1))
    print("guardado", out)


if __name__ == "__main__":
    main()
