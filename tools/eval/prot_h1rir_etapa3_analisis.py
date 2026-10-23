#!/usr/bin/env python3
"""ETAPA 3 (protocolo H1->RIR): analisis del flujo completo del puente A2.

Definiciones congeladas de esta etapa (documentadas ANTES de mirar resultados):
- cobertura: casos con plan ejecutado (QuerySpec generado y validado por el engine).
- exactitud_queryspec (entre ejecutados): plan.initial == oro.initial_quantity,
  plan.unit == oro.entity, ops coinciden en longitud y signos en orden.
- validez_rir: entre los ejecutados, fraccion con status VALID del engine.
- abstencion_correcta: pred status != VALID y oro.status != VALID.
- aceptacion_falsa: pred VALID y oro != VALID y se ejecuto algo (estructura inventada
  o usada con oro invalido). falso_negativo: pred abstencion y oro VALID.
- todos los fallos se conservan completos (lista casos, sin muestreo).
Los "despacho" de referencia provienen de prot_h1rir_resultados.json [MEDIDO].
Latencia: mediana end-to-end por caso (mean de p50 entre casos).
"""
import json
from pathlib import Path
from collections import Counter

ROOT = Path(__file__).resolve().parent.parent.parent
SPLIT = ROOT / "reports/eval/prot_h1rir_test_split.json"


def ops_match(plan_ops, gold_ops):
    if len(plan_ops) != len(gold_ops):
        return False
    for (op, v), g in zip(plan_ops, gold_ops):
        signo = "+" if op == 0 else "-"
        gs = "+" if g["operator"] == "add" else "-"
        if signo != gs or abs(v - g["operand"]) > 1e-9:
            return False
    return True


def analiza_semillas():
    gold = json.load(open(SPLIT, encoding="utf-8"))["casos"]
    idos = {}
    por_seed = {}
    for i, g in enumerate(gold):
        idos[f"h1rir_{i}"] = g
    resumen = {"defs": "ver docstring", "semillas": {}, "fallos_completos": {}}
    for s in ["42", "123", "999"]:
        d = json.load(open(ROOT / f"reports/eval/dump_bridge_seed{s}.json", encoding="utf-8"))["cases"]
        m = Counter()
        fallos = []
        lat = []
        for c in d:
            g = idos[c["id"]]
            pred_abst = c["plan"] is None
            p = c["plan"]
            lat.append(c["lat_us_p50"])
            if p is not None:
                fields_ok = (abs(p["initial"] - g.get("initial_quantity", -1)) < 1e-9
                             and p["unit"] == g.get("entity", "")
                             and ops_match(p["ops"], g.get("operations", [])))
                if g["status"] != "VALID":
                    m["aceptacion_falsa"] += 1
                    fallos.append({"id": c["id"], "gold_status": g["status"], "ejecutado": True, "plan": p})
                elif fields_ok:
                    m["correcto_ejecutado"] += 1
                else:
                    m["queryspec_incorrecto"] += 1
                    fallos.append({"id": c["id"], "gold_status": "VALID", "plan": p,
                                   "gold": {k: g.get(k) for k in ("entity", "initial_quantity", "operations")},
                                   "unidad_ok": p["unit"] == g.get("entity", ""),
                                   "inicial_ok": abs(p["initial"] - g.get("initial_quantity", -1)) < 1e-9,
                                   "ops_ok": ops_match(p["ops"], g.get("operations", []))})
            else:
                if g["status"] != "VALID":
                    m["abstencion_correcta"] += 1
                else:
                    m["falso_rechazo"] += 1
                    fallos.append({"id": c["id"], "gold_status": "VALID", "ejecutado": False,
                                   "motivo": "sin plan extraible por el parseador existente",
                                   "pred_status": c["pred"].split("|")[1]})
        ejec = sum(m[k] for k in ("correcto_ejecutado", "queryspec_incorrecto", "aceptacion_falsa"))
        por_seed[s] = {
            "cobertura": round(ejec / len(d), 4),
            "exactitud_queryspec_pct": round(100 * m["correcto_ejecutado"] / len(d), 1),
            "validez_rir_pct": round(100 * m["correcto_ejecutado"] / ejec, 1) if ejec else 0.0,
            "abstencion_correcta": m["abstencion_correcta"],
            "aceptacion_falsa": m["aceptacion_falsa"],
            "falso_rechazo": m["falso_rechazo"],
            "queryspec_incorrecto": m["queryspec_incorrecto"],
            "lat_us_p50_media": round(sum(lat) / len(lat), 1),
        }
        resumen["fallos_completos"][s] = fallos
    med = lambda k: sorted(por_seed[s][k] for s in por_seed)[1]
    resumen["semillas"] = por_seed
    resumen["mediana_3_semillas"] = {"cobertura": med("cobertura"),
                                     "exactitud_queryspec_pct": med("exactitud_queryspec_pct"),
                                     "abstencion_correcta": med("abstencion_correcta"),
                                     "aceptacion_falsa": med("aceptacion_falsa")}
    ref = json.load(open(ROOT / "reports/eval/prot_h1rir_resultados.json", encoding="utf-8"))
    rows = ref.get("despacho_rust", {})
    resumen["referencia_despacho_rust"] = {"cobertura_rir_pct": rows.get("cobertura_rir_pct"),
                                           "concordancia_intencion_pct": rows.get("concordancia_intencion_pct"),
                                           "n": rows.get("n")}
    resumen["referencia_protocolo"] = {"tupla_exacta_pct_despacho": ref.get("tupla_exacta_pct"),
                                       "baseline_charbigram_acc": ref.get("baseline_charbigram_acc")}
    return resumen


if __name__ == "__main__":
    r = analiza_semillas()
    out = ROOT / "reports/eval/prot_h1rir_etapa3_resultados.json"
    json.dump(r, open(out, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(json.dumps(r["semillas"], indent=1))
    print("mediana:", json.dumps(r["mediana_3_semillas"]))
    print("ref despacho:", json.dumps(r["referencia_despacho_rust"]))
    print("escrito:", out)
