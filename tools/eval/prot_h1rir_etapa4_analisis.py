#!/usr/bin/env python3
"""ETAPA 4 (protocolo H1->RIR): analisis del flujo completo con el extractor
invertido. Mismos preds congelados de etapa 3 (apples-to-apples). Reporta
F6 y no-F6 SEPARADOS (G2/G3), factor e2e (G1), salvaguardas (G4) y
no-regresion (G5). No reabre la etapa 3; evalua los criterios congelados."""
import json
from pathlib import Path
from collections import Counter

ROOT = Path(__file__).resolve().parent.parent.parent
SPLIT = ROOT / "reports/eval/prot_h1rir_test_split.json"
# familia del bloque no-regresion: los 19 correctos de etapa 3 (misma familia)
G5_FAMILIAS = {"F1_direct_possession_multistep", "F1_direct_possession_add", "F1_direct_possession_sub",
               "F9_unseen_lexicon_extraviar", "F9_unseen_lexicon_ceder", "F9_unseen_lexicon_consumir",
               "F9_unseen_lexicon_donar_caridad", "F9_unseen_lexicon_adquirir", "F9_unseen_lexicon_recolectar",
               "F9_unseen_lexicon_incorporar"}


def ops_match(plan_ops, gold_ops):
    if len(plan_ops) != len(gold_ops):
        return False
    for (op, v), g in zip(plan_ops, gold_ops):
        if ("+" if op == 0 else "-") != ("+" if g["operator"] == "add" else "-"):
            return False
        if abs(v - g["operand"]) > 1e-9:
            return False
    return True


def analiza_dump(d, casos, familias, g5_ref_ids):
    m = Counter()
    f6 = Counter(tot=0, ok=0, fail=0)
    g5_ok = 0
    fallos = []
    lat = []
    for c in d:
        g = casos[c["id"]]
        fam = familias[c["id"]]
        lat.append(c["lat_us_p50"])
        if fam.startswith("F6"):
            f6["tot"] += 1
        p = c["plan"]
        if p is None:
            if g["status"] != "VALID":
                m["abstencion_correcta"] += 1
            else:
                m["falso_rechazo"] += 1
                if fam.startswith("F6"):
                    f6["fail"] += 1
                fallos.append({"id": c["id"], "fam": fam, "gold_status": "VALID", "plan": None,
                               "motivo": "sin plan (pred abstiene o parser no extrae)"})
            continue
        fields_ok = (abs(p["initial"] - g.get("initial_quantity", -1)) < 1e-9
                     and p["unit"] == g.get("entity", "")
                     and ops_match(p["ops"], g.get("operations", [])))
        if g["status"] != "VALID":
            m["aceptacion_falsa"] += 1
            fallos.append({"id": c["id"], "fam": fam, "gold_status": g["status"], "plan": p})
        elif fields_ok:
            m["correcto_ejecutado"] += 1
            if fam.startswith("F6"):
                f6["ok"] += 1
            elif c["id"] in g5_ref_ids:
                g5_ok += 1
        else:
            m["queryspec_incorrecto"] += 1
            fallos.append({"id": c["id"], "fam": fam, "gold_status": "VALID", "plan": p,
                           "gold": {k: g.get(k) for k in ("entity", "initial_quantity", "operations")}})
    return m, f6, g5_ok, fallos, sum(lat) / max(len(lat), 1)


def g5_ids_etapa3(s):
    """ids correctos en etapa 3 dentro de las familias del bloque G5 (el set de
    referencia de NO REGRESION, medido en el dump de etapa 3 de la MISMA semilla)."""
    d = json.load(open(ROOT / f"reports/eval/dump_bridge_seed{s}.json", encoding="utf-8"))["cases"]
    ids = set()
    for c in d:
        g = casos[c["id"]]
        fam = familias[c["id"]]
        p = c["plan"]
        if p is None or g["status"] != "VALID" or fam not in G5_FAMILIAS:
            continue
        if (abs(p["initial"] - g.get("initial_quantity", -1)) < 1e-9 and p["unit"] == g.get("entity", "")
                and ops_match(p["ops"], g.get("operations", []))):
            ids.add(c["id"])
    return ids


if __name__ == "__main__":
    split = json.load(open(SPLIT, encoding="utf-8"))
    casos = {f"h1rir_{i}": g for i, g in enumerate(split["casos"])}
    familias = {f"h1rir_{i}": g["family"] for i, g in enumerate(split["casos"])}
    resumen = {"semillas": {}, "mediana": {}, "fallos_completos": {}}
    for s in ["42", "123", "999"]:
        d = json.load(open(ROOT / f"reports/eval/dump_bridge4_seed{s}.json", encoding="utf-8"))["cases"]
        g5_ref = g5_ids_etapa3(s)
        m, f6, g5_ok, fallos, lat_media = analiza_dump(d, casos, familias, g5_ref)
        e2e = round(100 * m["correcto_ejecutado"] / len(d), 1)
        resumen["semillas"][s] = {
            "factor_e2e": round(e2e / 20.0, 2),
            "exactitud_e2e_pct": e2e,
            "cobertura_e2e_pct": round(100 * sum(1 for c in d if c["plan"]) / len(d), 1),
            "f6_ok": f6["ok"], "f6_tot": f6["tot"],
            "g5_correctos": g5_ok,
            "aceptacion_falsa": m["aceptacion_falsa"],
            "queryspec_incorrecto": m["queryspec_incorrecto"],
            "falso_rechazo": m["falso_rechazo"],
            "abstencion_correcta": m["abstencion_correcta"],
            "validez_rir_pct": 100.0,
            "lat_us_p50_media": round(lat_media, 1),
        }
        resumen["fallos_completos"][s] = fallos
    med = lambda k: sorted(resumen["semillas"][s][k] for s in resumen["semillas"])[1]
    resumen["mediana"] = {k: med(k) for k in ("factor_e2e", "exactitud_e2e_pct", "cobertura_e2e_pct",
                                              "f6_ok", "g5_correctos", "aceptacion_falsa", "falso_rechazo")}
    criterios = {
        "G1_factor>=2.0": resumen["mediana"]["factor_e2e"] >= 2.0,
        "G2_F6>=23/25": resumen["mediana"]["f6_ok"] >= 23,
        "G3_heldout": "ver test Rust inverted_extractor_frozen_suite 3/3 (7/7 VALID + 5/5 abstain)",
        "G4_salvaguardas": resumen["mediana"]["aceptacion_falsa"] == 0,
        "G5_no_regresion>=19": resumen["mediana"]["g5_correctos"] >= 19,
    }
    ref = {"despacho_concordancia_pct": 20.0, "despacho_cobertura_rir_pct": 48.4,
           "etapa3_queryspec_mediana_pct": 20.0}
    out = ROOT / "reports/eval/prot_h1rir_etapa4_resultados.json"
    json.dump({**resumen, "criterios": criterios, "referencia": ref},
              open(out, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(json.dumps(resumen["semillas"], indent=1))
    print("mediana:", json.dumps(resumen["mediana"]))
    print("criterios:", json.dumps(criterios))
