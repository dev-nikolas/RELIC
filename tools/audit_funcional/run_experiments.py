#!/usr/bin/env python3
"""
FASE 3: Experimentos de Línea Base de Cambio y Aprendizaje
Ejecuta y documenta:
1. Palabra / Verbo nuevo: "se embolataron" ("Tengo 10 peras y se me embolataron 3")
2. Hecho nuevo: "Cali conecta con Medellín. ¿Se puede ir de Cali a Medellín?"
3. Corrección del usuario: "No, 2 + 2 no es 4, en este juego 2 + 2 es 5" seguido de "¿cuánto es 2 + 2?"
"""

import json
import time
import urllib.request
import urllib.error
from pathlib import Path
from typing import Dict, Any

SERVER_URL = "http://127.0.0.1:5000/api/query"


def query_api(query_text: str) -> Dict[str, Any]:
    req_body = json.dumps({"query": query_text, "enable_trace": True}).encode("utf-8")
    req = urllib.request.Request(
        SERVER_URL,
        data=req_body,
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    t0 = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=5.0) as resp:
            elapsed_ms = (time.perf_counter() - t0) * 1000.0
            data = json.loads(resp.read().decode("utf-8"))
            data["http_code"] = resp.status
            data["latency_ms"] = elapsed_ms
            return data
    except urllib.error.HTTPError as e:
        return {
            "http_code": e.code,
            "text": e.read().decode("utf-8", errors="replace"),
            "status": f"HTTP_{e.code}",
            "route": "http_error",
            "latency_ms": 0.0
        }
    except Exception as e:
        return {
            "http_code": 0,
            "text": str(e),
            "status": "CONNECTION_ERROR",
            "route": "client_error",
            "latency_ms": 0.0
        }


def main():
    print("=== FASE 3: EXPERIMENTOS DE LÍNEA BASE DE CAMBIO Y APRENDIZAJE ===")
    
    experiments = [
        {
            "id": "exp_3b_1_verbo_nuevo",
            "title": "Verbo coloquial no registrado ('se embolataron')",
            "prompt": "Tengo 10 peras y se me embolataron 3",
            "expected_human": "Entender que 'se embolataron' equivale a perder/restar 3 peras -> quedan 7.",
            "theoretical_reason": (
                "El analizador de inventarios (relic_cli/src/dispatch.rs:76-81 y inverted.rs:18-24) "
                "posee una lista exhaustiva y cerrada de cadenas hardcodeadas para resta ('regalo', 'pierdo', "
                "'gasto', etc.). El verbo 'embolataron' no existe en dicha lista estática compilada en el binario Rust. "
                "No existe mecanismo de inducción léxica, consulta a diccionario externo ni tabla de alias dinámica. "
                "Por ende, el clasificador rechaza la cláusula y retorna abstención epistémica INCOMPLETE."
            )
        },
        {
            "id": "exp_3b_2_hecho_nuevo",
            "title": "Inyección de hecho nuevo en lenguaje natural",
            "prompt": "Cali está conectada con Medellín. ¿Se puede ir de Cali a Medellín?",
            "expected_human": "Aprender la arista Cali -> Medellín y confirmar que sí se puede ir.",
            "theoretical_reason": (
                "El servidor relic_server expone únicamente una función de lectura sobre EpisodicMemory "
                "(crates/relic_server/src/engine.rs:151-155 y router.rs:95-103). Los hechos episódicos están "
                "hardcodeados en código compilado (crates/relic_cli/src/knowledge.rs:6-23) y se inicializan solo al arrancar. "
                "No existe endpoint de mutación (POST /v1/knowledge o similar) ni parser de aserciones factuales. "
                "Al procesar la pregunta, Cali y Medellín no existen en la memoria episódica fija, por lo que el "
                "resolutor de nodos estricto (relic_cli/src/answer.rs:77-87) emite INVALID por entidad inexistente."
            )
        },
        {
            "id": "exp_3b_3_correccion_usuario",
            "title": "Corrección de regla por el usuario (Axioma local)",
            "prompt": "En este juego 2 + 2 es igual a 5. ¿Cuánto es 2 + 2?",
            "expected_human": "Adoptar la regla contextual y responder 5, o al menos registrar el conflicto.",
            "theoretical_reason": (
                "El motor aritmético (crates/relic_core/src/arithmetic.rs:50-78 y relic_cli/src/math.rs:14-139) "
                "ejecuta operaciones puras sobre punto flotante f64 de CPU de manera determinista e invariable. "
                "No hay capa semántica de asunciones o axiomas contextuales que pueda sobreescribir la tabla "
                "de operaciones aritméticas elementales. El servidor no mantiene estado de sesión ni contexto de turnos; "
                "cada invocación es stateless. Si el usuario intenta redefinir una verdad analítica, el parser extrae "
                "'2 + 2' y evalúa invariablemente a 4."
            )
        },
        {
            "id": "exp_3b_4_segundo_turno_correccion",
            "title": "Turno subsecuente tras intento de enseñanza previa",
            "prompt": "2 + 2",
            "expected_human": "Comprobar si persistió algún cambio tras la corrección anterior.",
            "theoretical_reason": (
                "El diseño del servidor HTTP en relic_server es estrictamente sin estado conversacional "
                "(ServerState solo contiene el modelo inmutable y la memoria episódica pre-cargada). "
                "No se almacena historial de turnos previos entre peticiones sucesivas."
            )
        }
    ]

    results = []
    for exp in experiments:
        print(f"\n[Ejecutando] {exp['id']}: {exp['title']}")
        print(f"  Prompt: \"{exp['prompt']}\"")
        res = query_api(exp["prompt"])
        
        status = res.get("status")
        route = res.get("route")
        text = res.get("text")
        trace = res.get("trace", [])
        
        print(f"  -> HTTP {res.get('http_code')} | Status: {status} | Ruta: {route}")
        print(f"  -> Respuesta: {text}")
        print(f"  -> Etapas de traza: {len(trace)}")
        
        exp_result = {
            "id": exp["id"],
            "title": exp["title"],
            "prompt": exp["prompt"],
            "status": status,
            "route": route,
            "text": text,
            "latency_ms": res.get("latency_ms"),
            "trace": trace,
            "expected_human": exp["expected_human"],
            "theoretical_reason": exp["theoretical_reason"]
        }
        results.append(exp_result)

    out_file = Path("reports/audit_funcional/fase3_experimentos_aprendizaje.json")
    out_file.parent.mkdir(parents=True, exist_ok=True)
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
        
    print(f"\n[OK] Resultados de Fase 3 guardados en: {out_file.resolve()}")


if __name__ == "__main__":
    main()
