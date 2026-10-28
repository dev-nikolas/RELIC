#!/usr/bin/env python3
"""
FASE 1: Matriz de Capacidades de Razonamiento Simbólico (RELIC)
Batería sistemática de pruebas a través de POST /api/query con enable_trace=True.
Evalúa: Resueltos Correctos, Resueltos Incorrectos, Abstenciones y Causa de Rechazo.
"""

import json
import time
import urllib.request
import urllib.error
from pathlib import Path
from typing import Dict, Any, List

SERVER_URL = "http://127.0.0.1:5000/api/query"

# 16 Categorías requeridas con >=8-12 casos cada una (>160 casos en total)
CASES = [
    # --- 1. Las 4 Operaciones Básicas ---
    {"cat": "1_cuatro_operaciones", "q": "2 + 2", "gold": "4", "gold_num": 4.0},
    {"cat": "1_cuatro_operaciones", "q": "15 + 27", "gold": "42", "gold_num": 42.0},
    {"cat": "1_cuatro_operaciones", "q": "100 - 37", "gold": "63", "gold_num": 63.0},
    {"cat": "1_cuatro_operaciones", "q": "12 * 8", "gold": "96", "gold_num": 96.0},
    {"cat": "1_cuatro_operaciones", "q": "144 / 12", "gold": "12", "gold_num": 12.0},
    {"cat": "1_cuatro_operaciones", "q": "cuánto es 50 más 30", "gold": "80", "gold_num": 80.0},
    {"cat": "1_cuatro_operaciones", "q": "calcula 90 menos 45", "gold": "45", "gold_num": 45.0},
    {"cat": "1_cuatro_operaciones", "q": "multiplica 7 por 6", "gold": "42", "gold_num": 42.0},
    {"cat": "1_cuatro_operaciones", "q": "suma 14 y 26", "gold": "40", "gold_num": 40.0},
    {"cat": "1_cuatro_operaciones", "q": "resta 15 a 50", "gold": "35", "gold_num": 35.0},
    {"cat": "1_cuatro_operaciones", "q": "quítale 8 a 20", "gold": "12", "gold_num": 12.0},
    {"cat": "1_cuatro_operaciones", "q": "10 / 0", "gold": "División por cero no permitida", "is_error": True},

    # --- 2. Decimales ---
    {"cat": "2_decimales", "q": "3.5 + 2.1", "gold": "5.6", "gold_num": 5.6},
    {"cat": "2_decimales", "q": "10.75 - 4.25", "gold": "6.5", "gold_num": 6.5},
    {"cat": "2_decimales", "q": "2.5 * 4", "gold": "10", "gold_num": 10.0},
    {"cat": "2_decimales", "q": "7.5 / 2.5", "gold": "3", "gold_num": 3.0},
    {"cat": "2_decimales", "q": "0.1 + 0.2", "gold": "0.3", "gold_num": 0.3},
    {"cat": "2_decimales", "q": "12.345 + 67.89", "gold": "80.235", "gold_num": 80.235},
    {"cat": "2_decimales", "q": "100.5 - 0.25", "gold": "100.25", "gold_num": 100.25},
    {"cat": "2_decimales", "q": "0.005 * 200", "gold": "1", "gold_num": 1.0},
    {"cat": "2_decimales", "q": "1.0 / 3.0", "gold": "0.33", "gold_num": 0.333333},
    {"cat": "2_decimales", "q": "tres coma cinco más dos", "gold": "5.5", "gold_num": 5.5},

    # --- 3. Negativos ---
    {"cat": "3_negativos", "q": "-5 + 8", "gold": "3", "gold_num": 3.0},
    {"cat": "3_negativos", "q": "-10 - 5", "gold": "-15", "gold_num": -15.0},
    {"cat": "3_negativos", "q": "4 - 10", "gold": "-6", "gold_num": -6.0},
    {"cat": "3_negativos", "q": "-3 * 7", "gold": "-21", "gold_num": -21.0},
    {"cat": "3_negativos", "q": "-20 / -4", "gold": "5", "gold_num": 5.0},
    {"cat": "3_negativos", "q": "15 + -5", "gold": "10", "gold_num": 10.0},
    {"cat": "3_negativos", "q": "(-8) * (-2)", "gold": "16", "gold_num": 16.0},
    {"cat": "3_negativos", "q": "menos cinco más diez", "gold": "5", "gold_num": 5.0},
    {"cat": "3_negativos", "q": "-50 + 20", "gold": "-30", "gold_num": -30.0},
    {"cat": "3_negativos", "q": "0 - 15", "gold": "-15", "gold_num": -15.0},

    # --- 4. Números Grandes ---
    {"cat": "4_numeros_grandes", "q": "1000000 + 2500000", "gold": "3500000", "gold_num": 3500000.0},
    {"cat": "4_numeros_grandes", "q": "999999999 + 1", "gold": "1000000000", "gold_num": 1000000000.0},
    {"cat": "4_numeros_grandes", "q": "5000000 * 2000", "gold": "10000000000", "gold_num": 10000000000.0},
    {"cat": "4_numeros_grandes", "q": "1000000000000 / 1000000", "gold": "1000000", "gold_num": 1000000.0},
    {"cat": "4_numeros_grandes", "q": "9007199254740991 + 1", "gold": "9007199254740992", "gold_num": 9007199254740992.0},
    {"cat": "4_numeros_grandes", "q": "1e15 + 1e15", "gold": "2e15", "gold_num": 2e15},
    {"cat": "4_numeros_grandes", "q": "un millón más dos millones", "gold": "3000000", "gold_num": 3000000.0},
    {"cat": "4_numeros_grandes", "q": "1000000000000000000 + 1", "gold": "1000000000000000001", "gold_num": 1e18},
    {"cat": "4_numeros_grandes", "q": "1e300 * 10", "gold": "1e301", "gold_num": 1e301},
    {"cat": "4_numeros_grandes", "q": "1e308 * 2", "gold": "desbordamiento", "is_error": True},

    # --- 5. Porcentajes ---
    {"cat": "5_porcentajes", "q": "cuánto es el 20% de 150", "gold": "30", "gold_num": 30.0},
    {"cat": "5_porcentajes", "q": "calcula el 50% de 80", "gold": "40", "gold_num": 40.0},
    {"cat": "5_porcentajes", "q": "15% de 200", "gold": "30", "gold_num": 30.0},
    {"cat": "5_porcentajes", "q": "100 + 10%", "gold": "110", "gold_num": 110.0},
    {"cat": "5_porcentajes", "q": "descuento del 25% a 200", "gold": "150", "gold_num": 150.0},
    {"cat": "5_porcentajes", "q": "qué porcentaje representa 25 de 100", "gold": "25%", "gold_num": 25.0},
    {"cat": "5_porcentajes", "q": "5% de 1000", "gold": "50", "gold_num": 50.0},
    {"cat": "5_porcentajes", "q": "el 0.5% de 500", "gold": "2.5", "gold_num": 2.5},

    # --- 6. Potencias y Raíces ---
    {"cat": "6_potencias_y_raices", "q": "2 ^ 8", "gold": "256", "gold_num": 256.0},
    {"cat": "6_potencias_y_raices", "q": "2 ** 10", "gold": "1024", "gold_num": 1024.0},
    {"cat": "6_potencias_y_raices", "q": "3 elevado a la 4", "gold": "81", "gold_num": 81.0},
    {"cat": "6_potencias_y_raices", "q": "5 al cuadrado", "gold": "25", "gold_num": 25.0},
    {"cat": "6_potencias_y_raices", "q": "2 al cubo", "gold": "8", "gold_num": 8.0},
    {"cat": "6_potencias_y_raices", "q": "raíz cuadrada de 144", "gold": "12", "gold_num": 12.0},
    {"cat": "6_potencias_y_raices", "q": "sqrt(81)", "gold": "9", "gold_num": 9.0},
    {"cat": "6_potencias_y_raices", "q": "raíz cúbica de 27", "gold": "3", "gold_num": 3.0},
    {"cat": "6_potencias_y_raices", "q": "10 ^ 3", "gold": "1000", "gold_num": 1000.0},
    {"cat": "6_potencias_y_raices", "q": "cuánto da 4 elevado a 0.5", "gold": "2", "gold_num": 2.0},

    # --- 7. Fracciones ---
    {"cat": "7_fracciones", "q": "1/2 + 1/4", "gold": "3/4 (0.75)", "gold_num": 0.75},
    {"cat": "7_fracciones", "q": "un tercio más un sexto", "gold": "1/2 (0.5)", "gold_num": 0.5},
    {"cat": "7_fracciones", "q": "3/5 * 10", "gold": "6", "gold_num": 6.0},
    {"cat": "7_fracciones", "q": "2/3 de 90", "gold": "60", "gold_num": 60.0},
    {"cat": "7_fracciones", "q": "5/2 - 1/2", "gold": "2", "gold_num": 2.0},
    {"cat": "7_fracciones", "q": "tres cuartos de 100", "gold": "75", "gold_num": 75.0},
    {"cat": "7_fracciones", "q": "7/8 dividido 1/4", "gold": "3.5", "gold_num": 3.5},
    {"cat": "7_fracciones", "q": "simplifica 4/8", "gold": "1/2", "gold_num": 0.5},

    # --- 8. Mitad / Doble / Promedio ---
    {"cat": "8_mitad_doble_promedio", "q": "la mitad de 50", "gold": "25", "gold_num": 25.0},
    {"cat": "8_mitad_doble_promedio", "q": "el doble de 35", "gold": "70", "gold_num": 70.0},
    {"cat": "8_mitad_doble_promedio", "q": "el triple de 15", "gold": "45", "gold_num": 45.0},
    {"cat": "8_mitad_doble_promedio", "q": "cuánto es el doble de 12 más 5", "gold": "29", "gold_num": 29.0},
    {"cat": "8_mitad_doble_promedio", "q": "el promedio de 10 y 20", "gold": "15", "gold_num": 15.0},
    {"cat": "8_mitad_doble_promedio", "q": "promedio entre 4, 8 y 12", "gold": "8", "gold_num": 8.0},
    {"cat": "8_mitad_doble_promedio", "q": "la cuarta parte de 100", "gold": "25", "gold_num": 25.0},
    {"cat": "8_mitad_doble_promedio", "q": "media aritmética de 5 y 15", "gold": "10", "gold_num": 10.0},

    # --- 9. Varios Pasos Encadenados ---
    {"cat": "9_varios_pasos", "q": "2 + 3 * 4", "gold": "14", "gold_num": 14.0},
    {"cat": "9_varios_pasos", "q": "(2 + 3) * 4", "gold": "20", "gold_num": 20.0},
    {"cat": "9_varios_pasos", "q": "10 - 2 * 3 + 4", "gold": "8", "gold_num": 8.0},
    {"cat": "9_varios_pasos", "q": "(10 + 20) / (2 + 3)", "gold": "6", "gold_num": 6.0},
    {"cat": "9_varios_pasos", "q": "5 * 4 / 2 + 1", "gold": "11", "gold_num": 11.0},
    {"cat": "9_varios_pasos", "q": "100 - (20 + 30)", "gold": "50", "gold_num": 50.0},
    {"cat": "9_varios_pasos", "q": "2 * (3 + (4 * 5))", "gold": "46", "gold_num": 46.0},
    {"cat": "9_varios_pasos", "q": "50 / 5 * 2", "gold": "20", "gold_num": 20.0},
    {"cat": "9_varios_pasos", "q": "10 + 20 + 30 + 40 - 50", "gold": "50", "gold_num": 50.0},
    {"cat": "9_varios_pasos", "q": "((2 + 2) * (3 + 3)) / 4", "gold": "6", "gold_num": 6.0},

    # --- 10. Inventarios ---
    {"cat": "10_inventarios", "q": "Tengo 10 manzanas, compro 5 y regalo 3", "gold": "12", "gold_num": 12.0},
    {"cat": "10_inventarios", "q": "Hay 50 libros, recibo 20 y presto 15", "gold": "55", "gold_num": 55.0},
    {"cat": "10_inventarios", "q": "Cuento con 100 dólares, gasto 35 y encuentro 10", "gold": "75", "gold_num": 75.0},
    {"cat": "10_inventarios", "q": "Tenía 20 lápices, me dan 10 y pierdo 4", "gold": "26", "gold_num": 26.0},
    {"cat": "10_inventarios", "q": "Poseo 8 perros y adopto 2", "gold": "10", "gold_num": 10.0},
    {"cat": "10_inventarios", "q": "De las 15 frutas que tenía, como 4 y regalo 2", "gold": "9", "gold_num": 9.0},
    {"cat": "10_inventarios", "q": "Tengo veinte caramelos y me como cinco", "gold": "15", "gold_num": 15.0},
    {"cat": "10_inventarios", "q": "Tengo 10 monedas, luego recibo 5, después gasto 2", "gold": "13", "gold_num": 13.0},
    {"cat": "10_inventarios", "q": "Tengo algunas manzanas y me regalan 4", "gold": "Rechazo epistémico (cantidad inicial desconocida)", "is_incomplete": True},
    {"cat": "10_inventarios", "q": "Tengo 10 peras y se me embolataron 3", "gold": "7", "gold_num": 7.0},

    # --- 11. Comparaciones Relacionales ---
    {"cat": "11_comparaciones", "q": "Carlos es más alto que Juan. Pedro es más alto que Carlos. ¿Quién es el más alto?", "gold": "Pedro", "rel_role": "Pedro"},
    {"cat": "11_comparaciones", "q": "Ana supera en altura a Sofía. Lucía aventaja a Ana en altura. ¿Quién tiene mayor estatura?", "gold": "Lucía", "rel_role": "Lucía"},
    {"cat": "11_comparaciones", "q": "Marta es más baja que Elena. Elena es más baja que Rosa. ¿Quién es la más alta?", "gold": "Rosa", "rel_role": "Rosa"},
    {"cat": "11_comparaciones", "q": "Juan es más rápido que Pedro. Pedro es más rápido que Luis. ¿Quién es el más rápido?", "gold": "Juan", "rel_role": "Juan"},
    {"cat": "11_comparaciones", "q": "El elefante es más pesado que el león. El león es más pesado que el gato. ¿Quién es más pesado?", "gold": "elefante", "rel_role": "elefante"},
    {"cat": "11_comparaciones", "q": "A es más alto que B. B es más alto que A. ¿Quién es más alto?", "gold": "Inconsistencia / ciclo", "is_invalid": True},
    {"cat": "11_comparaciones", "q": "Carlos es más alto que Juan. Pedro es más alto que Luis. ¿Quién es el más alto?", "gold": "Ambigüedad / desconectado", "is_ambiguous": True},
    {"cat": "11_comparaciones", "q": "Carlos no es más alto que Juan. ¿Quién es más alto?", "gold": "Negación no soportada", "is_incomplete": True},
    {"cat": "11_comparaciones", "q": "¿Es 10 mayor que 5?", "gold": "Sí", "is_rel": True},
    {"cat": "11_comparaciones", "q": "quién tiene menor estatura entre Juan y Pedro si Juan es más alto", "gold": "Pedro", "rel_role": "Pedro"},

    # --- 12. Ecuaciones Simples ---
    {"cat": "12_ecuaciones_simples", "q": "x + 5 = 12", "gold": "7", "gold_num": 7.0},
    {"cat": "12_ecuaciones_simples", "q": "2x = 10", "gold": "5", "gold_num": 5.0},
    {"cat": "12_ecuaciones_simples", "q": "3x + 4 = 19", "gold": "5", "gold_num": 5.0},
    {"cat": "12_ecuaciones_simples", "q": "x - 7 = 20", "gold": "27", "gold_num": 27.0},
    {"cat": "12_ecuaciones_simples", "q": "x / 4 = 8", "gold": "32", "gold_num": 32.0},
    {"cat": "12_ecuaciones_simples", "q": "despeja x en 5x = 40", "gold": "8", "gold_num": 8.0},
    {"cat": "12_ecuaciones_simples", "q": "halla x: 2x - 3 = 11", "gold": "7", "gold_num": 7.0},
    {"cat": "12_ecuaciones_simples", "q": "x + y = 10; x = 4; cuánto vale y", "gold": "6", "gold_num": 6.0},

    # --- 13. Grafos y Rutas ---
    {"cat": "13_grafos", "q": "¿Se puede ir de Madrid a París?", "gold": "Sí (Madrid -> Barcelona -> París, 2 saltos)", "is_graph": True, "expected_reach": True},
    {"cat": "13_grafos", "q": "¿Es posible alcanzar París desde Madrid pasando por Barcelona?", "gold": "Sí (vía Barcelona)", "is_graph": True, "expected_reach": True},
    {"cat": "13_grafos", "q": "¿Se puede ir de Madrid a París sin pasar por Barcelona?", "gold": "No", "is_graph": True, "expected_reach": False},
    {"cat": "13_grafos", "q": "¿Se puede ir de Bogotá a Buenos Aires?", "gold": "Sí (Bogotá -> Lima -> Buenos Aires, 2 saltos)", "is_graph": True, "expected_reach": True},
    {"cat": "13_grafos", "q": "¿Es imposible ir de Bogotá a París?", "gold": "Correcto: No existe ninguna ruta viable", "is_graph": True, "expected_reach": False},
    {"cat": "13_grafos", "q": "¿Depende el crate relic_cli del paquete memmap2?", "gold": "Sí (relic_cli -> relic_core -> memmap2, 2 saltos)", "is_graph": True, "expected_reach": True},
    {"cat": "13_grafos", "q": "¿Depende relic_cli de thiserror?", "gold": "Sí (relic_cli -> relic_core -> thiserror)", "is_graph": True, "expected_reach": True},
    {"cat": "13_grafos", "q": "¿Se puede ir de Marta a biblioteca?", "gold": "Sí (Marta -> estación -> biblioteca)", "is_graph": True, "expected_reach": True},
    {"cat": "13_grafos", "q": "¿Se puede ir de Tokio a Londres?", "gold": "Entidad no existe en memoria episódica", "is_graph": True, "is_invalid": True},
    {"cat": "13_grafos", "q": "¿Se puede ir de Madrid a Toledo?", "gold": "Sí (1 salto)", "is_graph": True, "expected_reach": True},

    # --- 14. Reglas de Cadenas (Transformación de Texto) ---
    {"cat": "14_reglas_cadenas", "q": "invierte la palabra hola", "gold": "aloh", "is_string_rule": True},
    {"cat": "14_reglas_cadenas", "q": "aplica reverse a 'Relic'", "gold": "cileR", "is_string_rule": True},
    {"cat": "14_reglas_cadenas", "q": "cambia mayúsculas y minúsculas de 'aBc'", "gold": "AbC", "is_string_rule": True},
    {"cat": "14_reglas_cadenas", "q": "duplica las vocales de 'relic'", "gold": "reeliic", "is_string_rule": True},
    {"cat": "14_reglas_cadenas", "q": "elimina la primera y última letra de 'computador'", "gold": "omputado", "is_string_rule": True},
    {"cat": "14_reglas_cadenas", "q": "desplaza a la izquierda 'hola'", "gold": "olah", "is_string_rule": True},
    {"cat": "14_reglas_cadenas", "q": "aplica swap_case y luego reverse a 'Python'", "gold": "NOHTYp", "is_string_rule": True},
    {"cat": "14_reglas_cadenas", "q": "regla encadenada: duplicate_vowels sobre 'sol'", "gold": "sool", "is_string_rule": True},

    # --- 15. Memoria y Hechos ---
    {"cat": "15_memoria_hechos", "q": "¿Cuál es la capital de Francia?", "gold": "París", "is_kb": True},
    {"cat": "15_memoria_hechos", "q": "El pingüino encontró una señal en Bogotá", "gold": "Fuera de dominio", "is_free": True},
    {"cat": "15_memoria_hechos", "q": "¿Quién escribió Don Quijote?", "gold": "Miguel de Cervantes", "is_kb": True},
    {"cat": "15_memoria_hechos", "q": "La velocidad de la luz en el vacío", "gold": "299792458 m/s", "is_kb": True},
    {"cat": "15_memoria_hechos", "q": "relic_core depende de serde", "gold": "Sí", "is_kb": True},
    {"cat": "15_memoria_hechos", "q": "¿Existe conexión entre nodo_A y nodo_B?", "gold": "Sí", "is_graph": True},
    {"cat": "15_memoria_hechos", "q": "¿Quién descubrió la penicilina?", "gold": "Alexander Fleming", "is_kb": True},
    {"cat": "15_memoria_hechos", "q": "¿En qué año se fundó Bogotá?", "gold": "1538", "is_kb": True},

    # --- 16. Lenguaje Libre y Generación ---
    {"cat": "16_lenguaje_libre", "q": "Hola, ¿cómo estás?", "gold": "Fuera de dominio", "is_free": True},
    {"cat": "16_lenguaje_libre", "q": "Escribe un poema sobre el cielo", "gold": "Generación neuronal", "is_gen": True},
    {"cat": "16_lenguaje_libre", "q": "Genera una historia sobre un robot lógico", "gold": "Generación neuronal", "is_gen": True},
    {"cat": "16_lenguaje_libre", "q": "Inventa un cuento corto de dragones", "gold": "Generación neuronal", "is_gen": True},
    {"cat": "16_lenguaje_libre", "q": "Explícame qué es la teoría de la relatividad", "gold": "Fuera de dominio", "is_free": True},
    {"cat": "16_lenguaje_libre", "q": "¿Por qué el cielo es azul?", "gold": "Fuera de dominio", "is_free": True},
    {"cat": "16_lenguaje_libre", "q": "Dame un consejo para ser más productivo", "gold": "Fuera de dominio", "is_free": True},
    {"cat": "16_lenguaje_libre", "q": "Traduce 'hello world' al español", "gold": "Fuera de dominio", "is_free": True},
    {"cat": "16_lenguaje_libre", "q": "¿Qué opinas de la inteligencia artificial simbólica?", "gold": "Fuera de dominio", "is_free": True},
    {"cat": "16_lenguaje_libre", "q": "Cuéntame un chiste", "gold": "Fuera de dominio", "is_free": True},
]


def query_api(query_text: str) -> Dict[str, Any]:
    req_body = json.dumps({"query": query_text, "enable_trace": True}).encode("utf-8")
    req = urllib.request.Request(
        SERVER_URL,
        data=req_body,
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    try:
        t0 = time.perf_counter()
        with urllib.request.urlopen(req, timeout=5.0) as resp:
            elapsed_ms = (time.perf_counter() - t0) * 1000.0
            data = json.loads(resp.read().decode("utf-8"))
            data["http_code"] = resp.status
            data["client_latency_ms"] = elapsed_ms
            return data
    except urllib.error.HTTPError as e:
        return {
            "http_code": e.code,
            "text": e.read().decode("utf-8", errors="replace"),
            "route": "http_error",
            "status": f"HTTP_{e.code}",
            "latency_ms": 0.0,
            "trace": []
        }
    except Exception as e:
        return {
            "http_code": 0,
            "text": str(e),
            "route": "client_error",
            "status": "CONNECTION_ERROR",
            "latency_ms": 0.0,
            "trace": []
        }


def evaluate_outcome(case: Dict[str, Any], res: Dict[str, Any]) -> Dict[str, Any]:
    text = res.get("text", "")
    route = res.get("route", "")
    status = res.get("status", "")
    
    # Clasificación inicial
    is_abstention = status in ["INCOMPLETE", "UNSUPPORTED", "AMBIGUOUS", "HTTP_413", "CONNECTION_ERROR"]
    
    # Determinar si el resultado es correcto o incorrecto
    is_correct = False
    is_incorrect = False
    cause = None
    
    if "gold_num" in case:
        import re
        nums = re.findall(r"[-+]?\d*\.?\d+", text)
        if nums and status == "VALID":
            try:
                # Extraer último número (suele ser el resultado en 'resultado es X')
                val = float(nums[-1])
                if abs(val - case["gold_num"]) < 1e-4:
                    is_correct = True
                else:
                    is_incorrect = True
                    cause = f"Discrepancia numérica: obtenido {val}, esperado {case['gold_num']}"
            except Exception:
                is_incorrect = True
                cause = "Fallo al parsear número"
        elif is_abstention:
            cause = f"Abstención del motor ({status}): {text}"
        elif case.get("is_error") and ("Error" in text or status in ["INVALID", "INCOMPLETE"]):
            is_correct = True
        else:
            is_incorrect = True
            cause = f"Respuesta no válida o errónea para cálculo numérico: {text}"
            
    elif case.get("is_graph"):
        if case.get("expected_reach") is True:
            if "Sí" in text and status == "VALID":
                is_correct = True
            else:
                is_incorrect = True
                cause = f"Ruta esperada alcanzable pero motor respondió: {text}"
        elif case.get("expected_reach") is False:
            if ("No" in text or "Correcto: No existe" in text) and status == "VALID":
                is_correct = True
            else:
                is_incorrect = True
                cause = f"Ruta esperada no alcanzable pero motor respondió: {text}"
        elif case.get("is_invalid") and status == "INVALID":
            is_correct = True
        elif is_abstention:
            cause = f"Abstención en grafo: {text}"
            
    elif case.get("rel_role"):
        if case["rel_role"].lower() in text.lower() and status in ["VALID", "PARTIAL_VALID"]:
            is_correct = True
        elif is_abstention:
            cause = f"Abstención en comparación relacional ({status}): {text}"
        else:
            is_incorrect = True
            cause = f"Entidad máxima incorrecta: {text}"
            
    elif case.get("is_incomplete") and status == "INCOMPLETE":
        is_correct = True
    elif case.get("is_invalid") and status == "INVALID":
        is_correct = True
    elif case.get("is_ambiguous") and status == "AMBIGUOUS":
        is_correct = True
    elif case.get("is_free") or case.get("is_kb") or case.get("is_string_rule"):
        if is_abstention:
            cause = f"Abstención legítima/fuera de dominio ({status})"
        elif status == "VALID":
            is_incorrect = True
            cause = f"Falsa aceptación / alucinación de validez en caso no soportado: {text}"
        else:
            cause = f"Estado: {status}"
    elif case.get("is_gen"):
        if "relic-10m" in route or "degraded_mode" in route:
            is_correct = True  # Canalizado a generación
        else:
            cause = f"Ruta de generación no activada: {route}"
            
    # Resultado estructurado
    classification = "CORRECT" if is_correct else ("INCORRECT" if is_incorrect else "ABSTENTION")
    
    return {
        "category": case["cat"],
        "query": case["q"],
        "gold": case.get("gold", ""),
        "gold_num": case.get("gold_num"),
        "status": status,
        "route": route,
        "text": text,
        "classification": classification,
        "cause": cause,
        "latency_ms": res.get("latency_ms", 0.0),
        "trace_stages_count": len(res.get("trace") or [])
    }


def main():
    print(f"=== FASE 1: BATERÍA DE CAPACIDADES RELIC ({len(CASES)} casos) ===")
    results = []
    by_category = {}
    
    for i, case in enumerate(CASES):
        res = query_api(case["q"])
        eval_item = evaluate_outcome(case, res)
        results.append(eval_item)
        
        cat = case["cat"]
        if cat not in by_category:
            by_category[cat] = {"total": 0, "correct": 0, "incorrect": 0, "abstention": 0, "causes": {}}
        
        by_category[cat]["total"] += 1
        cls = eval_item["classification"]
        if cls == "CORRECT":
            by_category[cat]["correct"] += 1
        elif cls == "INCORRECT":
            by_category[cat]["incorrect"] += 1
        else:
            by_category[cat]["abstention"] += 1
            cause = eval_item["cause"] or "Desconocida"
            by_category[cat]["causes"][cause] = by_category[cat]["causes"].get(cause, 0) + 1
            
        print(f"[{i+1:03d}/{len(CASES):03d}] [{cat[:15]:<15}] '{case['q'][:30]:<30}' -> {cls} ({eval_item['status']})")
    
    # Reporte consolidado
    report = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "total_cases": len(CASES),
        "total_correct": sum(c["correct"] for c in by_category.values()),
        "total_incorrect": sum(c["incorrect"] for c in by_category.values()),
        "total_abstention": sum(c["abstention"] for c in by_category.values()),
        "by_category": by_category,
        "detailed_results": results
    }
    
    out_path = Path("reports/audit_funcional/fase1_matriz_capacidades.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
        
    print(f"\n[OK] Reporte guardado en: {out_path.resolve()}")
    print("\nRESUMEN POR CATEGORÍA:")
    print(f"{'Categoría':<25} | {'Total':<6} | {'Correcto':<8} | {'Incorrecto':<10} | {'Abstención':<10}")
    print("-" * 70)
    for cat, data in sorted(by_category.items()):
        print(f"{cat:<25} | {data['total']:<6} | {data['correct']:<8} | {data['incorrect']:<10} | {data['abstention']:<10}")
    print("-" * 70)
    print(f"{'TOTAL':<25} | {report['total_cases']:<6} | {report['total_correct']:<8} | {report['total_incorrect']:<10} | {report['total_abstention']:<10}")


if __name__ == "__main__":
    main()
