#!/usr/bin/env python3
"""Analisis de dumps crudos del harness eval_dump.rs (solo medicion, clasifica errores del
pipeline existente; no evalua funcionalidad nueva).

Taxonomia de error (contracto con docs/ESTADO_Y_ORIENTACION.md):
  (i)   no_parseo          : el pipeline no reconocio la frase (ruta unsupported/free_language)
  (ii)  rir_malo_aceptado  : parse interpreto MAL pero el RIR se valido y se ejecuto
  (iii) motor_fallo        : parse correcto (ruta/slots ok) pero la salida contradice el oro
  (iv)  abstencion_correcta: el oro pide no-ejecutar y el sistema abstiene
  (v)   abstencion_innecesaria: el oro pide ejecutar y el sistema abstiene

Uso: python tools/eval/analyze_dump.py <dump.json> <suite.json> [dump2 suite2 ...]
Salida: reports/eval/analisis_<stem>.json + resumen por consola.
"""
from __future__ import annotations

import json
import os
import sys

# [DESIGN] Estados/senales que cuentan como abstencion honesta del pipeline Rust.
ABSTAIN_STATUS = {"INVALID", "INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS"}
ABSTAIN_ROUTE = {"epistemic_rejection", "unsupported", "free_language", "degraded_mode"}


def es_abstencion(caso: dict) -> bool:
    rep = caso["repeats"][0]
    return rep["status"] in ABSTAIN_STATUS or rep["route"] in ABSTAIN_ROUTE


def clasificar_simple(caso: dict, oro: dict) -> str:
    rep = caso["repeats"][0]
    texto = rep["text"].lower()
    aguja = (oro.get("expected_output") or "").lower()
    ruta_gold = oro.get("expected_route")

    if ruta_gold in ("epistemic_rejection", "unsupported"):
        return "iv_abstencion_correcta" if es_abstencion(caso) else "fracaso_ruta"

    if rep["route"] == ruta_gold:
        if aguja and aguja in texto:
            return "correcto"
        if aguja:
            return "iii_salida_incorrecta_ruta_ok"
        return "correcto"  # sin aguja declarada, solo exigimos ruta

    if es_abstencion(caso):
        return "v_abstencion_innecesaria"
    if rep["route"] in ("unsupported", "free_language"):
        return "i_no_parseo"
    if rep["status"] == "VALID":
        return "ii_rir_malo_aceptado"
    return "iii_salida_incorrecta_ruta_ok"


def oro_semantico(caso: dict) -> dict | None:
    """Convierte labels de intent/slots del banco semantico a un oro comparable con el dump."""
    gi = caso.get("expected_intent")
    if not gi:
        return None
    mapa = {
        "reachability": "graph",
        "path_with_waypoint": "graph",
        "path_avoid_node": "graph",
        "incomplete": "incomplete",
        "rule_transformation": "rule_transformation",
        "inventory_word_problem": "arithmetic",
        "arithmetic": "arithmetic",
    }
    intent = mapa.get(gi, "free_language")
    slots = {
        "origin": (caso.get("expected_origin") or "").strip().lower() or None,
        "destination": (caso.get("expected_destination") or "").strip().lower() or None,
        "via": (caso.get("expected_via") or "").strip().lower() or None,
        "avoid": (caso.get("expected_avoid") or "").strip().lower() or None,
    }
    return {"intent": intent, "slots": slots, "negated": bool(caso.get("is_negated"))}


def slots_rust(spec: dict) -> dict:
    def val(x):
        if isinstance(x, dict):
            x = x.get("value")
        if x is None:
            return None
        return str(x).strip().lower()

    return {
        "origin": val(spec.get("initial")),
        "destination": val(spec.get("target")),
        "via": val(spec.get("via")),
        "avoid": val(spec.get("avoid")),
    }


def clasificar_semantico(caso: dict, gold: dict, oro_full: dict) -> str:
    rep = caso["repeats"][0]
    intent_rust = caso.get("intent") or rep.get("intent")
    spec = caso.get("spec") or {}
    sr = slots_rust(spec)
    golded = gold["slots"]
    esperado = (oro_full.get("expected_status") or "VALID").upper()

    # (i) el pipeline no expreso una representacion estructurada comparable
    if intent_rust not in ("graph", "arithmetic", "incomplete") and intent_rust != gold["intent"]:
        if es_abstencion(caso):
            return "v_abstencion_innecesaria" if gold["intent"] != "free_language" else "iv_abstencion_correcta"
        return "i_no_parseo"

    slots_ok = all(
        (g is None) or (r is not None and r == g) for g, r in ((golded[k], sr[k]) for k in golded)
    ) and (sr["origin"] is not None or sr["destination"] is not None or gold["intent"] == "incomplete")

    if intent_rust == gold["intent"]:
        if gold["intent"] == "incomplete":
            return "correcto" if es_abstencion(caso) else "iii_salida_incorrecta_ruta_ok"
        if gold["intent"] == "rule_transformation":
            return "iv_abstencion_correcta" if es_abstencion(caso) else "i_no_parseo"
        if gold["intent"] == "graph":
            if slots_ok:
                return "correcto"
            return "ii_rir_malo_aceptado" if rep["status"] == "VALID" else "v_abstencion_innecesaria"
    if es_abstencion(caso):
        return "iv_abstencion_correcta" if esperado in ("INVALID", "INCOMPLETE") else "v_abstencion_innecesaria"
    return "i_no_parseo" if gold["intent"] in ("rule_transformation", "free_language") else "ii_rir_malo_aceptado"


# [DESIGN] Resultado honesto esperado por id de la sonda de abstencion (conjunto interno,
# compuesto por el agente; NO equivale a un conjunto de terceros).
ORO_PROBE = {
    "desconocida_09": {"desconocida": None}, "fuera_dominio_09": None,
}


def clasificar_probe(caso: dict, categoria: str) -> str:
    rep = caso["repeats"][0]
    texto = rep["text"].lower()
    abst = es_abstencion(caso)
    transparente = "entiendo" in texto or "entendi" in texto

    if categoria == "entidad_desconocida":
        return "iv_abstencion_correcta" if abst else "ii_rir_malo_aceptado"
    if categoria == "fuera_de_dominio":
        return "iv_abstencion_correcta" if abst else "fracaso_ruta"
    if categoria == "frase_ambigua":
        if abst or transparente:
            return "iv_abstencion_correcta"
        return "ii_rir_malo_aceptado"
    if categoria == "cebo_falsa_aceptacion":
        # cebo_03/cebo_04: la respuesta VALID "No, no existe..." es correcta (no es falsa aceptacion)
        if caso.get("id","").startswith("bait_falso_acepte_03") or caso.get("id","").startswith("bait_falso_acepte_04"):
            if rep["route"] == "deterministic_graph" and ("no existe" in texto or "no hay" in texto or "no se puede" in texto):
                return "correcto"
            return "iii_salida_incorrecta_ruta_ok"
        return "iv_abstencion_correcta" if abst else "ii_rir_malo_aceptado"
    return "sin_oro"


def analizar_par(dump_path: str, suite_path: str | None) -> dict:
    with open(dump_path, encoding="utf-8") as f:
        dump = json.load(f)
    golds = {}
    categorias = {}
    if suite_path:
        with open(suite_path, encoding="utf-8") as f:
            suite = json.load(f)
        items = suite["cases"] if isinstance(suite, dict) and "cases" in suite else suite
        for it in items:
            key = it.get("id") or (it.get("input") or it.get("text") or "").strip().lower()
            golds[key] = it
            cat = it.get("category") or it.get("categoria") or ""
            categorias[key] = cat

    es_probe = any(v.startswith("entidad_desconocida") for v in categorias.values()) if categorias else False

    por_label: dict[str, int] = {}
    lat_p50: list[float] = []
    lat_p95: list[float] = []
    rss_max = 0.0
    det_no_deterministas = 0
    total = 0
    detalle = []
    for caso in dump["cases"]:
        total += 1
        key = caso.get("id") or (caso.get("input") or "").strip().lower()
        oro_full = golds.get(key, {})
        if es_probe:
            label = clasificar_probe(caso, categorias.get(key, ""))
        elif oro_full.get("expected_intent"):
            g = oro_semantico(oro_full)
            label = clasificar_semantico(caso, g, oro_full) if g else clasificar_simple(caso, oro_full)
        else:
            label = clasificar_simple(caso, oro_full)
        por_label[label] = por_label.get(label, 0) + 1
        lat_p50.append(caso["latency_us"]["p50"])
        lat_p95.append(caso["latency_us"]["p95"])
        rss_max = max(rss_max, caso.get("rss_kb_max", 0.0))
        if not caso["deterministic"]:
            det_no_deterministas += 1
        detalle.append({"id": caso.get("id") or caso.get("input"), "label": label})

    correctos = por_label.get("correcto", 0)
    abst_ok = por_label.get("iv_abstencion_correcta", 0)
    abst_total = sum(v for k, v in por_label.items() if k.startswith("iv_"))
    resumen = {
        "dump": dump_path,
        "suite": suite_path,
        "total_casos": total,
        "precision_pct": round(100.0 * correctos / max(total, 1), 1),
        "precision_extendida_pct": round(100.0 * (correctos + abst_ok) / max(total, 1), 1),
        "por_label": por_label,
        "determinismo_pct": round(100.0 * (total - det_no_deterministas) / max(total, 1), 1),
        "latencia_us_p50_mediana": sorted(lat_p50)[len(lat_p50)//2] if lat_p50 else 0,
        "latencia_us_p95_max_caso": max(lat_p95) if lat_p95 else 0,
        "rss_kb_max": rss_max,
        "abstencion_pct": round(100.0 * abst_total / max(total, 1), 1),
        "detalle": detalle,
    }
    return resumen


def main() -> None:
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        sys.exit(1)
    pares = []
    i = 0
    while i < len(args):
        dump = args[i]
        succed = args[i + 1] if i + 1 < len(args) else None
        if succed and os.path.isfile(succed) and "dump" not in os.path.basename(succed):
            pares.append((dump, succed))
            i += 2
        else:
            pares.append((dump, None))
            i += 1

    os.makedirs("reports/eval", exist_ok=True)
    todo = []
    for dump, suite in pares:
        r = analizar_par(dump, suite)
        todo.append(r)
        stem = os.path.splitext(os.path.basename(dump))[0]
        out = f"reports/eval/analisis_{stem}.json"
        with open(out, "w", encoding="utf-8") as f:
            json.dump(r, f, ensure_ascii=False, indent=2)
        print(f"== {os.path.basename(dump)} -> {out}")
        print(f"   casos={r['total_casos']} precision={r['precision_pct']}% determinismo={r['determinismo_pct']}% abstencion={r['abstencion_pct']}% lat_p50={r['latencia_us_p50_mediana']:.0f}us lat_p95_max={r['latencia_us_p95_max_caso']:.0f}us rss_max={r['rss_kb_max']:.0f}kb")
        print(f"   labels: {json.dumps(r['por_label'], ensure_ascii=False)}")
    with open("reports/eval/analisis_consolidado.json", "w", encoding="utf-8") as f:
        json.dump(todo, f, ensure_ascii=False, indent=2)
    print("consolidado: reports/eval/analisis_consolidado.json")


if __name__ == "__main__":
    main()
