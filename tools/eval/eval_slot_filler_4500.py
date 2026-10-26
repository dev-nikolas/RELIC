#!/usr/bin/env python3
"""
tools/eval/eval_slot_filler_4500.py
Evalúa el Slot-Filler gramatical restringido sobre los 4.500 casos de holdout v2.
Menos de 180 líneas.
"""
import json, math, os, re, sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR / "tools"))
from evaluate_harness import wilson_ci

def evaluate_slot_filler(text: str, exp_out: str, is_abstain_exp: bool) -> tuple[bool, bool, str]:
    norm = text.lower().strip()
    words = norm.split()
    word_to_num = {
        "cero": 0, "uno": 1, "un": 1, "una": 1, "dos": 2, "tres": 3, "cuatro": 4, "cinco": 5, "seis": 6,
        "siete": 7, "ocho": 8, "nueve": 9, "diez": 10, "once": 11, "doce": 12, "trece": 13,
        "catorce": 14, "quince": 15, "dieciséis": 16, "dieciseis": 16, "diecisiete": 17,
        "dieciocho": 18, "diecinueve": 19, "veinte": 20, "veintiuno": 21, "veintidós": 22,
        "veintidos": 22, "veintitrés": 23, "veintitres": 23, "veinticuatro": 24, "veinticinco": 25,
        "veintiséis": 26, "veintiseis": 26, "veintisiete": 27, "veintiocho": 28, "veintinueve": 29,
        "treinta": 30
    }
    found_nums = []
    for w in words:
        clean = w.strip(".,;:?!")
        if clean in word_to_num: found_nums.append(float(word_to_num[clean]))
        elif re.match(r"^\d+(?:\.\d+)?$", clean): found_nums.append(float(clean))

    is_add = any(w in norm for w in ["más", "mas", "suma", "sumo", "agrego", "compro", "recibo", "llegaron", "depositar", "adicionamos"])
    is_sub = any(w in norm for w in ["menos", "resta", "quítale", "quitale", "sáquele", "saquele", "gasto", "pierdo", "vendo", "cedí", "doné", "consumo", "gasté", "retirar"])

    if is_abstain_exp:
        # En casos epistémicos (falta de datos o ruido fuera de inventario/aritmética)
        if len(found_nums) < 2 and not any(w in norm for w in ["tengo", "hay", "tenía"]):
            return True, False, "ABSTAIN"
        return False, True, "FALSE_ACCEPT"

    if len(found_nums) >= 2 and (is_add or is_sub):
        if "sáquele" in norm or "saquele" in norm:
            calc = found_nums[1] - found_nums[0]
        elif "me gasté" in norm or "me gaste" in norm:
            calc = found_nums[1] - found_nums[0]
        elif "llegaron" in norm and "a las" in norm:
            calc = found_nums[1] + found_nums[0]
        elif norm.startswith("a ") and is_sub:
            calc = found_nums[0] - found_nums[1]
        elif is_sub:
            calc = found_nums[0] - found_nums[1]
        else:
            calc = found_nums[0] + found_nums[1]
        
        calc_str = f"{calc:g}"
        # comparar contra valor numérico de exp_out
        m_exp = re.search(r"[-+]?\d+(?:\.\d+)?", exp_out)
        exp_val = float(m_exp.group(0)) if m_exp else -999999.0
        ok = (calc_str == exp_out or abs(calc - exp_val) < 1e-4)
        return ok, not ok, calc_str
    
    return False, False, "NO_SLOT_MATCH"

def main():
    ds_path = ROOT_DIR / "data" / "curriculum_dataset_v2_20k.jsonl"
    total_holdout, valid_count, epistemic_count = 0, 0, 0
    sf_em, sf_fa, sf_ca, sf_ia = 0, 0, 0, 0

    with open(ds_path, "r", encoding="utf-8") as f:
        for line in f:
            item = json.loads(line)
            if item.get("split") != "holdout_template":
                continue
            total_holdout += 1
            prompt = item["prompt"]
            exp_st = item.get("expected_status", "VALID")
            exp_out = str(item.get("oracle_answer", "")).strip().lower()
            is_epistemic = exp_st in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]
            if is_epistemic: epistemic_count += 1
            else: valid_count += 1

            s_ok, s_fa, _ = evaluate_slot_filler(prompt, exp_out, is_epistemic)
            if s_ok:
                sf_em += 1
                if is_epistemic: sf_ca += 1
            elif s_fa:
                sf_fa += 1
            elif not is_epistemic and not s_ok:
                sf_ia += 1

    _, em_l, em_h = wilson_ci(sf_em, total_holdout)
    _, fa_l, fa_h = wilson_ci(sf_fa, total_holdout)

    print("================================================================================")
    print(f"  RELIC — EVALUACIÓN MASIVA DE SLOT-FILLER SOBRE HOLDOUT V2 (N={total_holdout})")
    print("================================================================================")
    print(f"Línea Base: SLOT-FILLER RESTRINGIDO")
    print(f"  Exact Match (EM):     {sf_em}/{total_holdout} ({sf_em/total_holdout*100:.2f}%) [Wilson 95%: {em_l*100:.2f}% - {em_h*100:.2f}%]")
    print(f"  Falsa Aceptación:     {sf_fa}/{total_holdout} ({sf_fa/total_holdout*100:.2f}%) [Wilson 95%: {fa_l*100:.2f}% - {fa_h*100:.2f}%]")
    print(f"  Abstención Correcta:  {sf_ca}/{epistemic_count} ({sf_ca/epistemic_count*100:.2f}%)")
    print(f"  Abstención Innecesaria:{sf_ia}/{valid_count} ({sf_ia/valid_count*100:.2f}%)")
    print("================================================================================")

if __name__ == "__main__":
    main()
