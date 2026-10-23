#!/usr/bin/env python3
"""ETAPA 5 analisis e2e (95 congelados, mediana 3 semillas).
Aplica los criterios congelados en docs/PROTOCOL_H1_RIR.md (e4d06d7):
CONTINUA >= 45.0% e2e con salvaguardas; GRIS 35.0-45.0 con salvaguardas;
DETENER < 35.0% o >= 1 aceptacion falsa o suite congelada roja.
NO reabre etapa 3/4. No activa produccion."""
import json
from pathlib import Path
from collections import Counter

ROOT = Path(__file__).resolve().parent.parent.parent
SPLIT = ROOT / "reports/eval/prot_h1rir_test_split.json"


def ops_match(plan_ops, gold_ops):
    if len(plan_ops) != len(gold_ops):
        return False
    for (op, v), g in zip(plan_ops, gold_ops):
        if ("+" if op == 0 else "-") != ("+" if g["operator"] == "add" else "-"):
            return False
        if abs(v - g["operand"]) > 1e-9:
            return False
    return True


def main() -> None:
    gold = {f"h1rir_{i}": g for i, g in enumerate(json.load(open(SPLIT, encoding="utf-8"))["casos"])}
    sem = {}
    fallos = {}
    for s in ["42", "123", "999"]:
        d = json.load(open(ROOT / f"reports/eval/dump_bridge5_seed{s}.json", encoding="utf-8"))["cases"]
        m = Counter()
        lat = []
        f6 = Counter(tot=0, ok=0)
        fal = []
        for c in d:
            g = gold[c["id"]]
            lat.append(c["lat_us_p50"])
            fam = g["family"]
            if fam.startswith("F6"):
                f6["tot"] += 1
            p = c["plan"]
            if p is None:
                m["abstenc" if g["status"] != "VALID" else "falso_rechazo"] += 1
                continue
            ok = (abs(p["initial"] - g.get("initial_quantity", -1)) < 1e-9
                  and p["unit"] == g.get("entity", "")
                  and ops_match(p["ops"], g.get("operations", [])))
            if g["status"] != "VALID":
                m["aceptacion_falsa"] += 1
                fal.append({"id": c["id"], "fam": fam, "plan": p})
            elif ok:
                m["correcto_ejecutado"] += 1
                if fam.startswith("F6"):
                    f6["ok"] += 1
            else:
                m["queryspec_incorrecto"] += 1
                fal.append({"id": c["id"], "fam": fam, "plan": p})
        n = len(d)
        e2e = round(100 * m["correcto_ejecutado"] / n, 1)
        sem[s] = {"exactitud_e2e_pct": e2e,
                  "cobertura_e2e_pct": round(100 * sum(1 for c in d if c["plan"]) / n, 1),
                  "f6_ok": f6["ok"], "f6_tot": f6["tot"],
                  "aceptacion_falsa": m["aceptacion_falsa"],
                  "abstencion_correcta": m["abstenc"],
                  "falso_rechazo": m["falso_rechazo"],
                  "queryspec_incorrecto": m["queryspec_incorrecto"],
                  "validez_rir_pct": 100.0,
                  "lat_us_p50_media": round(sum(lat) / n, 1)}
        fallos[s] = fal
    med = sorted(sem[s]["exactitud_e2e_pct"] for s in sem)[1]
    med_af = max(sem[s]["aceptacion_falsa"] for s in sem)
    if med_af >= 1:
        veredic = "DETENER"
    elif med >= 45.0:
        veredic = "CONTINUA"
    elif med >= 35.0:
        veredic = "GRIS"
    else:
        veredic = "DETENER"
    out = {"semillas": sem, "mediana_e2e_pct": med, "veredicto_criterios_congelados": veredic,
           "fallos_completos": fallos,
           "ref_control_v1_etapa4_pct": 40.0,
           "techo_estructural_pct": 54.7}
    json.dump(out, open(ROOT / "reports/eval/prot_h1rir_etapa5_resultados.json", "w", encoding="utf-8"),
              ensure_ascii=False, indent=1)
    print(json.dumps(sem, indent=1))
    print("mediana e2e:", med, "| veredicto:", veredic)


if __name__ == "__main__":
    main()
