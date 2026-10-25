#!/usr/bin/env python3
"""
tools/eval/eval_holdout_75_per_case.py
Evalúa caso por caso los 75 ejemplos de holdout_templates_v1.json comparando:
1. Reglas simbólicas (relic_cli)
2. Slot-filler gramatical restringido (relic_cli dispatch/inverted)
3. Modelo neuronal causal libre (relic_10m_trained)
Guarda la traza completa por caso en reports/eval/holdout_75_per_case.json.
Restricción: <= 180 líneas.
"""
import json, math, os, re, subprocess, sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR / "tools"))
from run_model_inference import load_relic_model, RelicByteTokenizer, generate_rir
from evaluate_harness import wilson_ci, mcnemar_test

EXE_PATH = ROOT_DIR / "target" / "debug" / "relic_cli.exe"

def run_cli_query(text: str) -> dict:
    try:
        cmd = [str(EXE_PATH), "query", text]
        p = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=5)
        out = p.stdout
        st = re.search(r"\[2\.\s+ESTADO\]:\s+([A-Z_]+)\s+\(([^)]+)\)", out)
        status = st.group(1) if st else "UNKNOWN"
        route = st.group(2) if st else "unknown"
        ans = out.split("[3. RESPUESTA VERIFICADA]:")[1].split("---")[0].strip() if "[3. RESPUESTA VERIFICADA]:" in out else ""
        return {"status": status, "route": route, "ans": ans, "raw": out}
    except Exception as e:
        return {"status": "ERROR", "route": "error", "ans": str(e), "raw": ""}

def evaluate_slot_filler(text: str, exp_out: str, is_abstain_exp: bool) -> tuple[bool, bool, str]:
    # Slot-filler restringido canónico de números y operadores de inventario
    norm = text.lower().strip()
    words = norm.split()
    nums = re.findall(r"\b\d+(?:\.\d+)?\b", norm)
    # Detección de números en palabras comunes
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
        # En casos epistémicos (sin números suficientes o intención no aritmética), el slot-filler abstiene
        if len(found_nums) < 2 and not any(w in norm for w in ["tengo", "hay"]):
            return True, False, "ABSTAIN" # acierto
        return False, True, "FALSE_ACCEPT" # falsa aceptación

    if len(found_nums) >= 2 and (is_add or is_sub):
        # En sintaxis "sáquele A a B" o "a A quítale B":
        if "sáquele" in norm or "saquele" in norm:
            calc = found_nums[1] - found_nums[0]
        elif norm.startswith("a ") and is_sub:
            calc = found_nums[0] - found_nums[1]
        elif is_sub:
            calc = found_nums[0] - found_nums[1]
        else:
            calc = found_nums[0] + found_nums[1]
        calc_str = f"{calc:g}"
        ok = (calc_str == exp_out or abs(calc - float(exp_out or -9999)) < 1e-4)
        return ok, not ok, calc_str
    return False, False, "NO_SLOT_MATCH"

def main():
    suite_path = ROOT_DIR / "tests" / "holdout_templates_v1.json"
    weights_path = ROOT_DIR / "artifacts" / "weights" / "relic_10m_trained.safetensors"
    cases = json.load(open(suite_path, encoding="utf-8"))

    print(f"Cargando modelo neuronal desde {weights_path}...")
    model = load_relic_model(str(weights_path))
    tok = RelicByteTokenizer()

    per_case = []
    # Contadores
    em_r, fa_r = 0, 0
    em_s, fa_s = 0, 0
    em_m, fa_m = 0, 0

    # McNemar tablas: b = base acierta y modelo falla; c = base falla y modelo acierta
    b_mr, c_mr = 0, 0
    b_ms, c_ms = 0, 0

    print(f"Evaluando {len(cases)} casos de holdout...")
    for idx, c in enumerate(cases):
        cid = c["id"]
        inp = c["input"]
        exp_st = c["expected_status"]
        exp_out = str(c.get("expected_output", "")).strip().lower()
        is_abst = exp_st in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]

        # 1. Reglas CLI
        cli_res = run_cli_query(inp)
        cli_abst = cli_res["status"] in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]
        cli_match = exp_out in cli_res["ans"].lower() if exp_out else True
        if is_abst and cli_abst: r_ok, r_fa = True, False
        elif not is_abst and not cli_abst and cli_match: r_ok, r_fa = True, False
        elif is_abst and not cli_abst: r_ok, r_fa = False, True
        else: r_ok, r_fa = False, False
        if r_ok: em_r += 1
        if r_fa: fa_r += 1

        # 2. Slot-Filler Restringido
        s_ok, s_fa, s_val = evaluate_slot_filler(inp, exp_out, is_abst)
        if s_ok: em_s += 1
        if s_fa: fa_s += 1

        # 3. Modelo 10M libre
        raw_m = generate_rir(model, tok, inp, max_new_tokens=40)
        # Parseo JSON con métrica estricta:
        # Falsa aceptación = salida con RIR válido y distinto del gold.
        # Salida inválida (vacía o no parseable) = "rechazo", no falsa aceptación.
        m_ok, m_fa, m_rej = False, False, False
        try:
            p_json = json.loads(raw_m)
            m_st = p_json.get("status", "VALID")
            m_abst = m_st in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]
            m_match = exp_out in raw_m.lower() if exp_out else True
            if is_abst and m_abst:
                m_ok, m_fa = True, False
            elif not is_abst and not m_abst and m_match:
                m_ok, m_fa = True, False
            elif is_abst and not m_abst:
                m_ok, m_fa = False, True
            else:
                m_ok, m_fa = False, True
        except Exception:
            # Salida no parseable o malformada = Rechazo sintáctico
            m_ok, m_fa, m_rej = False, False, True
        if m_ok: em_m += 1
        if m_fa: fa_m += 1

        # Concordancias McNemar
        # b = baseline acierta (ok) y modelo falla (not ok)
        # c = baseline falla (not ok) y modelo acierta (ok)
        if r_ok and not m_ok: b_mr += 1
        elif not r_ok and m_ok: c_mr += 1

        if s_ok and not m_ok: b_ms += 1
        elif not s_ok and m_ok: c_ms += 1

        per_case.append({
            "id": cid, "input": inp, "expected_status": exp_st, "expected_output": exp_out,
            "rules": {"ok": r_ok, "fa": r_fa, "pred": cli_res["ans"], "status": cli_res["status"]},
            "slot_filler": {"ok": s_ok, "fa": s_fa, "pred": s_val},
            "model": {"ok": m_ok, "fa": m_fa, "syntax_rejected": m_rej, "raw_output": raw_m}
        })

    out_file = ROOT_DIR / "reports" / "eval" / "holdout_75_per_case.json"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(per_case, f, indent=2, ensure_ascii=False)

    chi2_mr, p_mr = mcnemar_test(b_mr, c_mr)
    chi2_ms, p_ms = mcnemar_test(b_ms, c_ms)

    print("\n" + "="*70)
    print("  RESULTADOS PER-CASO CONSOLIDADOS (N=75 HOLDOUT)")
    print("="*70)
    print(f"Reglas:       EM={em_r}/75 ({em_r/75*100:.1f}%), FA={fa_r}/75 ({fa_r/75*100:.1f}%)")
    print(f"Slot-Filler:  EM={em_s}/75 ({em_s/75*100:.1f}%), FA={fa_s}/75 ({fa_s/75*100:.1f}%)")
    print(f"Modelo 10M:   EM={em_m}/75 ({em_m/75*100:.1f}%), FA={fa_m}/75 ({fa_m/75*100:.1f}%)")
    print(f"\nMcNemar (Modelo vs Reglas):      b={b_mr}, c={c_mr} -> Chi2={chi2_mr:.4f}, p={p_mr:.6e}")
    print(f"McNemar (Modelo vs Slot-Filler): b={b_ms}, c={c_ms} -> Chi2={chi2_ms:.4f}, p={p_ms:.6e}")
    print(f"[OK] Reporte por caso guardado en: {out_file}")

if __name__ == "__main__":
    main()
