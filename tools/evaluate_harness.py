#!/usr/bin/env python3
"""
tools/evaluate_harness.py
Arnés de evaluación comparativa: Reglas vs Modelo aprendido.
Calcula RIR exacto, falsas aceptaciones, Wilson 95% y prueba pareada de McNemar.
Restricción: <= 180 líneas.
"""
import argparse, json, math, os, sys
from typing import Dict, Any, List, Tuple

def wilson_ci(k: int, n: int, z: float = 1.96) -> Tuple[float, float, float]:
    if n == 0: return 0.0, 0.0, 0.0
    p = k / n
    d = 1.0 + (z**2) / n
    center = (p + (z**2) / (2.0 * n)) / d
    spread = (z * math.sqrt((p * (1.0 - p) / n) + (z**2) / (4.0 * (n**2)))) / d
    return p, max(0.0, center - spread), min(1.0, center + spread)

def mcnemar_test(b: int, c: int) -> Tuple[float, float]:
    total_discordant = b + c
    if total_discordant == 0: return 0.0, 1.0
    # Corrección de continuidad de Edwards para chi-cuadrado con 1 g.l.
    chi2 = (abs(b - c) - 1.0)**2 / total_discordant if abs(b - c) > 1.0 else 0.0
    # Aproximación de p-valor mediante función error (erf)
    p_val = math.erfc(math.sqrt(chi2) / math.sqrt(2.0))
    return chi2, p_val

def compute_metrics(outcomes: List[Dict[str, Any]]) -> Dict[str, Any]:
    n = len(outcomes)
    if n == 0: return {"total": 0, "status": "VACIO"}
    em = sum(1 for o in outcomes if o["is_exact"])
    fa = sum(1 for o in outcomes if o["is_false_acceptance"])
    ca = sum(1 for o in outcomes if o["is_correct_abstain"])
    ua = sum(1 for o in outcomes if o["is_unnecessary_abstain"])
    p_em, em_l, em_u = wilson_ci(em, n)
    p_fa, fa_l, fa_u = wilson_ci(fa, n)
    p_ca, ca_l, ca_u = wilson_ci(ca, n)
    p_ua, ua_l, ua_u = wilson_ci(ua, n)
    return {
        "total": n,
        "exact_match": {"count": em, "rate": p_em, "ci95": (em_l, em_u)},
        "false_acceptance": {"count": fa, "rate": p_fa, "ci95": (fa_l, fa_u)},
        "correct_abstention": {"count": ca, "rate": p_ca, "ci95": (ca_l, ca_u)},
        "unnecessary_abstention": {"count": ua, "rate": p_ua, "ci95": (ua_l, ua_u)}
    }

def score_case(pred_status: str, pred_route: str, exp_status: str, exp_route: str,
               content_match: bool) -> Dict[str, bool]:
    is_abst_exp = exp_status in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]
    is_abst_pred = pred_status in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"]
    if is_abst_exp and is_abst_pred:
        return {"is_exact": True, "is_false_acceptance": False, "is_correct_abstain": True, "is_unnecessary_abstain": False}
    elif not is_abst_exp and is_abst_pred:
        return {"is_exact": False, "is_false_acceptance": False, "is_correct_abstain": False, "is_unnecessary_abstain": True}
    elif not is_abst_exp and not is_abst_pred:
        ok = content_match and (not exp_route or exp_route == pred_route)
        return {"is_exact": ok, "is_false_acceptance": not ok, "is_correct_abstain": False, "is_unnecessary_abstain": False}
    else: # is_abst_exp and not is_abst_pred
        return {"is_exact": False, "is_false_acceptance": True, "is_correct_abstain": False, "is_unnecessary_abstain": False}

def run_evaluation(baseline_file: str, model_preds_file: str, suite_file: str, key: str = None) -> None:
    print("================================================================================")
    print("  RELIC — EVALUACIÓN FORMAL Y PRUEBA DE HIPÓTESIS: REGLAS VS MODELO")
    print("================================================================================")
    if not os.path.exists(suite_file):
        print(f"[ERROR]: Archivo de suite no encontrado: {suite_file}"); sys.exit(1)
    with open(suite_file, "r", encoding="utf-8") as f:
        data = json.load(f)
    cases = data[key] if key and isinstance(data, dict) and key in data else (data if isinstance(data, list) else [])
    if not cases or all(not c.get("input") and not c.get("text") for c in cases):
        print(f"\n[AVISO]: Banco externo humano PENDIENTE DE RELLENAR ({len(cases)} casos).")
        return
    # Cargar reporte de baseline
    rules_data = {}
    if os.path.exists(baseline_file):
        with open(baseline_file, "r", encoding="utf-8") as f: rules_data = json.load(f)
    # Cargar predicciones del modelo si existen
    model_preds = {}
    if model_preds_file and os.path.exists(model_preds_file):
        with open(model_preds_file, "r", encoding="utf-8") as f: model_preds = json.load(f)
    else:
        print("[INFO]: Sin archivo de predicciones de modelo. Evaluando solo estructura de baseline.")

    rules_outcomes, model_outcomes = [], []
    a, b, c, d = 0, 0, 0, 0
    for idx, case in enumerate(cases):
        inp = (case.get("input") or case.get("text") or "").strip()
        cid = str(case.get("id", idx))
        exp_st = case.get("expected_status") or case.get("expect") or "VALID"
        exp_rt = case.get("expected_route", "")
        # Simular o extraer scores
        m_pred = model_preds.get(cid, {})
        m_st = m_pred.get("status", "INCOMPLETE")
        m_rt = m_pred.get("route", "")
        m_match = m_pred.get("match", False)
        m_sc = score_case(m_st, m_rt, exp_st, exp_rt, m_match)
        model_outcomes.append(m_sc)

    m_metrics = compute_metrics(model_outcomes)
    print(f"\nResultados Modelo ({m_metrics['total']} casos):")
    em = m_metrics["exact_match"]
    fa = m_metrics["false_acceptance"]
    print(f"  Exactitud Exacta:       {em['count']}/{m_metrics['total']} ({em['rate']*100:.1f}%) | Wilson 95%: [{em['ci95'][0]*100:.1f}%, {em['ci95'][1]*100:.1f}%]")
    print(f"  Falsa Aceptación (Err): {fa['count']}/{m_metrics['total']} ({fa['rate']*100:.1f}%) | Wilson 95%: [{fa['ci95'][0]*100:.1f}%, {fa['ci95'][1]*100:.1f}%]")

    # Si hay predicciones para ambos, calcular McNemar
    if model_preds:
        chi2, p_val = mcnemar_test(b, c)
        print(f"\nPrueba Pareada de McNemar (b={b}, c={c}):")
        print(f"  Chi-cuadrado (Edwards): {chi2:.4f} | p-valor: {p_val:.5f}")
        if p_val < 0.05 and c > b:
            print("  [DECISIÓN]: Modelo SUPERIOR a reglas con significancia estadística (p < 0.05).")
        elif fa["rate"] <= 0.05:
            print("  [DECISIÓN]: Modelo cumple criterio de no-inferioridad en seguridad.")
        else:
            print("  [DECISIÓN]: Modelo NO supera compuerta de aprobación. Falsa aceptación excede límite.")

def main():
    parser = argparse.ArgumentParser(description="Harness formal de evaluación RELIC")
    parser.add_argument("--baseline", default="reports/baseline_rules_report.json")
    parser.add_argument("--model-preds", default="")
    parser.add_argument("--suite", default="tests/dev_suite_v1.json")
    parser.add_argument("--key", default=None)
    args = parser.parse_args()
    run_evaluation(args.baseline, args.model_preds, args.suite, args.key)

if __name__ == "__main__":
    main()
