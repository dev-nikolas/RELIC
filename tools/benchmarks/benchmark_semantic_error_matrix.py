#!/usr/bin/env python3
"""Benchmark: Matriz de Errores Semánticos y Comparación de Tres Métodos de Interpretación.

Evalúa empíricamente los 4 puntos propuestos:
1. Matriz de Errores Desagregada: origen, destino, waypoint, negación, estado tripartito.
2. Comparación de 3 Métodos:
   - Método A: Regex y reglas ingenuas.
   - Método B: Parser sintáctico de reglas explícitas.
   - Método C: H1 como compilador semántico / extractor neuronal.
3. Casos Adversariales: doble negación, exclusiones ('sin pasar por C'), waypoints inexistentes.
4. Verificación End-to-End con separación estricta de estados:
   - VÁLIDA (hecho conocido y resoluble)
   - INCOMPLETA (aclaración solicitada)
   - INVÁLIDA (entidad desconocida / irresoluble rechazada sin alucinar)
"""

import json
import sys
import time
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT_DIR / "python"))

from relic.episodic_memory import GraphEpisodicMemory, RuleEpisodicCatalog
from relic.semantic_bridge import (
    GrammarRuleParser,
    H1NeuralSemanticParser,
    NaiveRegexParser,
    VerifiedReasoningPipeline,
)

sys.stdout.reconfigure(encoding="utf-8")

BENCHMARK_PATH = ROOT_DIR / "data" / "benchmarks" / "semantic_adversarial_es.json"
REPORT_PATH = ROOT_DIR / "reports" / "semantic_error_matrix_report.json"


def evaluate_parser_method(name: str, parser_instance, test_queries: list[dict], memory: GraphEpisodicMemory, rules: RuleEpisodicCatalog):
    pipeline = VerifiedReasoningPipeline(memory=memory, rules=rules, parser=parser_instance)

    errors = {
        "intent_errors": 0,
        "origin_errors": 0,
        "destination_errors": 0,
        "waypoint_errors": 0,
        "avoid_errors": 0,
        "negation_errors": 0,
        "status_errors": 0,
        "strict_tuple_matches": 0,
        "end_to_end_correct": 0,
    }

    latencies_us = []
    total = len(test_queries)

    sample_logs = []

    for item in test_queries:
        text = item["text"]
        exp_status = item["expected_status"]
        exp_intent = item["expected_intent"]
        exp_orig = item["expected_origin"]
        exp_dest = item["expected_destination"]
        exp_via = item["expected_via"]
        exp_avoid = item["expected_avoid"]
        exp_neg = item["is_negated"]

        t0 = time.perf_counter()
        result = pipeline.process_query(text)
        dt_us = (time.perf_counter() - t0) * 1e6
        latencies_us.append(dt_us)

        task = result["task"]

        # 1. Comprobar errores por slot
        has_intent_err = (task.intent != exp_intent)
        has_orig_err = (task.origin != exp_orig)
        has_dest_err = (task.destination != exp_dest)
        has_via_err = (task.via != exp_via)
        has_avoid_err = (task.avoid != exp_avoid)
        has_neg_err = (task.is_negated != exp_neg)
        has_status_err = (result["status"] != exp_status)

        if has_intent_err: errors["intent_errors"] += 1
        if has_orig_err: errors["origin_errors"] += 1
        if has_dest_err: errors["destination_errors"] += 1
        if has_via_err: errors["waypoint_errors"] += 1
        if has_avoid_err: errors["avoid_errors"] += 1
        if has_neg_err: errors["negation_errors"] += 1
        if has_status_err: errors["status_errors"] += 1

        is_strict_tuple = not (has_intent_err or has_orig_err or has_dest_err or has_via_err or has_avoid_err or has_neg_err)
        if is_strict_tuple:
            errors["strict_tuple_matches"] += 1

        # Verificación end-to-end: interpretación correcta y manejo del estado correcto
        if is_strict_tuple and not has_status_err:
            errors["end_to_end_correct"] += 1

        sample_logs.append({
            "id": item["id"],
            "query": text,
            "category": item["category"],
            "expected": {
                "status": exp_status,
                "intent": exp_intent,
                "origin": exp_orig,
                "destination": exp_dest,
                "via": exp_via,
                "avoid": exp_avoid,
                "is_negated": exp_neg,
            },
            "predicted": {
                "status": result["status"],
                "intent": task.intent,
                "origin": task.origin,
                "destination": task.destination,
                "via": task.via,
                "avoid": task.avoid,
                "is_negated": task.is_negated,
            },
            "response": result["response"],
            "is_strict_match": is_strict_tuple,
            "is_end_to_end_valid": (is_strict_tuple and not has_status_err),
        })

    avg_lat = sum(latencies_us) / max(1, total)

    return {
        "method_name": name,
        "total_queries": total,
        "errors": errors,
        "rates": {
            "strict_tuple_accuracy_pct": round((errors["strict_tuple_matches"] / total) * 100.0, 2),
            "end_to_end_accuracy_pct": round((errors["end_to_end_correct"] / total) * 100.0, 2),
            "intent_error_rate_pct": round((errors["intent_errors"] / total) * 100.0, 2),
            "origin_error_rate_pct": round((errors["origin_errors"] / total) * 100.0, 2),
            "destination_error_rate_pct": round((errors["destination_errors"] / total) * 100.0, 2),
            "waypoint_error_rate_pct": round((errors["waypoint_errors"] / total) * 100.0, 2),
            "avoid_error_rate_pct": round((errors["avoid_errors"] / total) * 100.0, 2),
            "negation_error_rate_pct": round((errors["negation_errors"] / total) * 100.0, 2),
            "status_confusion_rate_pct": round((errors["status_errors"] / total) * 100.0, 2),
        },
        "avg_latency_us": round(avg_lat, 2),
        "sample_logs": sample_logs,
    }


def main():
    print("=" * 80)
    print(" EXPERIMENTO: LOCALIZACIÓN DEL ERROR SEMÁNTICO Y MATRIZ DE COMPARACIÓN")
    print("=" * 80)

    with open(BENCHMARK_PATH, "r", encoding="utf-8") as f:
        test_queries = json.load(f)

    # Hechos conocidos en la memoria episódica
    memory = GraphEpisodicMemory()
    memory.insert_edge("Madrid", "Barcelona")
    memory.insert_edge("Barcelona", "París")
    memory.insert_edge("Madrid", "Toledo")
    memory.insert_edge("Bogotá", "Lima")
    memory.insert_edge("Lima", "Buenos Aires")
    memory.insert_edge("Marta", "estación")
    memory.insert_edge("estación", "biblioteca")
    memory.insert_edge("nodo_A", "nodo_B")
    memory.insert_edge("servidor_1", "base_de_datos")

    rules = RuleEpisodicCatalog()
    rules.register_rule("reverse", lambda s: s[::-1])
    rules.register_rule("swap_case", lambda s: s.swapcase())

    methods = [
        ("Método A: Naive Regex", NaiveRegexParser()),
        ("Método B: Grammar Rule Parser", GrammarRuleParser()),
        ("Método C: H1 Neural Compiler", H1NeuralSemanticParser()),
    ]

    report = {}

    for name, parser in methods:
        eval_res = evaluate_parser_method(name, parser, test_queries, memory, rules)
        report[name] = eval_res

    # Imprimir Matriz de Errores Desagregada
    print("\n" + "=" * 95)
    print(" MATRIZ DE ERRORES DESAGREGADA POR COMPONENTE SEMÁNTICO (16 CASOS ADVERSARIALES)")
    print("=" * 95)
    print(f"{'Métrica / Ranura Evaluada':<32} | {'Método A (Naive Regex)':<20} | {'Método B (Grammar)':<18} | {'Método C (H1 Neural)':<18}")
    print("-" * 95)

    metrics_to_show = [
        ("Exactitud Tupla Completa (Strict)", "strict_tuple_accuracy_pct", "%"),
        ("Exactitud End-to-End Verificada", "end_to_end_accuracy_pct", "%"),
        ("Error en Clasificación de Intención", "intent_error_rate_pct", "%"),
        ("Error en Detección de Origen", "origin_error_rate_pct", "%"),
        ("Error en Detección de Destino", "destination_error_rate_pct", "%"),
        ("Error en Restricción Waypoint", "waypoint_error_rate_pct", "%"),
        ("Error en Restricción Avoid (sin pasar)", "avoid_error_rate_pct", "%"),
        ("Error en Negación / Doble Negación", "negation_error_rate_pct", "%"),
        ("Confusión de Estado (Valid/Incom/Inv)", "status_confusion_rate_pct", "%"),
    ]

    mA = report["Método A: Naive Regex"]["rates"]
    mB = report["Método B: Grammar Rule Parser"]["rates"]
    mC = report["Método C: H1 Neural Compiler"]["rates"]

    for label, key, unit in metrics_to_show:
        valA = f"{mA[key]:.1f}{unit}"
        valB = f"{mB[key]:.1f}{unit}"
        valC = f"{mC[key]:.1f}{unit}"
        print(f"{label:<32} | {valA:>18}   | {valB:>16}   | {valC:>16}")

    print("-" * 95)
    latA = f"{report['Método A: Naive Regex']['avg_latency_us']:.1f} µs"
    latB = f"{report['Método B: Grammar Rule Parser']['avg_latency_us']:.1f} µs"
    latC = f"{report['Método C: H1 Neural Compiler']['avg_latency_us']:.1f} µs"
    print(f"{'Latencia Promedio End-to-End':<32} | {latA:>18}   | {latB:>16}   | {latC:>16}")
    print("=" * 95)

    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(REPORT_PATH, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    print(f"\nReporte detallado guardado en: {REPORT_PATH}")


if __name__ == "__main__":
    main()
