#!/usr/bin/env python3
"""
tools/leak_control.py
Auditoría y reporte de solapamiento de n-gramas (2-gramas y 3-gramas) entre splits:
Train vs Holdout-Template vs Dev vs External. Menos de 180 líneas.
"""
import json, os, sys
from typing import Set

def get_ngrams(text: str, n: int) -> Set[str]:
    toks = text.lower().strip().split()
    if len(toks) < n: return set()
    return {" ".join(toks[i:i+n]) for i in range(len(toks) - n + 1)}

def main():
    ds_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "data", "curriculum_dataset_v1.jsonl"))
    dev_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "tests", "dev_suite_v1.json"))
    ext_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "tests", "external_human_gold_v1.json"))

    if not os.path.exists(ds_path):
        print(f"Error: {ds_path} no encontrado.")
        sys.exit(1)

    train_prompts, holdout_prompts = [], []
    with open(ds_path, "r", encoding="utf-8") as f:
        for line in f:
            item = json.loads(line)
            if item["split"] == "train":
                train_prompts.append(item["prompt"])
            else:
                holdout_prompts.append(item["prompt"])

    dev_prompts = []
    if os.path.exists(dev_path):
        with open(dev_path, "r", encoding="utf-8") as f:
            dev_cases = json.load(f)
            dev_prompts = [c["input"] for c in dev_cases if c.get("input")]

    ext_prompts = []
    if os.path.exists(ext_path):
        with open(ext_path, "r", encoding="utf-8") as f:
            ext_data = json.load(f)
            ext_prompts = [c["input"] for c in ext_data.get("cases", []) if c.get("input")]

    train_3g = {g for p in train_prompts for g in get_ngrams(p, 3)}
    train_2g = {g for p in train_prompts for g in get_ngrams(p, 2)}
    holdout_3g = {g for p in holdout_prompts for g in get_ngrams(p, 3)}
    dev_3g = {g for p in dev_prompts for g in get_ngrams(p, 3)}

    h_overlap = holdout_3g.intersection(train_3g)
    d_overlap = dev_3g.intersection(train_3g)

    print("================================================================================")
    print("  RELIC — REPORTE DE CONTROL DE FUGAS Y SOLAPAMIENTO DE N-GRAMAS")
    print("================================================================================")
    print(f"Dataset: curriculum_dataset_v1.jsonl")
    print(f"  Total Train Prompts:   {len(train_prompts)} | 3-gramas únicos: {len(train_3g)} | 2-gramas: {len(train_2g)}")
    print(f"  Total Holdout Prompts: {len(holdout_prompts)} | 3-gramas únicos: {len(holdout_3g)}")
    print(f"  Solapamiento Train vs Holdout (3-gramas): {len(h_overlap)}/{len(holdout_3g)} ({len(h_overlap)/max(1, len(holdout_3g))*100:.2f}%)")
    print(f"  Solapamiento Train vs Dev Suite (3-gramas): {len(d_overlap)}/{len(dev_3g)} ({len(d_overlap)/max(1, len(dev_3g))*100:.2f}%)")
    if ext_prompts:
        ext_3g = {g for p in ext_prompts for g in get_ngrams(p, 3)}
        e_overlap = ext_3g.intersection(train_3g)
        print(f"  Solapamiento Train vs External Human (3-gramas): {len(e_overlap)}/{len(ext_3g)} ({len(e_overlap)/max(1, len(ext_3g))*100:.2f}%)")
    else:
        print("  Banco Externo Humano: PENDIENTE DE RELLENAR POR TERCEROS (0 inputs presentes).")
    print("================================================================================")

if __name__ == "__main__":
    main()
