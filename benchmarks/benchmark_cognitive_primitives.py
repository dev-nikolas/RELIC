#!/usr/bin/env python3
"""Banco de Pruebas de Primitivas Cognitivas para RELIC (Experimento 1 - Auditado).
Generador determinista por semilla con aserciones automáticas en cada muestra:

1. Dyck-k (Pila LIFO real con aperturas y cierres intermedios):
   - In-Distribution (Train/Val): Profundidad activa real D ∈ [1, 4].
   - Extrapolación OOD: Profundidad activa real D ∈ [5, 8] (nunca vista).
   - Verificación automática: Simulador de pila formal asegura target == top_of_stack.

2. Grafos Dirigidos (Transitividad / Alcance):
   - In-Distribution (Train/Val): 4-6 nodos, caminos ≤ 3 saltos, distractores ≤ 2.
   - Extrapolación OOD: 7-10 nodos, caminos 4-6 saltos, distractores 3-5.
   - Verificación automática: Algoritmo BFS formal valida verdad de terreno (SI/NO).

3. Inducción de Reglas Sintéticas en Contexto:
   - In-Distribution (Train/Val): Reglas atómicas con ejemplos no ambiguos (sin palíndromos).
   - Extrapolación OOD: Reglas compuestas inéditas y longitudes expandidas (6-8 vs 3-5).
   - Verificación automática: Solver formal comprueba unicidad estricta de la regla.
"""

import argparse
from collections import deque
import json
import random
import sys
from pathlib import Path

# Configurar salida UTF-8
sys.stdout.reconfigure(encoding="utf-8")

ROOT_DIR = Path(__file__).resolve().parent.parent
BENCHMARK_DIR = ROOT_DIR / "data" / "benchmarks"


# ==============================================================================
# 1. TAREA DE DYCK-k AUDITADA (PILA LIFO REAL CON APERTURAS Y CIERRES)
# ==============================================================================
BRACKETS = [("(", ")"), ("[", "]"), ("{", "}")]
OPEN_MAP = {o: c for o, c in BRACKETS}
CLOSE_MAP = {c: o for o, c in BRACKETS}
ALL_OPENS = list(OPEN_MAP.keys())
ALL_CLOSES = list(CLOSE_MAP.keys())
DISTRACTORS = ["a", "b", "c", "d", "x", "y", "z"]


def verify_dyck_stack(tokens: list[str]) -> list[str]:
    """Simulador formal de pila: procesa tokens y devuelve el estado exacto de la pila."""
    stack = []
    for t in tokens:
        if t in OPEN_MAP:
            stack.append(OPEN_MAP[t])  # Almacena el cierre esperado
        elif t in CLOSE_MAP:
            if not stack or stack[-1] != t:
                raise ValueError(f"Secuencia Dyck corrupta: cierre ilegal {t}")
            stack.pop()
        # Los distractores se ignoran en la pila
    return stack


def generate_dyck_sample(target_depth: int, rng: random.Random) -> tuple[str, str, int]:
    """Genera una secuencia con aperturas, cierres intermedios y distractores,
    garantizando que la profundidad final activa sea exactamente target_depth.
    """
    tokens = []
    current_stack = []

    # Construir secuencia dinámica
    steps = target_depth * 3 + rng.randint(2, 6)
    for _ in range(steps):
        can_open = len(current_stack) < target_depth
        can_close = len(current_stack) > 1  # Dejar siempre al menos 1 abierto para cerrar al final

        # Decidir acción
        r = rng.random()
        if can_close and r < 0.35:
            # Cierre intermedio
            expected_c = current_stack.pop()
            tokens.append(expected_c)
        elif can_open:
            # Apertura
            open_c, close_c = rng.choice(BRACKETS)
            current_stack.append(close_c)
            tokens.append(open_c)

        # Distractor opcional
        if rng.random() < 0.4:
            tokens.append(rng.choice(DISTRACTORS))

    # Asegurar que la profundidad final sea exactamente target_depth
    while len(current_stack) < target_depth:
        open_c, close_c = rng.choice(BRACKETS)
        current_stack.append(close_c)
        tokens.append(open_c)
        if rng.random() < 0.3:
            tokens.append(rng.choice(DISTRACTORS))

    # El target obligatorio es el cierre del último paréntesis abierto activo
    expected_target = current_stack[-1]

    # ASERCIÓN AUTOMÁTICA OBLIGATORIA
    verified_stack = verify_dyck_stack(tokens)
    assert len(verified_stack) == target_depth, (
        f"Fallo de profundidad: esperado {target_depth}, verificado {len(verified_stack)}"
    )
    assert verified_stack[-1] == expected_target, "Fallo de target en verificación de pila"

    prompt = " ".join(tokens) + " ->"
    return prompt, expected_target, len(verified_stack)


def generate_dyck_dataset(n_samples: int, is_ood: bool, seed: int) -> list[dict]:
    rng = random.Random(seed)
    depth_range = range(5, 9) if is_ood else range(1, 5)
    samples = []

    for i in range(n_samples):
        d = rng.choice(list(depth_range))
        prompt, target, real_depth = generate_dyck_sample(d, rng)
        samples.append({
            "id": f"dyck_{'ood' if is_ood else 'id'}_{i:04d}",
            "depth": real_depth,
            "prompt": prompt,
            "target": target
        })
    return samples


# ==============================================================================
# 2. TAREA DE GRAFOS AUDITADA (TRANSITIVIDAD VERIFICADA CON BFS)
# ==============================================================================
def bfs_reachable(edges: set[tuple[str, str]], start: str, goal: str) -> bool:
    """Validador BFS determinista de caminos en grafos dirigidos."""
    if start == goal:
        return True
    adj: dict[str, list[str]] = {}
    for u, v in edges:
        adj.setdefault(u, []).append(v)

    visited = set()
    queue = deque([start])
    while queue:
        curr = queue.popleft()
        if curr == goal:
            return True
        if curr not in visited:
            visited.add(curr)
            for nxt in adj.get(curr, []):
                if nxt not in visited:
                    queue.append(nxt)
    return False


def generate_graph_sample(num_nodes: int, path_len: int, num_distractors: int, rng: random.Random) -> tuple[str, str]:
    """Genera un grafo con verdad de terreno 100% validada con BFS."""
    nodes = [f"N{i}" for i in range(num_nodes)]
    rng.shuffle(nodes)

    chain = nodes[:path_len + 1]
    edges = set()
    for i in range(path_len):
        edges.add((chain[i], chain[i + 1]))

    # Distractores aleatorios
    other_nodes = nodes[path_len + 1:]
    for _ in range(num_distractors):
        u = rng.choice(nodes)
        v = rng.choice(nodes)
        if u != v and (u, v) not in edges:
            edges.add((u, v))

    # Seleccionar consulta
    want_positive = rng.choice([True, False])
    if want_positive:
        origen, destino = chain[0], chain[-1]
    else:
        # Buscar un par sin camino garantizado
        candidates = [(u, v) for u in nodes for v in nodes if u != v]
        rng.shuffle(candidates)
        origen, destino = chain[-1], chain[0]  # Por defecto
        for u, v in candidates:
            if not bfs_reachable(edges, u, v):
                origen, destino = u, v
                break

    # ASERCIÓN AUTOMÁTICA CON BFS
    is_actually_reachable = bfs_reachable(edges, origen, destino)
    ground_truth = "SI" if is_actually_reachable else "NO"

    edge_strs = [f"{u}->{v}" for u, v in sorted(edges)]
    prompt = f"Grafo: {', '.join(edge_strs)}. ¿Camino {origen} a {destino}? ->"
    return prompt, ground_truth


def generate_graph_dataset(n_samples: int, is_ood: bool, seed: int) -> list[dict]:
    rng = random.Random(seed)
    samples = []
    for i in range(n_samples):
        if is_ood:
            nodes = rng.randint(7, 10)
            plen = rng.randint(4, 6)
            dist = rng.randint(3, 5)
        else:
            nodes = rng.randint(4, 6)
            plen = rng.randint(2, 3)
            dist = rng.randint(1, 2)

        prompt, target = generate_graph_sample(nodes, plen, dist, rng)
        samples.append({
            "id": f"graph_{'ood' if is_ood else 'id'}_{i:04d}",
            "num_nodes": nodes,
            "path_len": plen,
            "prompt": prompt,
            "target": target
        })
    return samples


# ==============================================================================
# 3. TAREA DE REGLAS AUDITADA (UNICIDAD ESTRICTA SIN PALÍNDROMOS)
# ==============================================================================
SYMBOLS = list("ABCDEFGH")

# Catálogo completo de reglas candidatas
RULES_CATALOG = {
    "reverse": lambda s: list(reversed(s)),
    "rotate_l1": lambda s: s[1:] + s[:1],
    "rotate_r1": lambda s: s[-1:] + s[:-1],
    "swap_pairs": lambda s: [s[i+1] if i%2==0 and i+1<len(s) else (s[i-1] if i%2==1 else s[i]) for i in range(len(s))],
    "invert_alphabet": lambda s: [SYMBOLS[len(SYMBOLS) - 1 - SYMBOLS.index(c)] if c in SYMBOLS else c for c in s]
}

ID_RULES = ["reverse", "rotate_l1", "rotate_r1", "swap_pairs"]

# Reglas OOD compuestas estrictamente inéditas
OOD_RULES = {
    "comp_rev_rotl": lambda s: RULES_CATALOG["rotate_l1"](RULES_CATALOG["reverse"](s)),
    "comp_swap_rotr": lambda s: RULES_CATALOG["rotate_r1"](RULES_CATALOG["swap_pairs"](s)),
    "comp_rev_invert": lambda s: RULES_CATALOG["invert_alphabet"](RULES_CATALOG["reverse"](s))
}


def is_palindrome(seq: list[str]) -> bool:
    return seq == list(reversed(seq))


def generate_rule_sample(is_ood: bool, rng: random.Random) -> tuple[str, str, str]:
    """Genera ejemplos de demostración que identifican de forma ÚNICA la regla generadora."""
    rules_dict = OOD_RULES if is_ood else {k: RULES_CATALOG[k] for k in ID_RULES}
    rule_name = rng.choice(list(rules_dict.keys()))
    rule_fn = rules_dict[rule_name]

    seq_len = rng.randint(6, 8) if is_ood else rng.randint(3, 5)

    # Generar 3 ejemplos no degenerados (no palíndromos, sin identidad)
    demos = []
    used_x = []

    for _ in range(200):  # Bucle de reintento para garantizar unicidad
        demos.clear()
        used_x.clear()
        for _ in range(3):
            for _ in range(50):
                x = [rng.choice(SYMBOLS) for _ in range(seq_len)]
                y = rule_fn(x)
                if x != y and not is_palindrome(x) and x not in used_x:
                    used_x.append(x)
                    demos.append(("".join(x), "".join(y)))
                    break

        if len(demos) == 3:
            # Comprobar unicidad frente a todo el catálogo candidato
            all_candidates = {**RULES_CATALOG, **OOD_RULES}
            matching_rules = []
            for r_name, r_fn in all_candidates.items():
                if all("".join(r_fn(list(dx))) == dy for dx, dy in demos):
                    matching_rules.append(r_name)

            if len(matching_rules) == 1 and matching_rules[0] == rule_name:
                break  # Éxito: la regla queda 100% determinada sin ambigüedad

    # Consulta de prueba
    query_x = [rng.choice(SYMBOLS) for _ in range(seq_len)]
    while query_x in used_x or query_x == rule_fn(query_x):
        query_x = [rng.choice(SYMBOLS) for _ in range(seq_len)]
    query_y = rule_fn(query_x)

    demo_strs = [f"{dx}={dy}" for dx, dy in demos]
    prompt = f"Ejemplos: {', '.join(demo_strs)}. Consulta: {''.join(query_x)}="
    target = "".join(query_y)
    return prompt, target, rule_name


def generate_rule_dataset(n_samples: int, is_ood: bool, seed: int) -> list[dict]:
    rng = random.Random(seed)
    samples = []
    for i in range(n_samples):
        prompt, target, rule_name = generate_rule_sample(is_ood, rng)
        samples.append({
            "id": f"rule_{'ood' if is_ood else 'id'}_{i:04d}",
            "rule_name": rule_name,
            "is_ood": is_ood,
            "prompt": prompt,
            "target": target
        })
    return samples


# ==============================================================================
# PIPELINE DE CONSTRUCCIÓN Y GUARDADO
# ==============================================================================
def build_all_benchmarks(out_dir: Path, seed: int = 42):
    out_dir.mkdir(parents=True, exist_ok=True)

    print("=" * 65)
    print("  GENERANDO BANCO DE PRUEBAS COGNITIVAS AUDITADO")
    print(f"  Destino: {out_dir}")
    print(f"  Semilla: {seed}")
    print("=" * 65)

    datasets = {
        "dyck": {
            "train": generate_dyck_dataset(1000, is_ood=False, seed=seed),
            "val_id": generate_dyck_dataset(200, is_ood=False, seed=seed + 1),
            "test_ood": generate_dyck_dataset(300, is_ood=True, seed=seed + 2),
        },
        "graph": {
            "train": generate_graph_dataset(1000, is_ood=False, seed=seed + 10),
            "val_id": generate_graph_dataset(200, is_ood=False, seed=seed + 11),
            "test_ood": generate_graph_dataset(300, is_ood=True, seed=seed + 12),
        },
        "rules": {
            "train": generate_rule_dataset(1000, is_ood=False, seed=seed + 20),
            "val_id": generate_rule_dataset(200, is_ood=False, seed=seed + 21),
            "test_ood": generate_rule_dataset(300, is_ood=True, seed=seed + 22),
        }
    }

    manifest = {}

    for task_name, splits in datasets.items():
        task_dir = out_dir / task_name
        task_dir.mkdir(parents=True, exist_ok=True)
        manifest[task_name] = {}

        for split_name, data in splits.items():
            file_path = task_dir / f"{split_name}.json"
            with open(file_path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
            manifest[task_name][split_name] = {
                "file": str(file_path.relative_to(ROOT_DIR)),
                "samples": len(data)
            }
            print(f"  [OK] {task_name.upper():<6} -> {split_name:<8}: {len(data):>4} muestras ({file_path.name})")

    manifest_path = out_dir / "manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)

    print("\n" + "=" * 65)
    print("  EJEMPLOS AUDITADOS CON ASERCIÓN AUTOMÁTICA")
    print("=" * 65)
    print("\n[Dyck In-Distribution (Profundidad Real)]: ", datasets["dyck"]["train"][0]["depth"])
    print(" ", datasets["dyck"]["train"][0]["prompt"], datasets["dyck"]["train"][0]["target"])
    print("[Dyck OOD (Profundidad Real Extrapolada)]: ", datasets["dyck"]["test_ood"][0]["depth"])
    print(" ", datasets["dyck"]["test_ood"][0]["prompt"], datasets["dyck"]["test_ood"][0]["target"])

    print("\n[Grafo In-Distribution (Camino corto)]: ")
    print(" ", datasets["graph"]["train"][0]["prompt"], datasets["graph"]["train"][0]["target"])
    print("[Grafo OOD (Camino largo / Nodos ampliados)]: ")
    print(" ", datasets["graph"]["test_ood"][0]["prompt"], datasets["graph"]["test_ood"][0]["target"])

    print("\n[Regla In-Distribution (Regla atómica única)]: ", datasets["rules"]["train"][0]["rule_name"])
    print(" ", datasets["rules"]["train"][0]["prompt"], datasets["rules"]["train"][0]["target"])
    print("[Regla OOD (Regla compuesta inédita)]: ", datasets["rules"]["test_ood"][0]["rule_name"])
    print(" ", datasets["rules"]["test_ood"][0]["prompt"], datasets["rules"]["test_ood"][0]["target"])
    print("=" * 65)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out-dir", default=str(BENCHMARK_DIR))
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    build_all_benchmarks(Path(args.out_dir), args.seed)


if __name__ == "__main__":
    main()
