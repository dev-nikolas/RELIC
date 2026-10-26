#!/usr/bin/env python3
"""
Benchmark complementario para Fase 2:
Medición de escalabilidad del grafo (50, 200, 1.000, 10.000, 50.000 nodos/aristas)
Mide: Tiempo de inserción, tiempo de BFS por consulta, y memoria con tracemalloc.
"""

import time
import tracemalloc
import json
from pathlib import Path
from collections import deque, defaultdict


class EpisodicGraphSim:
    """Implementación fiel a EpisodicMemory y BFS de RELIC."""
    def __init__(self):
        self.adj = defaultdict(list)
        self.nodes = set()
        self.edges = set()

    def insert_edge(self, u: str, v: str):
        self.nodes.add(u)
        self.nodes.add(v)
        pair = (u, v)
        if pair not in self.edges:
            self.edges.add(pair)
            self.adj[u].append(v)

    def bfs_reachability(self, start: str, target: str, max_depth: int = 8):
        if start == target:
            return True, 0, [start]
        visited = {start}
        queue = deque([(start, [start])])
        while queue:
            curr, path = queue.popleft()
            if len(path) > max_depth:
                continue
            for neighbor in self.adj.get(curr, []):
                if neighbor == target:
                    return True, len(path), path + [neighbor]
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append((neighbor, path + [neighbor]))
        return False, 0, []


def benchmark_scale():
    sizes = [50, 200, 1000, 10000, 50000]
    results = []

    for n in sizes:
        tracemalloc.start()
        g = EpisodicGraphSim()
        t0 = time.perf_counter()
        for i in range(n - 1):
            g.insert_edge(f"node_{i}", f"node_{i+1}")
            if i % 3 == 0 and i + 3 < n:
                g.insert_edge(f"node_{i}", f"node_{i+3}")
        insert_time_ms = (time.perf_counter() - t0) * 1000.0

        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        ram_kb = peak / 1024.0

        # Medir latencia de 200 consultas BFS de hasta 8 saltos
        t_bfs_0 = time.perf_counter()
        num_queries = 200
        for q in range(num_queries):
            start = f"node_{q % (n // 2)}"
            target = f"node_{(q % (n // 2)) + 6}" if (q % (n // 2)) + 6 < n else f"node_{n-1}"
            g.bfs_reachability(start, target, max_depth=8)
        avg_bfs_us = ((time.perf_counter() - t_bfs_0) * 1_000_000.0) / num_queries

        res_item = {
            "num_nodes": n,
            "num_edges": len(g.edges),
            "insert_time_ms": round(insert_time_ms, 3),
            "avg_bfs_latency_us": round(avg_bfs_us, 2),
            "peak_ram_kb": round(ram_kb, 2)
        }
        results.append(res_item)
        print(f"Nodos: {n:>6} | Aristas: {len(g.edges):>6} | Inserción: {insert_time_ms:>8.2f} ms | BFS 8 saltos: {avg_bfs_us:>7.2f} µs | RAM Pico: {ram_kb:>8.2f} KB")

    out_file = Path("reports/audit_funcional/fase2_graph_scaling.json")
    out_file.parent.mkdir(parents=True, exist_ok=True)
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2)


if __name__ == "__main__":
    benchmark_scale()
