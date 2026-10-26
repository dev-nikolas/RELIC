#!/usr/bin/env python3
"""
tools/audit/run_reasoning_probe.py
Ejecuta la batería de sondeo cognitivo sobre RELIC HTTP Server (/api/query)
con enable_trace=True y guarda los resultados en reports/audit/reasoning_probe.json.
"""
import json
import os
import sys
import time
import urllib.request
from pathlib import Path

SERVER_URL = "http://127.0.0.1:5000/api/query"

TEST_BATTERY = [
    # Aritmética
    {"category": "aritmetica", "query": "cuánto es 15 por 3"},
    {"category": "aritmetica", "query": "veintiuno más nueve"},
    {"category": "aritmetica", "query": "parce cuánto es 7 más 8"},
    {"category": "aritmetica", "query": "tengo 10 y regalo 3"},
    {"category": "aritmetica", "query": "resta 3 y 10"},
    {"category": "aritmetica", "query": "10 / 0"},
    {"category": "aritmetica", "query": "dos más dos son cuatro?"},

    # Fuera de cobertura
    {"category": "fuera_de_cobertura", "query": "me tumbaron 5 de 20 gallinas, cuántas quedan?"},
    {"category": "fuera_de_cobertura", "query": "reparto 12 entre 4"},
    {"category": "fuera_de_cobertura", "query": "dame la mitad de 50"},
    {"category": "fuera_de_cobertura", "query": "tengo 3 gatos y 2 perros, cuántos animales?"},
    {"category": "fuera_de_cobertura", "query": "el cielo es azul"},
    {"category": "fuera_de_cobertura", "query": "hola"},

    # Memoria y grafos
    {"category": "memoria_grafos", "query": "¿Se puede ir de Bogotá a Lima?"},
    {"category": "memoria_grafos", "query": "¿Depende relic_cli de memmap2?"},
    {"category": "memoria_grafos", "query": "¿Depende el crate relic_cli del paquete memmap2?"},
    {"category": "memoria_grafos", "query": "¿Depende el crate relic_cli del paquete tokio?"},
    {"category": "memoria_grafos", "query": "¿Depende el crate Relic_Cli del paquete memmap2?"},

    # 20 frases nuevas
    {"category": "nueva_modismo", "query": "quiubo socio, súmeme 14 y 26"},
    {"category": "nueva_modismo", "query": "cuánto me queda si me gasto 50 lucas de 100 lucas"},
    {"category": "nueva_modismo", "query": "hágale la rebaja de 15 a 80"},
    {"category": "nueva_orden_invertido", "query": "a 40 réstele 12"},
    {"category": "nueva_orden_invertido", "query": "100 menos 25 cuánto da"},
    {"category": "nueva_orden_invertido", "query": "por 8 multiplique 7"},
    {"category": "nueva_ortografia", "query": "cuanto hes dies mas sinko"},
    {"category": "nueva_ortografia", "query": "restar sinco de dies"},
    {"category": "nueva_ortografia", "query": "divicion de 50 entre 2"},
    {"category": "nueva_grandes", "query": "cuánto es 1000000 más 2500000"},
    {"category": "nueva_grandes", "query": "multiplica 999999 por 0"},
    {"category": "nueva_decimales", "query": "suma 12.5 y 3.75"},
    {"category": "nueva_decimales", "query": "cuánto es 0.5 por 4"},
    {"category": "nueva_decimales", "query": "resta 0.1 a 1.0"},
    {"category": "nueva_ambiguedad", "query": "¿es mejor sumar o restar?"},
    {"category": "nueva_ambiguedad", "query": "tengo manzanas y peras"},
    {"category": "nueva_ambiguedad", "query": "cuánto vale la vida"},
    {"category": "nueva_grafos", "query": "¿está conectado relic_server con tiny_http?"},
    {"category": "nueva_grafos", "query": "¿Depende el paquete tokio de relic_core?"},
    {"category": "nueva_sintaxis", "query": "de 50 quitar 15"},
]


def run_probe():
    results = []
    print(f"Lanzando batería de sondeo ({len(TEST_BATTERY)} consultas) a {SERVER_URL}...")

    resolved_count = 0
    abstention_count = 0
    error_count = 0
    confidently_wrong_count = 0

    for idx, item in enumerate(TEST_BATTERY):
        q = item["query"]
        cat = item["category"]
        payload = json.dumps({"query": q, "enable_trace": True}).encode("utf-8")
        req = urllib.request.Request(
            SERVER_URL,
            data=payload,
            headers={"Content-Type": "application/json"},
            method="POST",
        )

        t0 = time.perf_counter()
        try:
            with urllib.request.urlopen(req, timeout=5.0) as resp:
                status_code = resp.status
                data = json.loads(resp.read().decode("utf-8"))
        except Exception as e:
            status_code = 500
            data = {"text": f"Error: {e}", "route": "client_error", "status": "ERROR", "trace": []}

        elapsed_ms = (time.perf_counter() - t0) * 1000

        text = data.get("text", "")
        route = data.get("route", "")
        status = data.get("status", "")
        trace = data.get("trace", [])

        is_resolved = status == "VALID"
        is_abstained = status in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS", "DEGRADED"]

        if is_resolved:
            resolved_count += 1
        elif is_abstained:
            abstention_count += 1
        else:
            error_count += 1

        record = {
            "id": idx + 1,
            "category": cat,
            "query": q,
            "http_status": status_code,
            "response_status": status,
            "route": route,
            "text": text,
            "latency_ms": data.get("latency_ms", elapsed_ms),
            "trace": trace,
        }
        results.append(record)
        print(f"[{idx+1:02d}/{len(TEST_BATTERY)}] [{status:11s}] [{route:25s}] '{q[:30]}...' -> '{text[:40]}...'")

    summary = {
        "total_probes": len(TEST_BATTERY),
        "resolved_valid": resolved_count,
        "abstained": abstention_count,
        "errors": error_count,
        "results": results,
    }

    repo_root = Path(__file__).resolve().parents[2]
    out_file = repo_root / "reports" / "audit" / "reasoning_probe.json"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2, ensure_ascii=False)

    print("\n" + "=" * 60)
    print(f"Resumen de Sondeo Cognitivo RELIC:")
    print(f"  Total Evaluado: {len(TEST_BATTERY)}")
    print(f"  Resueltos (VALID): {resolved_count} ({resolved_count/len(TEST_BATTERY)*100:.1f}%)")
    print(f"  Abstenciones:      {abstention_count} ({abstention_count/len(TEST_BATTERY)*100:.1f}%)")
    print(f"  Errores/Otros:     {error_count}")
    print(f"Reporte guardado en: {out_file}")
    print("=" * 60)


if __name__ == "__main__":
    run_probe()
