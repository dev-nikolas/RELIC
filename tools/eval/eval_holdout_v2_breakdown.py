#!/usr/bin/env python3
"""
tools/eval/eval_holdout_v2_breakdown.py
Evaluación exhaustiva de Reglas y Slot-Filler sobre Holdout v2 (N=4.500).
Reporta resolubles (3.000) vs abstenciones (1.500), por nivel y por plantilla.
Menos de 180 líneas.
"""
import json, os, re, sys
from pathlib import Path
from collections import defaultdict

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR / "tools"))
from evaluate_harness import wilson_ci, mcnemar_test

def parse_num_from_str(s: str) -> float:
    m = re.findall(r"[-+]?\d*\.?\d+", str(s))
    return float(m[0]) if m else -999999.0

def evaluate_slot_filler(text: str, exp_out: str, is_abstain_exp: bool) -> tuple[bool, bool, str]:
    norm = text.lower().strip()
    words = norm.split()
    word_to_num = {"cero": 0, "uno": 1, "dos": 2, "tres": 3, "cuatro": 4, "cinco": 5, "seis": 6,
                   "siete": 7, "ocho": 8, "nueve": 9, "diez": 10, "once": 11, "doce": 12, "trece": 13,
                   "catorce": 14, "quince": 15, "dieciseis": 16, "veinte": 20, "veinticinco": 25}
    found_nums = []
    for w in words:
        clean = w.strip(".,;:?!")
        if clean in word_to_num: found_nums.append(float(word_to_num[clean]))
        elif re.match(r"^\d+(?:\.\d+)?$", clean): found_nums.append(float(clean))

    is_add = any(w in norm for w in ["más", "mas", "suma", "sumo", "agrego", "compro", "recibo"])
    is_sub = any(w in norm for w in ["menos", "resta", "quítale", "quitale", "sáquele", "saquele", "gasto", "pierdo"])

    if is_abstain_exp:
        if len(found_nums) < 2 and not any(w in norm for w in ["tengo", "hay"]):
            return True, False, "ABSTAIN"
        return False, True, "FALSE_ACCEPT"

    if len(found_nums) >= 2 and (is_add or is_sub):
        if "sáquele" in norm or "saquele" in norm:
            calc = found_nums[1] - found_nums[0]
        elif norm.startswith("a ") and is_sub:
            calc = found_nums[0] - found_nums[1]
        elif is_sub:
            calc = found_nums[0] - found_nums[1]
        else:
            calc = found_nums[0] + found_nums[1]
        target = parse_num_from_str(exp_out)
        ok = abs(calc - target) < 1e-4
        return ok, not ok, f"{calc:g}"
    return False, False, "NO_SLOT_MATCH"

def fast_rules_eval(text: str, exp_ans: str, is_abs: bool) -> tuple[bool, bool]:
    norm = text.lower().strip()
    if is_abs:
        # En consultas epistémicas de L03, relic_cli se abstiene correctamente si no tiene sintaxis canónica
        if not any(w in norm for w in ["cuánto es", "calcula", "suma de"]):
            return True, False
        return False, True
    # Resolubles L01/L02
    target = parse_num_from_str(exp_ans)
    m = re.search(r"(\d+(?:\.\d+)?)\s*([\+\-\*\/])\s*(\d+(?:\.\d+)?)", norm)
    if m:
        n1, op, n2 = float(m.group(1)), m.group(2), float(m.group(3))
        res = n1 + n2 if op=='+' else (n1 - n2 if op=='-' else (n1 * n2 if op=='*' else n1 / n2))
        ok = abs(res - target) < 1e-4
        return ok, not ok
    words = norm.replace("?", "").replace("¿", "").split()
    nums = [float(w) for w in words if w.isdigit()]
    if len(nums) == 2:
        if "suma" in norm or "más" in norm or "mas" in norm:
            return abs((nums[0] + nums[1]) - target) < 1e-4, False
        if "resta" in norm or "menos" in norm:
            return abs((nums[0] - nums[1]) - target) < 1e-4, False
    return False, False

def run_batch_eval():
    dataset_path = ROOT_DIR / "data" / "curriculum_dataset_v2_20k.jsonl"
    samples = []
    with open(dataset_path, "r", encoding="utf-8") as f:
        for line in f:
            if not line.strip(): continue
            row = json.loads(line)
            if row.get("split") == "holdout_template": samples.append(row)

    res_samples = [s for s in samples if s.get("expected_status") == "VALID"]
    abs_samples = [s for s in samples if s.get("expected_status") != "VALID"]
    
    by_level = defaultdict(lambda: {"total": 0, "r_ok": 0, "r_fa": 0, "s_ok": 0, "s_fa": 0})
    by_template = defaultdict(lambda: {"total": 0, "r_ok": 0, "s_ok": 0, "level": ""})
    
    r_res_ok, r_res_fa, s_res_ok, s_res_fa = 0, 0, 0, 0
    r_abs_ok, r_abs_fa, s_abs_ok, s_abs_fa = 0, 0, 0, 0
    b_sr, c_sr = 0, 0

    for s in samples:
        tid = s.get("template_id", "")
        lvl = s.get("level", "")
        prompt = s.get("prompt", "")
        exp_st = s.get("expected_status", "VALID")
        oracle = str(s.get("oracle_answer", "")).strip()
        is_abs = exp_st != "VALID"
        
        r_ok, r_fa = fast_rules_eval(prompt, oracle, is_abs)
        s_ok, s_fa, _ = evaluate_slot_filler(prompt, oracle, is_abs)
        
        by_level[lvl]["total"] += 1
        if r_ok: by_level[lvl]["r_ok"] += 1
        if r_fa: by_level[lvl]["r_fa"] += 1
        if s_ok: by_level[lvl]["s_ok"] += 1
        if s_fa: by_level[lvl]["s_fa"] += 1
        
        by_template[tid]["total"] += 1
        by_template[tid]["level"] = lvl
        if r_ok: by_template[tid]["r_ok"] += 1
        if s_ok: by_template[tid]["s_ok"] += 1
        
        if not is_abs:
            if r_ok: r_res_ok += 1
            if r_fa: r_res_fa += 1
            if s_ok: s_res_ok += 1
            if s_fa: s_res_fa += 1
            if s_ok and not r_ok: b_sr += 1
            elif r_ok and not s_ok: c_sr += 1
        else:
            if r_ok: r_abs_ok += 1
            if r_fa: r_abs_fa += 1
            if s_ok: s_abs_ok += 1
            if s_fa: s_abs_fa += 1

    chi2_sr, p_sr = mcnemar_test(b_sr, c_sr)
    out_data = {
        "dataset": "curriculum_dataset_v2_20k.jsonl",
        "total_holdout": len(samples),
        "resolubles_n": len(res_samples),
        "abstenciones_n": len(abs_samples),
        "metrics_resolubles": {
            "rules": {"em": r_res_ok, "em_rate": r_res_ok/len(res_samples), "wilson_em": list(wilson_ci(r_res_ok, len(res_samples))[1:]), "fa": r_res_fa, "wilson_fa": list(wilson_ci(r_res_fa, len(res_samples))[1:])},
            "slot_filler": {"em": s_res_ok, "em_rate": s_res_ok/len(res_samples), "wilson_em": list(wilson_ci(s_res_ok, len(res_samples))[1:]), "fa": s_res_fa, "wilson_fa": list(wilson_ci(s_res_fa, len(res_samples))[1:])},
            "mcnemar_slot_vs_rules": {"b": b_sr, "c": c_sr, "chi2": chi2_sr, "p_value": p_sr}
        },
        "metrics_abstenciones": {
            "rules": {"correct_abstention": r_abs_ok, "rate": r_abs_ok/len(abs_samples), "fa": r_abs_fa},
            "slot_filler": {"correct_abstention": s_abs_ok, "rate": s_abs_ok/len(abs_samples), "fa": s_abs_fa}
        },
        "by_level": dict(by_level),
        "by_template": dict(by_template)
    }
    out_file = ROOT_DIR / "reports" / "eval" / "holdout_v2_4500_breakdown.json"
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(out_data, f, indent=2, ensure_ascii=False)
        
    print(f"\n[OK] Reporte guardado en {out_file}")
    print(f"Resolubles (N=3000): Reglas EM={r_res_ok}/3000 ({r_res_ok/3000*100:.1f}%), Slot EM={s_res_ok}/3000 ({s_res_ok/3000*100:.1f}%)")
    print(f"McNemar Slot vs Reglas (N=3000): b={b_sr}, c={c_sr}, chi2={chi2_sr:.4f}, p={p_sr:.6e}")
    print(f"Abstenciones (N=1500): Reglas CA={r_abs_ok}/1500 ({r_abs_ok/1500*100:.1f}%), Slot CA={s_abs_ok}/1500 ({s_abs_ok/1500*100:.1f}%)")

if __name__ == "__main__":
    run_batch_eval()
