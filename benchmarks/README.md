# RELIC — Batería de Benchmarks y Evaluación Empírica

Este directorio contiene los arneses de benchmarking nativos en Rust y scripts de evaluación empírica para auditar la memoria, latencia, alucinación cero y resistencia adversarial de RELIC.

---

## 1. Binarios Nativos de Medición (`crates/relic_benchmarks`)

| Binario | Comando de Ejecución | Fenómeno Auditado y Alcance |
| :--- | :--- | :--- |
| **`relic_benchmarks`** | `cargo run --release -p relic_benchmarks` | Suite científica consolidada de los 6 benchmarks auditados. |
| **`measure_scale`** | `cargo run --release -p relic_benchmarks --bin measure_scale` | Memoria en escala (50k hechos), heap privada, RSS y bytes por hecho. |
| **`measure_structures`** | `cargo run --release -p relic_benchmarks --bin measure_structures` | Desglose de bytes vivos por estructura con `GlobalAlloc` contador. |
| **`measure_horizon`** | `cargo run --release -p relic_benchmarks --bin measure_horizon` | BFS bidireccional de horizonte elástico, cadenas ($k=2..100$) y distractores. |
| **`measure_commit_breakdown`** | `cargo run --release -p relic_benchmarks --bin measure_commit_breakdown` | Descomposición de commit in-place y escala de regresión ($R=200..10\text{k}$). |
| **`measure_sprt_validation`** | `cargo run --release -p relic_benchmarks --bin measure_sprt_validation` | Cota de martingala de Ville sobre $\ge 10^5$ trazas y familias nulas compuestas. |
| **`measure_mquake_edits`** | `cargo run --release -p relic_benchmarks --bin measure_mquake_edits` | Edición causal continua (100 mutaciones MQuAKE) y tasa de aristas fantasma. |
| **`measure_ms_bfs_batch`** | `cargo run --release -p relic_benchmarks --bin measure_ms_bfs_batch` | Evaluación paralela de 64 mundos contrafactuales con operaciones bitwise `u64`. |
| **`measure_relic_originality`** | `cargo run --release -p relic_benchmarks --bin measure_relic_originality` | Demostración de certificados inductivos duales y divergencia contrafactual. |

---

## 2. Resultados Empíricos Consolidados

### A. Memoria Escalar y Huella en Heap (50.000 Hechos Relacionales)
*Medido con `cargo run --release -p relic_benchmarks --bin measure_scale` y `measure_structures`:*

| Métrica de Escala | Estado Inicial | Estado Optimizado (Topología `u32` + `Box<str>`) | Mejora Observada |
| :--- | :---: | :---: | :---: |
| **Private Heap Delta** | +39.59 MB | **+18.94 MB** | **-52.2%** |
| **RSS Delta** | +38.36 MB | **+17.49 MB** | **-54.4%** |
| **Consumo por Hecho** | ~830 bytes/hecho | **~378 bytes/hecho** | **-54.4%** |
| **Latencia BFS Reachability (Media)** | 1,531.54 µs | **31.14 µs** | **49.2x más rápido** |

**Desglose de Memoria Viva por Estructura (`GlobalAlloc` con 50k hechos / 100k nodos):**
- **NodeInterner (`Box<str>` + `u32`)**: ~6.64 MB (16.0%)
- **Adyacencia fwd/bwd (`Vec<Vec<u32>>`)**: ~4.96 MB (12.0%)
- **EdgeStats & Edges (`(u32, u32)` sin Strings)**: ~1.91 MB (4.6%)
- **NodeResolver (`active_nodes`, `canonical_map`)**: ~5.34 MB (12.9%)

### B. Razonamiento en Grafos: Horizonte Elástico (BFS Bidireccional vs Unidireccional)
*Medido con `cargo run --release -p relic_benchmarks --bin measure_horizon`:*

| Escenario Topológico | Métrica Unidireccional | Métrica Bidireccional | Ganancia / Comportamiento |
| :--- | :---: | :---: | :--- |
| **Cadena con 10 distractores/nodo ($k=20$)** | 200 exp, 210 ops (31.6 µs) | **20 ops** (93.1 µs) | **10.5x menos aristas exploradas** (ignora distractores) |
| **Árbol $b=4, d=8$ (87.381 nodos)** | 13.654 exp, 54.613 ops (29.2 ms) | **8 ops** (32.1 ms) | **6.826x menos operaciones** de frontera |
| **Asimétrico ($s: b=20 \to t: b=1$)** | 22 exp, 41 ops | **41 ops** | Paridad en ops (ambas convergen en el puente) |
| **Asimétrico inverso ($s: b=1 \to t: b=20$)** | **3 exp, 22 ops** | 41 ops | **Unidireccional gana** (fan-out bajo avanza directo) |

**Latencias en Cadenas $k=2..100$ (1.000 iteraciones + calentamiento, buffers reutilizables):**
- $k=2$: Mediana 8.30 µs | Media 28.45 µs | p99 39.10 µs | Desv: 468.95 µs
- $k=5$: Mediana 10.30 µs | Media 17.25 µs | p99 39.30 µs | Desv: 179.74 µs
- $k=10$: Mediana 12.70 µs | Media 25.40 µs | p99 34.80 µs | Desv: 376.27 µs
- $k=25$: Mediana 21.10 µs | Media 45.91 µs | p99 128.90 µs | Desv: 383.83 µs
- $k=50$: Mediana 41.50 µs | Media 74.63 µs | p99 197.90 µs | Desv: 513.87 µs
- $k=75$: Mediana 61.50 µs | Media 110.80 µs | p99 1.134.50 µs | Desv: 490.22 µs
- $k=100$: Mediana 87.50 µs | Media 226.94 µs | p99 5.315.90 µs | Desv: 1.252.30 µs

### C. Compuerta Transaccional: Commit In-Place y Escala de Regresión
*Medido con `cargo run --release -p relic_benchmarks --bin measure_commit_breakdown`:*

| Escala del Grafo | Total Commit Previo (Clon) | Total Commit In-Place | In-Place + Undo Setup | Oráculo | Regresión | Append + Hash |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **5.000 Hechos** | 35.80 ms | **0.514 ms (513.7 µs)** | 7.45 µs | 1.26 µs | 486.19 µs | 18.83 µs |
| **50.000 Hechos** | 302.20 ms | **0.568 ms (568.0 µs)** | 7.33 µs | 0.73 µs | 543.76 µs | 16.22 µs |
| **Mejora** | — | **530x más rápido** | **40.000x más rápido** | Estable | Estable | Estable |

**Escalabilidad de la Suite de Regresión ($R$ casos adicionales):**
- $R = 200$ casos: 2.45 ms total (**11.65 µs/caso**)
- $R = 1.000$ casos: 14.72 ms total (**14.57 µs/caso**)
- $R = 10.000$ casos: 130.12 ms total (**13.00 µs/caso**)
*(Crecimiento estrictamente lineal $O(R)$ a ~13 µs por caso de verificación determinista).*

### D. Consistencia Causal (MQuAKE) y Mundos Contrafactuales (MS-BFS)
*Medido con `measure_mquake_edits` y `measure_ms_bfs_batch`:*

- **Edición Causal Continua (100 hechos MQuAKE):**
  - Latencia media de edición con reemplazo atómico: **38.68 µs**
  - Retención de consistencia transitiva: **100.00% (100/100)**
  - Tasa de aristas fantasma (*ghost edges*): **0.00%**
- **Evaluación Paralela de 64 Mundos Contrafactuales (MS-BFS):**
  - Tiempo lote de 64 mundos simultáneos: **5.23 µs** (frente a 1.260 µs secuencial)
  - Factor de aceleración en un solo core: **241.0x** (ratio lote vs individual: **0.27x**)

### E. Resumen Ejecutivo de la Suite Científica Auditada
*Medido con `cargo run --release -p relic_benchmarks`:*

| Benchmark | Métrica Clave | Resultado Medido | Garantía Formal |
| :--- | :--- | :---: | :--- |
| **1. Polisemia Cuántica** | Accuracy / Coverage / F1 | **100.00% / 100.00% / 1.0000** | Interferencia destructiva de fase vs baseline clásico |
| **2. Razonamiento / Alucinación** | Selective Acc / Hallucination Rate | **100.00% / 0.00%** | Cero respuestas falsas ante trampas epistémicas |
| **3. Telemetría Espectral** | Beacon Recall / FPR ($P_0$) | **100.00% / 0/10 000 (cota 95% $\le$ 0.03%)** | Detección DFT Donoho-Stark y aislamiento topológico |
| **4. Aprendizaje Continuo** | CLUTRR Multi-Hop / Retención | **100.00% / 100.00%** | Cero olvido catastrófico en 200 mutaciones y rollback |
| **5. Transición Kuramoto** | AURC (Integral Risk-Coverage) | **0.0022** | Sincronización $r \to 1.0$ en régimen supercrítico |
| **6. Robustez Jitter ($P=20$)** | Distancia Donoho-Stark Media | **5.7000** | Abstención espectral determinista ante aperiodicidad |

---

## 3. Arneses Complementarios Offline (Python)

| Script | Propósito y Evaluación |
| :--- | :--- |
| [`benchmark.py`](benchmark.py) | Punto de entrada unificado y despacho de benchmarks offline. |
| [`benchmark_cognitive_primitives.py`](benchmark_cognitive_primitives.py) | Evaluación de primitivas cognitivas (aritmética exacta, transitividad). |
| [`benchmark_episodic_reasoning.py`](benchmark_episodic_reasoning.py) | Evaluación de razonamiento episódico y memoria asociativa de grafos temporales. |
| [`benchmark_llm_comparison.py`](benchmark_llm_comparison.py) | Comparativa directa frente a LLMs (exactitud, alucinación, latencia). |
| [`benchmark_cross_domain_transfer.py`](benchmark_cross_domain_transfer.py) | Pruebas de transferencia de conocimiento a dominios cruzados. |
| [`benchmark_semantic_error_matrix.py`](benchmark_semantic_error_matrix.py) | Matriz de confusión y categorización formal de causas de fallo o ambigüedad. |
