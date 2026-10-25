#!/usr/bin/env python3
"""
tools/measure_baseline.py
Medición formal de línea base del analizador de reglas de relic_cli.
Calcula: RIR exacto, falsa aceptación, abstención correcta/innecesaria e intervalos Wilson (95%).
Menos de 180 líneas.
"""
import json, math, os, re, subprocess, sys
from typing import Dict, Any, List, Tuple

EXE_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "target", "debug", "relic_cli.exe"))

def wilson_interval(successes: int, total: int, z: float = 1.96) -> Tuple[float, float, float]:
    if total == 0: return 0.0, 0.0, 0.0
    p = successes / total
    denom = 1.0 + (z**2) / total
    center = (p + (z**2) / (2.0 * total)) / denom
    spread = (z * math.sqrt((p * (1.0 - p) / total) + (z**2) / (4.0 * (total**2)))) / denom
    return p, max(0.0, center - spread), min(1.0, center + spread)

def run_query(query: str) -> Dict[str, str]:
    try:
        proc = subprocess.run([EXE_PATH, "query", query], capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=5)
        out = proc.stdout
        status_match = re.search(r"\[2\.\s+ESTADO\]:\s+([A-Z_]+)\s+\(([^)]+)\)", out)
        status = status_match.group(1) if status_match else "UNKNOWN"
        route = status_match.group(2) if status_match else "unknown"
        ans_part = out.split("[3. RESPUESTA VERIFICADA]:")[1].split("---")[0].strip() if "[3. RESPUESTA VERIFICADA]:" in out else ""
        return {"status": status, "route": route, "text": ans_part}
    except Exception as e:
        return {"status": "ERROR", "route": "error", "text": str(e)}

def evaluate_suite(name: str, cases: List[Dict[str, Any]]) -> Dict[str, Any]:
    total = len(cases)
    if total == 0 or all(not c.get("input") for c in cases):
        return {"name": name, "status": "PENDIENTE_DE_DATOS", "total": total}
    exact_acc, false_acc, correct_abstain, unnec_abstain = 0, 0, 0, 0
    for c in cases:
        inp = (c.get("input") or c.get("text") or "").strip()
        if not inp: continue
        res = run_query(inp)
        exp_status = c.get("expected_status") or c.get("expect") or ("INCOMPLETE" if "epistemic" in c.get("expected_route", "") else "VALID")
        exp_route = c.get("expected_route", "")
        exp_out = str(c.get("expected_output") or c.get("expected_result") or "").lower()
        is_abstain_expected = exp_status in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]
        is_abstain_pred = res["status"] in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]
        matched_content = exp_out in res["text"].lower() if exp_out else True
        if is_abstain_expected and is_abstain_pred:
            correct_abstain += 1
            exact_acc += 1
        elif not is_abstain_expected and is_abstain_pred:
            unnec_abstain += 1
        elif not is_abstain_expected and not is_abstain_pred:
            if matched_content and (not exp_route or exp_route == res["route"]):
                exact_acc += 1
            else:
                false_acc += 1
        elif is_abstain_expected and not is_abstain_pred:
            false_acc += 1
    p_acc, acc_l, acc_u = wilson_interval(exact_acc, total)
    p_fa, fa_l, fa_u = wilson_interval(false_acc, total)
    p_ca, ca_l, ca_u = wilson_interval(correct_abstain, total)
    p_ua, ua_l, ua_u = wilson_interval(unnec_abstain, total)
    return {
        "name": name, "status": "EVALUADO", "total": total,
        "exact_acc": {"count": exact_acc, "rate": p_acc, "wilson_ci": (acc_l, acc_u)},
        "false_acceptance": {"count": false_acc, "rate": p_fa, "wilson_ci": (fa_l, fa_u)},
        "correct_abstention": {"count": correct_abstain, "rate": p_ca, "wilson_ci": (ca_l, ca_u)},
        "unnecessary_abstention": {"count": unnec_abstain, "rate": p_ua, "wilson_ci": (ua_l, ua_u)}
    }

def main():
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "tests"))
    suites_files = [
        ("unseen_frozen_v2", os.path.join(base_dir, "unseen_frozen_v2.json"), "cases"),
        ("known_failures_v2", os.path.join(base_dir, "known_failures_v2.json"), None),
        ("f6_frozen_25", os.path.join(base_dir, "f6_inverted_oro_v1.json"), "f6_frozen"),
        ("dev_suite_v1", os.path.join(base_dir, "dev_suite_v1.json"), None),
        ("external_human_gold_v1", os.path.join(base_dir, "external_human_gold_v1.json"), "cases")
    ]
    report = {}
    print("================================================================================")
    print("  RELIC — MEDICIÓN FORMAL DE LÍNEA BASE (ANALIZADOR DE REGLAS relic_cli)")
    print("================================================================================")
    for name, path, key in suites_files:
        if not os.path.exists(path): continue
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
            cases = data[key] if key and isinstance(data, dict) and key in data else (data if isinstance(data, list) else [])
        res = evaluate_suite(name, cases)
        report[name] = res
        print(f"\nSuite: {name} (Total: {res['total']}) -> Estado: {res['status']}")
        if res["status"] == "EVALUADO":
            acc = res["exact_acc"]
            fa = res["false_acceptance"]
            print(f"  Exactitud Exacta:       {acc['count']}/{res['total']} ({acc['rate']*100:.1f}%) | Wilson 95%: [{acc['wilson_ci'][0]*100:.1f}%, {acc['wilson_ci'][1]*100:.1f}%]")
            print(f"  Falsa Aceptación (Err): {fa['count']}/{res['total']} ({fa['rate']*100:.1f}%) | Wilson 95%: [{fa['wilson_ci'][0]*100:.1f}%, {fa['wilson_ci'][1]*100:.1f}%]")
            print(f"  Abstención Correcta:    {res['correct_abstention']['count']}/{res['total']} | Innecesaria: {res['unnecessary_abstention']['count']}/{res['total']}")
        else:
            print("  [AVISO]: Suite no rellenada aún por evaluadores externos humanos. Marcada PENDIENTE.")
    out_json = os.path.join(os.path.dirname(__file__), "..", "reports", "baseline_rules_report.json")
    os.makedirs(os.path.dirname(out_json), exist_ok=True)
    with open(out_json, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
    print(f"\n[OK] Informe de baseline consolidado guardado en: {out_json}")

if __name__ == "__main__":
    main()
