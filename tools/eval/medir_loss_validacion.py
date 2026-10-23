#!/usr/bin/env python3
"""Medicion de perdida de validacion del checkpoint relic_10m sobre data/val.txt,
con dos referencias baratas que un lenguaje dugno superar: (a) uniforme ln(vocab),
(b) un modelo bigram entrenado SOLO sobre data/train.txt.

Uso: python tools/eval/medir_loss_validacion.py [--max_batches N] [--batch 32] [--seq 256]
Salida: reports/eval/loss_validacion.json + consola. Solo medicion.
"""
from __future__ import annotations

import argparse
import json
import math
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "python"))

import torch
import torch.nn as nn
from tokenizers import Tokenizer


def load_model(ckpt_path: str):
    from relic.model import RelicForCausalLM
    from relic.config import RelicConfig
    ck = torch.load(ckpt_path, map_location="cpu", weights_only=False)
    cfg = RelicConfig(**ck["config"])
    model = RelicForCausalLM(cfg)
    sd = ck["model"]
    missing, unexpected = model.load_state_dict(sd, strict=False)
    if unexpected:
        print(f"[warn] tensores inesperados: {unexpected}")
    model.eval()
    return model, ck.get("step")


@torch.no_grad()
def eval_chunks(model, ids, seq=256, batch=32, max_batches=0):
    total_n, total_loss = 0, 0.0
    n_chunks = (len(ids) - 1) // seq
    buf = []
    for c in range(n_chunks):
        chunk = torch.tensor(ids[c * seq:(c + 1) * seq + 1])
        buf.append(chunk)
        if len(buf) == batch or c == n_chunks - 1:
            x = torch.stack(buf)[:, :-1]
            y = torch.stack(buf)[:, 1:]
            logits = model(x)
            loss = nn.functional.cross_entropy(
                logits.reshape(-1, logits.size(-1)).float(), y.reshape(-1), reduction="sum")
            total_loss += loss.item()
            total_n += y.numel()
            buf = []
            done = c // batch + 1
            if max_batches and done >= max_batches:
                break
            if done % 50 == 0:
                print(f"  chunk {c+1}/{n_chunks} loss_avg={total_loss/max(total_n,1):.4f}", flush=True)
    return total_loss / max(total_n, 1), total_n


def bigram_baseline(train_ids, val_ids, vocab):
    counts = torch.zeros(vocab, vocab, dtype=torch.float64)
    t = torch.tensor(train_ids, dtype=torch.long)
    uniq, inv = torch.unique(t, return_inverse=True)
    counts.index_put_((inv[:-1], inv[1:]), torch.ones(len(inv) - 1, dtype=torch.float64), accumulate=True)
    row = counts.sum(dim=1)
    probs = ((counts + 1.0) / (row.unsqueeze(1) + float(len(uniq)))).clamp_min(1e-12)
    logp = probs.log()
    v = torch.tensor(val_ids, dtype=torch.long)
    seq_v = torch.searchsorted(uniq, v)
    in_train = torch.isin(v, uniq)
    seq_v = torch.where(in_train, seq_v, torch.full_like(seq_v, -1))
    pairs = torch.stack((seq_v[:-1], seq_v[1:]), dim=1)
    lp = logp[pairs[:, 0], pairs[:, 1]]
    ok = (pairs[:, 0] >= 0) & (pairs[:, 1] >= 0)
    return -lp[ok].mean().item(), int(ok.sum())


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--ckpt", default="artifacts/checkpoints/relic_10m_latest.pt")
    p.add_argument("--tok", default="artifacts/tokenizer/tokenizer.json")
    p.add_argument("--val", default="data/val.txt")
    p.add_argument("--train", default="data/train.txt")
    p.add_argument("--seq", type=int, default=256)
    p.add_argument("--batch", type=int, default=32)
    p.add_argument("--max_batches", type=int, default=0)
    a = p.parse_args()

    tok = Tokenizer.from_file(a.tok)
    n_vocab_toker = tok.get_vocab_size(with_added_tokens=True)
    val_ids = tok.encode(open(a.val, encoding="utf-8").read()).ids
    train_ids = tok.encode(open(a.train, encoding="utf-8").read()).ids
    print(f"[tok] vocab_tokenizer={n_vocab_toker} | train_toks={len(train_ids)} val_toks={len(val_ids)}")

    model, step = load_model(a.ckpt)
    vocab_model = model.config.vocab_size
    loss_model, n_eval = eval_chunks(model, val_ids, seq=a.seq, batch=a.batch, max_batches=a.max_batches)
    loss_uniform_tok = math.log(n_vocab_toker)
    loss_uniform_model = math.log(vocab_model)
    loss_bigram, n_bigram = bigram_baseline(train_ids, val_ids, n_vocab_toker)

    res = {
        "checkpoint": a.ckpt, "step": step, "val_loss_nats_token": round(loss_model, 4),
        "val_ppl": round(math.exp(loss_model), 2), "tokens_evaluados": n_eval,
        "secuencia": a.seq, "batch": a.batch,
        "referencia_uniforme_tokenizer_ln(vocab)": round(loss_uniform_tok, 4),
        "referencia_uniforme_vocab_modelo_ln(8192)": round(loss_uniform_model, 4),
        "referencia_bigram_val_loss": round(loss_bigram, 4),
        "tokens_bigram_eval": n_bigram,
        "vocab_tokenizer": n_vocab_toker, "vocab_modelo": vocab_model,
        "notas": "[MEDIDO] GPU=none CPU. secuencia de evaluacion no superpuesta (chunking). bigram Laplace(+1) sobre train completo.",
    }
    os.makedirs("reports/eval", exist_ok=True)
    with open("reports/eval/loss_validacion.json", "w", encoding="utf-8") as f:
        json.dump(res, f, ensure_ascii=False, indent=2)
    print(json.dumps(res, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
