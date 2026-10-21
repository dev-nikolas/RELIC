"""Módulo de Memoria Episódica Estructurada para RELIC.

Implementa estructuras de datos explícitas y deterministas para almacenamiento
y recuperación de hechos relacionales (grafos, tripletas y transformaciones).
Separa limpiamente el almacenamiento de hechos del razonamiento sobre ellos.
"""

from collections import defaultdict, deque
from typing import Any


class GraphEpisodicMemory:
    """Memoria episódica explícita para hechos relacionales en grafos.
    Almacena aristas dirigidas y permite consultas deterministas indexadas O(1).
    """
    def __init__(self):
        self._adj: dict[str, list[str]] = defaultdict(list)
        self._edges: set[tuple[str, str]] = set()
        self.query_count = 0

    def edge_count(self) -> int:
        return len(self._edges)

    def node_count(self) -> int:
        return len(self._adj)

    def clear(self):
        self._adj.clear()
        self._edges.clear()
        self.query_count = 0

    def insert_edge(self, u: str, v: str):
        """Registra un hecho relacional: u -> v."""
        if (u, v) not in self._edges:
            self._edges.add((u, v))
            self._adj[u].append(v)

    def load_from_sequence(self, text: str):
        """Parsea hechos en formato 'Grafo: N0->N1, N1->N5. ¿Camino N2 a N1? ->' e indexa la memoria."""
        if ". ¿Camino " in text:
            edges_part = text.split(". ¿Camino ")[0].replace("Grafo: ", "").strip()
        else:
            edges_part = text

        tokens = [t.strip(" ,.;") for t in edges_part.replace(",", " ").split()]
        for tok in tokens:
            if "->" in tok and not tok.endswith("?"):
                parts = tok.split("->")
                if len(parts) == 2 and parts[0] and parts[1]:
                    self.insert_edge(parts[0], parts[1])

    def get_neighbors(self, node: str) -> list[str]:
        """Consulta direccionable: ¿qué nodos son alcanzables directamente desde node?"""
        self.query_count += 1
        return list(self._adj.get(node, []))

    def has_edge(self, u: str, v: str) -> bool:
        self.query_count += 1
        return (u, v) in self._edges

    def solve_bfs_ground_truth(self, start: str, target: str) -> tuple[bool, int, list[str]]:
        """Techo teórico determinista: resuelve si existe camino y retorna (alcanzable, longitud, camino)."""
        if start == target:
            return True, 0, [start]
        visited = {start}
        queue = deque([(start, [start])])
        while queue:
            curr, path = queue.popleft()
            for neighbor in self._adj.get(curr, []):
                if neighbor == target:
                    return True, len(path), path + [neighbor]
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append((neighbor, path + [neighbor]))
        return False, -1, []


class RuleEpisodicCatalog:
    """Catálogo episódico para transformaciones y reglas de cadenas."""
    def __init__(self):
        self._rules: dict[str, Any] = {}
        self.query_count = 0

    def register_rule(self, name: str, func):
        self._rules[name] = func

    def apply(self, name: str, arg: str) -> str | None:
        self.query_count += 1
        if name in self._rules:
            return self._rules[name](arg)
        return None
