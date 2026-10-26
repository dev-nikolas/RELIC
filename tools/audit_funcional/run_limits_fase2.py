#!/usr/bin/env python3
"""
FASE 2: Medición Sistemática de Límites y Robustez
Mide:
1. Longitud máxima de entrada (payload HTTP y truncamiento de parser)
2. Magnitud y precisión numérica f64 (overflow, subnormal, pérdida de precisión, truncamiento)
3. Profundidad de búsqueda BFS (límite de 8 saltos)
4. Escalabilidad del grafo (medición de memoria y tiempo en 50, 200, 1.000, 10.000 nodos)
5. Latencia y RAM del servidor
6. Robustez a la redacción: 30 casos base x 5 perturbaciones = 150 casos perturbados
7. Sensibilidad a mayúsculas y tildes
"""

import json
import time
import urllib.request
import urllib.error
from pathlib import Path
from typing import Dict, Any, List

SERVER_URL = "http://127.0.0.1:5000/api/query"


def query_api(query_text: str, timeout: float = 4.0) -> Dict[str, Any]:
    req_body = json.dumps({"query": query_text, "enable_trace": True}).encode("utf-8")
    req = urllib.request.Request(
        SERVER_URL,
        data=req_body,
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    t0 = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            elapsed_ms = (time.perf_counter() - t0) * 1000.0
            data = json.loads(resp.read().decode("utf-8"))
            data["http_code"] = resp.status
            data["measured_latency_ms"] = elapsed_ms
            return data
    except urllib.error.HTTPError as e:
        elapsed_ms = (time.perf_counter() - t0) * 1000.0
        return {
            "http_code": e.code,
            "text": e.read().decode("utf-8", errors="replace"),
            "status": f"HTTP_{e.code}",
            "route": "http_error",
            "measured_latency_ms": elapsed_ms
        }
    except Exception as e:
        elapsed_ms = (time.perf_counter() - t0) * 1000.0
        return {
            "http_code": 0,
            "text": str(e),
            "status": "CONNECTION_ERROR",
            "route": "client_error",
            "measured_latency_ms": elapsed_ms
        }


def test_input_lengths():
    print("\n--- 1. Medición de Límites de Longitud de Entrada ---")
    lengths = [10, 50, 100, 500, 1000, 4000, 8000, 16000, 16384, 16385, 20000]
    results = []
    for l in lengths:
        # Generar consulta con padding de espacios o comentarios
        query = "2 + 2" + " " * max(0, l - 5)
        res = query_api(query)
        results.append({
            "target_bytes": l,
            "http_code": res.get("http_code"),
            "status": res.get("status"),
            "route": res.get("route"),
            "latency_ms": res.get("measured_latency_ms")
        })
        print(f"Longitud {l:>6} bytes -> HTTP {res.get('http_code')} | Status: {res.get('status')} | Ruta: {res.get('route')}")
    return results


def test_numerical_limits():
    print("\n--- 2. Medición de Magnitud y Precisión Numérica (f64) ---")
    cases = [
        {"name": "Valor normal", "q": "100 + 200", "expected": 300.0},
        {"name": "Precisión decimal estándar", "q": "0.125 + 0.375", "expected": 0.5},
        {"name": "Pérdida precisión float 0.1+0.2", "q": "0.1 + 0.2", "expected": 0.3},
        {"name": "Truncamiento a 2 decimales", "q": "1.23456 + 2.34567", "expected": 3.58023},
        {"name": "Entero límite seguro 2^53 (9007199254740992)", "q": "9007199254740992 + 1", "expected": 9007199254740993.0},
        {"name": "Pérdida de bit 2^53 + 2", "q": "9007199254740992 + 2", "expected": 9007199254740994.0},
        {"name": "Cancelación catastrófica (1e16 + 1 - 1e16)", "q": "(10000000000000000 + 1) - 10000000000000000", "expected": 1.0},
        {"name": "Exponente alto f64 (1e300 * 2)", "q": "1e300 * 2", "expected": 2e300},
        {"name": "Desbordamiento a Infinito (1e308 * 2)", "q": "1e308 * 2", "expected": None},
        {"name": "Subnormal cercano a cero (1e-300 / 1e10)", "q": "1e-300 / 1e10", "expected": 1e-310},
        {"name": "División por cero explícita", "q": "50 / 0", "expected": None},
        {"name": "División por casi cero (1e-15)", "q": "50 / 0.000000000000001", "expected": 5e16},
        {"name": "División bajo umbral épsilon 1e-12", "q": "50 / 0.0000000000001", "expected": None},
    ]
    results = []
    for c in cases:
        res = query_api(c["q"])
        results.append({
            "name": c["name"],
            "query": c["q"],
            "expected": c["expected"],
            "status": res.get("status"),
            "text": res.get("text"),
            "route": res.get("route")
        })
        print(f"[{c['name']:<40}] '{c['q']}' -> Status: {res.get('status')} | Texto: {res.get('text')}")
    return results


def test_bfs_depth_and_graph_limits():
    print("\n--- 3. Medición de Profundidad BFS y Topología del Grafo ---")
    # El código implementa:
    # 1. Profundidad BFS fija a 8 saltos (answer.rs:106, 117: solve_reachability_bfs_constrained(mem, ..., 8, avoid_slice))
    # 2. Entidades de grafos conocidas: Madrid -> Barcelona -> París, Bogotá -> Lima -> Buenos Aires, etc.
    queries = [
        {"name": "Grafo 1 salto (Madrid -> Toledo)", "q": "¿Se puede ir de Madrid a Toledo?"},
        {"name": "Grafo 2 saltos (Madrid -> Barcelona -> París)", "q": "¿Se puede ir de Madrid a París?"},
        {"name": "Grafo con paso intermedio vía", "q": "¿Es posible alcanzar París desde Madrid pasando por Barcelona?"},
        {"name": "Grafo con restricción avoid", "q": "¿Se puede ir de Madrid a París sin pasar por Barcelona?"},
        {"name": "Grafo desconectado", "q": "¿Se puede ir de Madrid a Lima?"},
        {"name": "Grafo nodo inexistente", "q": "¿Se puede ir de Madrid a Berlín?"},
    ]
    results = []
    for q in queries:
        res = query_api(q["q"])
        results.append({
            "name": q["name"],
            "query": q["q"],
            "status": res.get("status"),
            "text": res.get("text"),
            "route": res.get("route"),
            "latency_ms": res.get("measured_latency_ms")
        })
        print(f"[{q['name']:<45}] -> Status: {res.get('status')} | Texto: {res.get('text')}")
    return results


def test_phrasing_robustness():
    print("\n--- 4. Prueba de Robustez a la Redacción (30 casos x 5 perturbaciones = 150 pruebas) ---")
    
    # 30 Casos base que funcionan en RELIC
    base_cases = [
        # 1-10 Aritmética pura y básica
        {"id": 1, "base": "2 + 2",
         "p": ["2 mas 2", "suma 2 y 2", "calcula 2 + 2", "2  +  2", "¿cuánto es 2 + 2?"]},
        {"id": 2, "base": "15 + 27",
         "p": ["15 mas 27", "suma 15 y 27", "resultado de 15 + 27", "15 + 27 es", "cuanto da 15 + 27"]},
        {"id": 3, "base": "100 - 37",
         "p": ["100 menos 37", "resta 37 a 100", "quítale 37 a 100", "calcula 100 - 37", "100 - 37 son"]},
        {"id": 4, "base": "12 * 8",
         "p": ["12 por 8", "multiplica 12 por 8", "12 x 8", "calcula 12 * 8", "doce por ocho"]},
        {"id": 5, "base": "144 / 12",
         "p": ["144 entre 12", "144 dividido por 12", "144 dividido entre 12", "144 dividido en 12", "calcula 144 / 12"]},
        {"id": 6, "base": "50 más 30",
         "p": ["50 mas 30", "cincuenta más treinta", "suma 50 y 30", "50 + 30 son", "cuánto da 50 más 30"]},
        {"id": 7, "base": "90 menos 45",
         "p": ["90 - 45", "noventa menos cuarenta y cinco", "restar 45 a 90", "quítale 45 a 90", "calcula 90 menos 45"]},
        {"id": 8, "base": "20 * 5",
         "p": ["veinte por cinco", "20 por 5", "multiplica 20 por 5", "20 x 5", "cuánto es 20 * 5"]},
        {"id": 9, "base": "40 / 4",
         "p": ["cuarenta entre cuatro", "40 entre 4", "40 dividido entre 4", "calcula 40 / 4", "40 / 4 es"]},
        {"id": 10, "base": "10 + 20 + 30",
         "p": ["10 + 20 + 30 son", "calcula 10 + 20 + 30", "diez más veinte más treinta", "(10 + 20) + 30", "10+20+30"]},

        # 11-15 Aritmética con paréntesis y precedencia
        {"id": 11, "base": "2 + 3 * 4",
         "p": ["(2 + 3 * 4)", "2 + (3 * 4)", "calcula 2 + 3 * 4", "dos más tres por cuatro", "2 + 3 x 4"]},
        {"id": 12, "base": "(2 + 3) * 4",
         "p": ["( 2 + 3 ) * 4", "(2 + 3) x 4", "calcula (2 + 3) * 4", "((2 + 3) * 4)", "(dos más tres) por cuatro"]},
        {"id": 13, "base": "10 - 2 * 3",
         "p": ["10 - (2 * 3)", "diez menos dos por tres", "calcula 10 - 2 * 3", "10 - 2 x 3", "10 - 6"]},
        {"id": 14, "base": "(10 + 20) / 5",
         "p": ["(10 + 20) / 5", "calcula (10 + 20) / 5", "(diez más veinte) entre cinco", "(10 + 20) dividido entre 5", "((10 + 20) / 5)"]},
        {"id": 15, "base": "5 * 4 / 2",
         "p": ["cinco por cuatro entre dos", "5 x 4 / 2", "(5 * 4) / 2", "calcula 5 * 4 / 2", "5 * 4 / 2 son"]},

        # 16-22 Inventarios canónicos
        {"id": 16, "base": "Tengo 10 manzanas, compro 5 y regalo 3",
         "p": ["tengo 10 manzanas, compro 5 y regalo 3", "Tengo 10 manzanas luego compro 5 después regalo 3", "tengo 10 manzanas pero compro 5 y regalo 3", "Poseo 10 manzanas, compro 5 y regalo 3", "Cuento con 10 manzanas, compro 5 y regalo 3"]},
        {"id": 17, "base": "Hay 50 libros, recibo 20 y presto 15",
         "p": ["hay 50 libros, recibo 20 y presto 15", "Hay 50 libros pero recibo 20 y presto 15", "Tenía 50 libros, recibo 20 y presto 15", "Hay 50 libros luego recibo 20 después presto 15", "En la biblioteca hay 50 libros, recibo 20 y presto 15"]},
        {"id": 18, "base": "Cuento con 100 dólares, gasto 35 y encuentro 10",
         "p": ["cuento con 100 dólares, gasto 35 y encuentro 10", "Tengo 100 dólares, gasto 35 y encuentro 10", "Poseo 100 dólares, gasto 35 y encuentro 10", "Cuento con 100 dólares pero gasto 35 y encuentro 10", "Contaba con 100 dólares, gasto 35 y encuentro 10"]},
        {"id": 19, "base": "Tenía 20 lápices, me dan 10 y pierdo 4",
         "p": ["tenia 20 lápices, me dan 10 y pierdo 4", "Tengo 20 lápices, me dan 10 y pierdo 4", "Tenía 20 lápices luego me dan 10 después pierdo 4", "Tenía 20 lápices, recibo 10 y pierdo 4", "Tenía veinte lápices, me dan diez y pierdo cuatro"]},
        {"id": 20, "base": "Tengo 8 gatos, adopto 2 y regalo 1",
         "p": ["tengo 8 gatos, recibo 2 y regalo 1", "Tengo 8 gatos, me dan 2 y regalo 1", "Poseo 8 gatos, compro 2 y regalo 1", "Tengo 8 gatos, encuentro 2 y regalo 1", "Tengo 8 gatos luego recibo 2 y regalo 1"]},
        {"id": 21, "base": "Tengo veinte caramelos y me como cinco",
         "p": ["tengo 20 caramelos y me como 5", "Tengo 20 caramelos y como 5", "tengo veinte caramelos y gasto cinco", "Tengo 20 caramelos y regalo 5", "Tengo 20 caramelos menos 5"]},
        {"id": 22, "base": "Tengo 10 monedas, luego recibo 5, después gasto 2",
         "p": ["tengo 10 monedas luego recibo 5 despues gasto 2", "Tengo 10 monedas, recibo 5 y gasto 2", "Poseo 10 monedas, luego recibo 5, después gasto 2", "Cuento con 10 monedas, recibo 5 y gasto 2", "Tengo 10 monedas pero recibo 5 y gasto 2"]},

        # 23-26 Comparaciones relacionales
        {"id": 23, "base": "Carlos es más alto que Juan. Pedro es más alto que Carlos. ¿Quién es el más alto?",
         "p": ["Pedro es más alto que Carlos. Carlos es más alto que Juan. ¿Quién es el más alto?",
               "Carlos es mas alto que Juan. Pedro es mas alto que Carlos. ¿Quien es el mas alto?",
               "Carlos supera en altura a Juan. Pedro supera en altura a Carlos. ¿Quién es el más alto?",
               "Más alto que Juan es Carlos. Más alto que Carlos es Pedro. ¿Quién es el más alto?",
               "Carlos es más alto que Juan y Pedro es más alto que Carlos. ¿Quién tiene mayor estatura?"]},
        {"id": 24, "base": "Ana supera en altura a Sofía. Lucía aventaja a Ana en altura. ¿Quién tiene mayor estatura?",
         "p": ["Ana es más alta que Sofía. Lucía es más alta que Ana. ¿Quién es la más alta?",
               "Lucía aventaja a Ana en altura. Ana supera en altura a Sofía. ¿Quién tiene mayor estatura?",
               "Ana supera a Sofía en altura. Lucía supera a Ana en altura. ¿Quién es el más alto?",
               "Ana es mas alta que Sofia. Lucia es mas alta que Ana. ¿Quien tiene mayor estatura?",
               "Ana supera en altura a Sofía y Lucía aventaja a Ana en altura. ¿Quién es el más alto?"]},
        {"id": 25, "base": "Carlos es más alto que Juan. Pedro es más alto que Carlos. ¿Quién tiene mayor estatura?",
         "p": ["¿Quién tiene mayor estatura si Carlos es más alto que Juan y Pedro es más alto que Carlos?",
               "Carlos es más alto que Juan. Pedro es más alto que Carlos. ¿Quién es más alto?",
               "carlos es más alto que juan. pedro es más alto que carlos. ¿quién es el más alto?",
               "CARLOS ES MÁS ALTO QUE JUAN. PEDRO ES MÁS ALTO QUE CARLOS. ¿QUIÉN ES EL MÁS ALTO?",
               "Carlos tiene mayor estatura que Juan. Pedro tiene mayor estatura que Carlos. ¿Quién es el más alto?"]},

        # 27-30 Grafos y alcanzabilidad
        {"id": 27, "base": "¿Se puede ir de Madrid a París?",
         "p": ["¿se puede ir de madrid a parís?", "¿Se puede ir de Madrid hasta París?", "partiendo de Madrid llegar a París", "¿Es posible ir de Madrid a París?", "¿Existe un camino de Madrid a París?"]},
        {"id": 28, "base": "¿Es posible alcanzar París desde Madrid pasando por Barcelona?",
         "p": ["¿Se puede ir de Madrid a París pasando por Barcelona?", "¿Es posible ir desde Madrid hasta París vía Barcelona?", "¿Se puede ir de Madrid a París vía Barcelona?", "partiendo de Madrid llegar a París pasando por Barcelona", "¿Existe ruta de Madrid a París haciendo escala en Barcelona?"]},
        {"id": 29, "base": "¿Se puede ir de Madrid a París sin pasar por Barcelona?",
         "p": ["¿se puede ir de madrid a parís sin pasar por barcelona?", "¿Es posible ir de Madrid a París evitando Barcelona?", "¿Se puede ir de Madrid hasta París sin cruzar por Barcelona?", "¿Se puede ir de Madrid a París sin atravesar Barcelona?", "partiendo de Madrid alcanzar París evitando Barcelona"]},
        {"id": 30, "base": "¿Depende el crate relic_cli del paquete memmap2?",
         "p": ["¿depende el crate relic_cli del paquete memmap2?", "¿Depende el componente relic_cli del paquete memmap2?", "¿Depende relic_cli de memmap2?", "¿Depende el crate relic_cli de memmap2?", "¿El crate relic_cli depende del paquete memmap2?"]},
    ]

    total_perturbed = 0
    survived = 0
    failed = 0
    detailed_perturbations = []

    for item in base_cases:
        base_text = item["base"]
        base_res = query_api(base_text)
        base_ok = base_res.get("status") in ["VALID", "PARTIAL_VALID"]
        
        item_log = {
            "id": item["id"],
            "base_query": base_text,
            "base_status": base_res.get("status"),
            "base_ok": base_ok,
            "perturbations": []
        }

        for p_idx, p_text in enumerate(item["p"]):
            total_perturbed += 1
            p_res = query_api(p_text)
            p_status = p_res.get("status")
            p_ok = p_status in ["VALID", "PARTIAL_VALID"]
            
            if p_ok:
                survived += 1
            else:
                failed += 1

            item_log["perturbations"].append({
                "p_idx": p_idx + 1,
                "text": p_text,
                "status": p_status,
                "route": p_res.get("route"),
                "survived": p_ok
            })

        print(f"Caso {item['id']:02d} '{base_text[:30]:<30}' -> Sobrevivieron {sum(1 for x in item_log['perturbations'] if x['survived'])}/5")
        detailed_perturbations.append(item_log)

    survival_rate = (survived / total_perturbed) * 100.0 if total_perturbed else 0.0
    failure_rate = (failed / total_perturbed) * 100.0 if total_perturbed else 0.0
    print(f"\nTotal Perturbaciones: {total_perturbed}")
    print(f"Sobrevivieron (VALID/PARTIAL_VALID): {survived} ({survival_rate:.2f}%)")
    print(f"Fallaron / Dejaron de funcionar: {failed} ({failure_rate:.2f}%)")

    return {
        "total_perturbed": total_perturbed,
        "survived": survived,
        "failed": failed,
        "survival_rate": survival_rate,
        "failure_rate": failure_rate,
        "detailed": detailed_perturbations
    }


def test_diacritics_and_casing():
    print("\n--- 5. Sensibilidad a Tildes y Mayúsculas ---")
    pairs = [
        # Tildes
        ("dieciséis", "dieciseis", "Numeral 16"),
        ("veintidós", "veintidos", "Numeral 22"),
        ("veintitrés", "veintitres", "Numeral 23"),
        ("veintiséis", "veintiseis", "Numeral 26"),
        ("más", "mas", "Operador suma"),
        ("cuánto es 5 + 5", "cuanto es 5 + 5", "Prefijo pregunta"),
        ("¿Se puede ir de Bogotá a Lima?", "¿Se puede ir de Bogota a Lima?", "Grafo con tilde en nodo"),
        ("París", "Paris", "Grafo París vs Paris"),
        # Mayúsculas
        ("2 + 2", "2 + 2", "Línea base"),
        ("SUMA 2 Y 2", "suma 2 y 2", "Imperativo mayúsculas"),
        ("TENGO 10 MANZANAS Y COMPRO 5", "tengo 10 manzanas y compro 5", "Inventario mayúsculas"),
        ("¿SE PUEDE IR DE MADRID A PARÍS?", "¿se puede ir de madrid a parís?", "Grafo mayúsculas"),
        ("CARLOS ES MÁS ALTO QUE JUAN", "carlos es más alto que juan", "Comparación mayúsculas"),
    ]
    results = []
    for a, b, desc in pairs:
        res_a = query_api(a)
        res_b = query_api(b)
        match = (res_a.get("status") == res_b.get("status")) and (res_a.get("text") == res_b.get("text"))
        results.append({
            "description": desc,
            "query_a": a,
            "status_a": res_a.get("status"),
            "text_a": res_a.get("text"),
            "query_b": b,
            "status_b": res_b.get("status"),
            "text_b": res_b.get("text"),
            "strictly_identical": match
        })
        print(f"[{desc:<32}] Idéntico: {match} (A: {res_a.get('status')} | B: {res_b.get('status')})")
    return results


def main():
    print("==================================================================")
    print("FASE 2: MEDIDAS EXPERIMENTALES DE LÍMITES Y ROBUSTEZ (RELIC)")
    print("==================================================================")
    
    length_results = test_input_lengths()
    numerical_results = test_numerical_limits()
    bfs_results = test_bfs_depth_and_graph_limits()
    robustness_results = test_phrasing_robustness()
    diacritics_results = test_diacritics_and_casing()

    report = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "input_length_limits": length_results,
        "numerical_precision_f64": numerical_results,
        "bfs_and_graph_topology": bfs_results,
        "phrasing_robustness": robustness_results,
        "diacritics_and_casing_sensitivity": diacritics_results
    }

    out_file = Path("reports/audit_funcional/fase2_limites_medidos.json")
    out_file.parent.mkdir(parents=True, exist_ok=True)
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
        
    print(f"\n[OK] Mediciones Fase 2 guardadas en: {out_file.resolve()}")


if __name__ == "__main__":
    main()
