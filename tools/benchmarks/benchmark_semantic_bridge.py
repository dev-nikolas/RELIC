#!/usr/bin/env python3
"""Benchmark del Puente Neuro-Simbólico: Interpretación, Robustez y Memoria Real.

Evalúa:
1. Precisión de Interpretación Semántica (parafraseo, restricciones, negaciones).
2. Manejo de Casos Límite (datos incompletos, entidades desconocidas).
3. Latencia desagregada por etapa (Interpretación -> Recuperación -> Ejecución -> Respuesta).
4. Escalabilidad y consumo de memoria física real en bytes para N=10, 100, 1000 nodos.
"""

import json
import sys
import time
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))

from relic.episodic_memory import GraphEpisodicMemory, RuleEpisodicCatalog
from relic.semantic_bridge import NeuroSymbolicBridge, SemanticInterpreter

sys.stdout.reconfigure(encoding="utf-8")

BENCHMARK_PATH = ROOT_DIR / "data" / "benchmarks" / "semantic_queries_es.json"
REPORT_PATH = ROOT_DIR / "reports" / "semantic_bridge_report.json"


def measure_memory_bytes(obj, seen=None) -> int:
    """Mide el tamaño recursivo real en bytes de una estructura en memoria."""
    size = sys.getsizeof(obj)
    if seen is None:
        seen = set()
    obj_id = id(obj)
    if obj_id in seen:
        return 0
    seen.add(obj_id)
    if isinstance(obj, dict):
        size += sum(measure_memory_bytes(v, seen) for v in obj.values())
        size += sum(measure_memory_bytes(k, seen) for k in obj.keys())
    elif isinstance(obj, (list, tuple, set, frozenset)):
        size += sum(measure_memory_bytes(i, seen) for i in obj)
    return size


def run_benchmark():
    print("=" * 80)
    print(" BENCHMARK DEL PUENTE NEURO-SIMBÓLICO EN ESPAÑOL (RELIC)")
    print("============================================================")

    with open(BENCHMARK_PATH, "r", encoding="utf-8") as f:
        test_queries = json.load(f)

    # Inicializar memoria con hechos del mundo de prueba
    memory = GraphEpisodicMemory()
    # Conexiones de prueba
    memory.insert_edge("Madrid", "Barcelona")
    memory.insert_edge("Barcelona", "París")
    memory.insert_edge("Madrid", "Toledo")
    memory.insert_edge("Bogotá", "Lima")
    memory.insert_edge("Lima", "Buenos Aires")
    memory.insert_edge("Marta", "estación")
    memory.insert_edge("estación", "biblioteca")
    memory.insert_edge("nodo_A", "nodo_B")
    memory.insert_edge("servidor_1", "balanceador")
    memory.insert_edge("balanceador", "base_de_datos")

    rules = RuleEpisodicCatalog()
    rules.register_rule("reverse", lambda s: s[::-1])
    rules.register_rule("swap_case", lambda s: s.swapcase())
    rules.register_rule("duplicate_vowels", lambda s: "".join(c * 2 if c.lower() in "aeiou" else c for c in s))

    bridge = NeuroSymbolicBridge(memory=memory, rules=rules)

    total_queries = len(test_queries)
    intent_correct = 0
    slots_correct = 0
    rejections_correct = 0

    timing_breakdown = {
        "interpretation_us": [],
        "retrieval_and_exec_us": [],
        "response_us": [],
        "total_us": [],
    }

    results_details = []

    for item in test_queries:
        text = item["text"]
        exp_intent = item["expected_intent"]
        exp_orig = item["expected_origin"]
        exp_dest = item["expected_destination"]
        exp_via = item["expected_via"]
        exp_neg = item["is_negated"]

        t0 = time.perf_counter()
        task = bridge.interpreter.parse(text)
        t1 = time.perf_counter()
        res = bridge.execute_query(text)
        t2 = time.perf_counter()

        dt_interp = (t1 - t0) * 1e6
        dt_total = (t2 - t0) * 1e6
        dt_exec = dt_total - dt_interp

        timing_breakdown["interpretation_us"].append(dt_interp)
        timing_breakdown["retrieval_and_exec_us"].append(dt_exec)
        timing_breakdown["total_us"].append(dt_total)

        # Validación
        matches_intent = (task.intent == exp_intent)
        matches_slots = (
            (task.origin == exp_orig) and
            (task.destination == exp_dest) and
            (task.via == exp_via) and
            (task.is_negated == exp_neg)
        )

        if matches_intent:
            intent_correct += 1
        if matches_slots:
            slots_correct += 1

        is_rejection_case = item["category"] in {"incomplete_data", "unknown_entities"}
        properly_handled = False
        if is_rejection_case:
            if not res["success"] and not res["facts_verified"]:
                rejections_correct += 1
                properly_handled = True
        else:
            if res["success"]:
                properly_handled = True

        results_details.append({
            "id": item["id"],
            "query": text,
            "category": item["category"],
            "parsed_task": {
                "intent": task.intent,
                "origin": task.origin,
                "destination": task.destination,
                "via": task.via,
                "is_negated": task.is_negated,
            },
            "system_response": res["response"],
            "properly_handled": properly_handled,
            "latency_us": round(dt_total, 2),
        })

    print(f"\n1. PRECISIÓN SEMÁNTICA Y DE CONTROL ({total_queries} casos controlados):")
    print(f"   - Clasificación de Intención:        {(intent_correct / total_queries) * 100:.1f}%")
    print(f"   - Extracción Exacta de Slots:        {(slots_correct / total_queries) * 100:.1f}%")
    print(f"   - Detección de Casos Incompletos:    {rejections_correct}/3 casos anómalos correctamente rechazados sin alucinar")

    avg_interp = sum(timing_breakdown["interpretation_us"]) / total_queries
    avg_exec = sum(timing_breakdown["retrieval_and_exec_us"]) / total_queries
    avg_total = sum(timing_breakdown["total_us"]) / total_queries

    print(f"\n2. LATENCIA DESAGREGADA PROMEDIO:")
    print(f"   - Etapa 1 (Interpretación Semántica):   {avg_interp:>6.2f} µs")
    print(f"   - Etapa 2 y 3 (Recuperación y Ejec.):  {avg_exec:>6.2f} µs")
    print(f"   - Ciclo Completo End-to-End:           {avg_total:>6.2f} µs ({avg_total / 1000.0:.3f} ms)")

    # 3. Medición del Consumo Real de Memoria Física en Bytes
    print(f"\n3. CONSUMO REAL DE MEMORIA SEGÚN TAMAÑO DEL GRAFO:")
    scale_benchmarks = {}
    for n_nodes in [10, 100, 1000]:
        mem_scale = GraphEpisodicMemory()
        # Generar grafo de prueba conexo con 5 aristas por nodo
        for i in range(n_nodes):
            for k in range(1, 6):
                target = (i + k) % n_nodes
                mem_scale.insert_edge(f"Node_{i:04d}", f"Node_{target:04d}")

        raw_bytes = measure_memory_bytes(mem_scale._adj) + measure_memory_bytes(mem_scale._edges)
        scale_benchmarks[f"nodes_{n_nodes}"] = {
            "nodes": n_nodes,
            "edges": mem_scale.edge_count(),
            "memory_bytes": raw_bytes,
            "memory_kb": round(raw_bytes / 1024.0, 2),
            "memory_mb": round(raw_bytes / (1024.0 * 1024.0), 4),
        }
        print(f"   - N = {n_nodes:<4} nodos | Aristas: {mem_scale.edge_count():<5} | Memoria Real: {raw_bytes:>8,} bytes ({raw_bytes / 1024.0:>6.2f} KB)")

    report_data = {
        "title": "Evaluación del Puente Neuro-Simbólico en Español para RELIC",
        "date": "2026-10-21",
        "accuracy": {
            "intent_accuracy_percent": round((intent_correct / total_queries) * 100, 2),
            "slot_accuracy_percent": round((slots_correct / total_queries) * 100, 2),
            "total_queries_evaluated": total_queries,
        },
        "timing_microseconds": {
            "interpretation_avg_us": round(avg_interp, 2),
            "execution_avg_us": round(avg_exec, 2),
            "total_avg_us": round(avg_total, 2),
        },
        "memory_scaling_bytes": scale_benchmarks,
        "query_details": results_details,
    }

    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT_PATH, "w", encoding="utf-8") as f:
        json.dump(report_data, f, indent=2, ensure_ascii=False)

    print(f"\nReporte guardado con éxito en: {REPORT_PATH}")


if __name__ == "__main__":
    run_benchmark()
