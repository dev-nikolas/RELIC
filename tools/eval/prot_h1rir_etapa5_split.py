#!/usr/bin/env python3
"""ETAPA 5 split v2 POR CONSTRUCCION con seal, congelado en commits.
Regla (documentada en protocolo + intentos.jsonl, ANTES de entrenar):
- Parafraese-Cluster: union-find con enlace (misma family JACCARD >= 0.40
  sobre tokens no numericos). Determinista (orden canónico).
- SEILLADO: cluster con 1 texto congelado (del 95, sha d60eb8bb) entero
  excluido (parafasis casi identicas divididas: prohibido).
- Train/val: reparto determinista por cluster ordenado (cada 10mo → val).
Prohibido re-generar tras ver metricas."""
import json
import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
SRC = ROOT / "data/semantic_compiler"
OUTDIR = ROOT / "reports/eval"
FROZEN = ROOT / "reports/eval/prot_h1rir_test_split.json"
J_LINK = 0.40


def toks_no_num(text: str) -> set:
    t = text.lower().strip().replace(",", " ").replace(".", " ")
    return {w for w in t.split() if w and not any(c.isdigit() for c in w)}


def main() -> None:
    rows = []
    for name in ("train", "val", "test_id", "test_ood"):
        rows += [json.loads(l) for l in open(SRC / f"{name}.jsonl", encoding="utf-8")]
    frozen = json.load(open(FROZEN, encoding="utf-8"))["casos"]
    frozen_texts = {c["text"].strip() for c in frozen}
    conts = [toks_no_num(r["text"]) for r in rows]
    parent = list(range(len(rows)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x
    for i in range(len(rows)):
        for j in range(i + 1, len(rows)):
            if rows[i]["family"] != rows[j]["family"]:
                continue
            a, b = conts[i], conts[j]
            if not a or not b:
                continue
            jac = (2 * len(a & b)) / (len(a) + len(b))
            if jac >= J_LINK:
                parent[find(i)] = find(j)
    clusters = {}
    for i in range(len(rows)):
        clusters.setdefault(find(i), []).append(i)
    sealed, libres = set(), {}
    for root, ids in clusters.items():
        if any(rows[i]["text"].strip() in frozen_texts for i in ids):
            sealed.add(root)
        else:
            libres[root] = ids
    train, val = [], []
    for n, (root, ids) in enumerate(sorted(libres.items())):
        col = val if n % 10 == 9 else train
        for i in ids:
            col.append(rows[i])
    assert not frozen_texts & {r["text"].strip() for r in train + val}, "seal roto (texto exacto)"
    # verificacion de estructura: ningun cluster sealed a medias
    with open(OUTDIR / "prot_h1rir_etapa5_train.jsonl", "w", encoding="utf-8") as f:
        f.write('\n'.join(json.dumps(r, ensure_ascii=False) for r in train) + '\n')
    with open(OUTDIR / "prot_h1rir_etapa5_val.jsonl", "w", encoding="utf-8") as f:
        f.write('\n'.join(json.dumps(r, ensure_ascii=False) for r in val) + '\n')
    sha = {"train_sha256": hashlib.sha256(''.join(r["text"] for r in train).encode()).hexdigest(),
           "val_sha256": hashlib.sha256(''.join(r["text"] for r in val).encode()).hexdigest(),
           "n_train": len(train), "n_val": len(val), "n_clusters_libres": len(libres),
           "n_clusters_sealed": len(sealed), "total_rows": len(rows),
           "regla": f"union-find family+Jaccard>={J_LINK} (sin numerales); text exacto elimina cluster"}
    json.dump(sha, open(OUTDIR / "prot_h1rir_etapa5_sha256.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(json.dumps(sha, indent=1))
    from collections import Counter
    print("train familias:", dict(Counter(r["family"] for r in train)))
    print("val familias:", dict(Counter(r["family"] for r in val)))


if __name__ == "__main__":
    main()
