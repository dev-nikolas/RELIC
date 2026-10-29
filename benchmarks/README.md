# RELIC — Batería de Benchmarks y Evaluación Empírica

Este directorio contiene los arneses de benchmarking nativos en Rust y suites empíricas para auditar memoria, latencia, alucinación cero, consistencia causal y resistencia adversarial de RELIC.

---

## 1. Binarios Nativos de Medición (`crates/relic_benchmarks`)

| Binario | Comando de Ejecución | Fenómeno Auditado y Alcance |
| :--- | :--- | :--- |
| **`relic_benchmarks`** | `cargo run --release -p relic_benchmarks` | Suite científica consolidada de los 6 benchmarks auditados. |
| **`measure_scale`** | `cargo run --release -p relic_benchmarks --bin measure_scale` | Memoria en escala (50k hechos), heap privada, RSS y bytes por hecho. |
| **`measure_structures`** | `cargo run --release -p relic_benchmarks --bin measure_structures` | Desglose de bytes vivos por estructura con `GlobalAlloc` contador. |
| **`measure_horizon`** | `cargo run --release -p relic_benchmarks --bin measure_horizon` | BFS bidireccional, cadenas ($k=2..100$), árboles y distractores. |
| **`measure_commit_breakdown`** | `cargo run --release -p relic_benchmarks --bin measure_commit_breakdown` | Commit in-place vs clonación, desglose microsegundo y escala $R=200..10\text{k}$. |
| **`measure_sprt_validation`** | `cargo run --release -p relic_benchmarks --bin measure_sprt_validation` | Cota de martingala de Ville sobre $10^5$ trazas y nulos no estacionarios. |
| **`measure_sprt_grid`** | `cargo run --release -p relic_benchmarks --bin measure_sprt_grid` | Barrido paramétrico $(\alpha, \beta, \gamma)$ para el detector espectral. |
| **`measure_mquake_edits`** | `cargo run --release -p relic_benchmarks --bin measure_mquake_edits` | Edición causal continua (100 mutaciones MQuAKE) y tasa aristas fantasma. |
| **`measure_ms_bfs_batch`** | `cargo run --release -p relic_benchmarks --bin measure_ms_bfs_batch` | Evaluación paralela de 256 mundos contrafactuales con bitwise SIMD AVX2. |
| **`measure_relic_originality`** | `cargo run --release -p relic_benchmarks --bin measure_relic_originality` | Certificados inductivos duales, resistencia a manipulación y divergencia. |
| **`measure_semantic_eval`** | `cargo run --release -p relic_benchmarks --bin measure_semantic_eval` | Generalización OOD léxica y proposición de RIR sobre slots no vistos. |

---

## 2. Resultados Empíricos Detallados

### A. Memoria Escalar y Huella en Heap (50.000 Hechos Relacionales)
*Auditado con `measure_scale` y `measure_structures` (medición `GlobalAlloc` en Rust 1.85+):*

| Métrica de Escala | Estado Inicial | Estado Baseline (`u32`) | Estado Compacto (`CompactAdj` + Arena) | Mejora Total |
| :--- | :---: | :---: | :---: | :---: |
| **Private Heap Delta** | +39.59 MB | +18.94 MB | **+14.96 MB** | **-62.2%** |
| **RSS Delta** | +38.36 MB | +17.49 MB | **+13.62 MB** | **-64.5%** |
| **Consumo por Hecho** | ~830 bytes/hecho | ~378 bytes/hecho | **~299 bytes/hecho** | **-64.0%** |
| **Latencia BFS Reachability (Media)** | 1.531.54 µs | 31.14 µs | **28.46 µs** | **53.8x más rápido** |

**Desglose de Memoria Viva por Componente Interno (50.000 hechos / 100.000 nodos):**
- **NodeInterner (String Arena Contiguo + Offsets)**: ~2.82 MB (interning en arena continua, 0 `Box<str>` dispersos).
- **Adyacencia CompactAdj (Inlining $\le 4$ vecinos)**: ~1.60 MB (elimina 200.000 cabeceras `Vec<u32>` en heap).
- **EdgeStats & Edges (`(u32, u32)`)**: ~1.91 MB (pesos de refuerzo y frecuencias).
- **NodeResolver (`active_nodes`, `canonical_map`)**: ~5.34 MB (tablas hash directas).
- **SearchBuffer (Thread-Local Reutilizable)**: ~0.08 MB (0 asignaciones de heap en inferencia).
*(Huella neta en heap: +14.96 MB medidos vía `GlobalAlloc` / `measure_scale`).*

### B. Razonamiento en Grafos: Horizonte Elástico (BFS Bidireccional vs Unidireccional)
*Auditado con `measure_horizon` sobre topologías complejas y cadenas escalares $k=2..100$:*

| Escenario Topológico | Métrica Unidireccional | Métrica Bidireccional | Ganancia / Garantía Formal |
| :--- | :---: | :---: | :--- |
| **Cadena con 10 distractores ($k=20$)** | 200 exp, 210 ops (31.6 µs) | **20 ops** (93.1 µs) | **10.5x menos aristas** (inmune a ramificación espuria) |
| **Árbol $b=4, d=8$ (87.381 nodos)** | 13.654 exp, 54.613 ops (29.2 ms) | **8 ops** (< 2 µs pura / 32.1 ms setup) | **6.826x menos ops** (búsqueda instantánea; 30 ms de setup del árbol) |
| **Asimétrico ($s: b=20 \to t: b=1$)** | 22 exp, 41 ops | **41 ops** | Paridad de operaciones (convergencia en cuello) |
| **Asimétrico inverso ($s: b=1 \to t: b=20$)** | **3 exp, 22 ops** | 41 ops | **Unidireccional óptimo** (fan-out bajo avanza directo) |

**Latencias en Cadenas $k=2..100$ (1.000 iteraciones con buffers pre-asignados y zero-copy en IDs `solve_reachability_bfs_ids`):**
- $k=2$: Mediana **2.40 µs** | p99: **5.70 µs**
- $k=5$: Mediana **3.00 µs** | p99: **11.60 µs**
- $k=10$: Mediana **4.80 µs** | p99: **31.60 µs**
- $k=25$: Mediana **10.40 µs** | p99: **28.00 µs**
- $k=50$: Mediana **18.30 µs** | p99: **235.10 µs**
- $k=75$: Mediana **25.00 µs** | p99: **270.40 µs** (desaparece el pico previo de $1.134\ \mu\text{s}$ al evitar asignaciones de String)
- $k=100$: Mediana **27.40 µs** | p99: **88.80 µs** (búsqueda pura zero-copy; total combinado formateado: 43.90 µs med / 136.30 µs p99).

> *Validación Fase 1:* El pico p99 en $k=100$ colapsó de $5.315\ \mu\text{s}$ (5.3 ms) a **88.80 µs** gracias al Generational Visited Array $O(1)$ y al bypass de alocaciones de String vía `solve_reachability_bfs_ids`.

### C. Compuerta Transaccional: Commit In-Place y Regresión Causal Invertida
*Auditado con `measure_commit_breakdown` (oráculo determinista + rollback seguro `UndoGuard` + `CausalDependencyIndex`):*

| Escala del Grafo | Total Commit Previo (Clon) | Total Commit In-Place Causal | In-Place + Undo Setup | Oráculo | Regresión Causal | Append + Hash |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **5.000 Hechos** | 35.80 ms | **51.75 µs (0.052 ms)** | 6.52 µs | 1.94 µs | **26.58 µs** | 16.72 µs |
| **50.000 Hechos** | 302.20 ms | **89.06 µs (0.089 ms)** | 8.25 µs | 0.75 µs | **62.65 µs** | 17.42 µs |
| **Mejora** | — | **3.393x más rápido** | **36.000x más rápido** | O(1) | O(|Cone|) | O(1) |

- **Escalabilidad de la Suite de Regresión (Ciega $O(R)$ vs Causal Invertida $O(|Cone|)$):**
  - $R = 200$ casos: Ciega 2.43 ms | Causal **10.90 µs** (**223.3x de aceleración**).
  - $R = 1.000$ casos: Ciega 31.89 ms | Causal **18.73 µs** (**1.702.6x de aceleración**).
  - $R = 10.000$ casos: Ciega 266.26 ms | Causal **35.98 µs** (**7.400.4x de aceleración**).

### D. Diagnóstico Espectral y Validación de Martingala ($10^5$ Trazas)
*Auditado con `measure_sprt_validation` sobre procesos estocásticos no estacionarios:*
- **Trazas Evaluadas:** $100.000$ secuencias sintéticas bajo hipótesis nula $\Theta_0$.
- **Falsos Positivos Empíricos (FPR):** **0 / 100.000** (cota formal Clopper-Pearson 95%: $\le 0.003\%$).
- **Invarianza ante Familias Nulas Compuestas:** Verificación exacta de la cota de Ville $\mathbb{P}(\exists t: M_t \ge 1/\alpha) \le \alpha$.
- **Tiempo por Decisión SPRT:** **0.31 µs** por paso espectral con convergencia acotada.

### E. Consistencia Causal (MQuAKE) y Evaluación Paralela (MS-BFS)
*Auditado con `measure_mquake_edits` y `measure_ms_bfs_batch`:*
- **Edición Causal Continua (100 mutaciones MQuAKE consecutivas):**
  - Latencia media de sustitución atómica: **38.68 µs**.
  - Retención de consistencia transitiva: **100.00% (100/100)**.
  - Tasa de aristas fantasma (*ghost edges* de hechos obsoletos): **0.00%**.
- **Evaluación Paralela de 256 Mundos Contrafactuales (MS-BFS SIMD AVX2):**
  - Tiempo por lote de 256 mundos: **8.55 µs** (frente a 2.172 µs secuencial tradicional).
  - Factor de aceleración en un solo núcleo: **254.1x** (ratio lote vs mundo individual: **1.01x**).

### F. Suite de Originalidad y Certificados Epistémicos Duales
*Auditado con `measure_relic_originality`:*
- **Certificados Inductivos Duales:**
  - Caso alcanzable: Testigo de camino estricto verificado en $O(k)$ pasos (**100% validez**).
  - Caso inalcanzable: Corte inductivo $(S, V \setminus S)$ sin aristas salientes (**100% validez**).
- **Inviolabilidad Criptográfica de la Cadena de Hechos:**
  - Detección inmediata de mutaciones espurias en el log: **100.00%** de ataques detectados por SHA-256.
- **Bifurcación Contrafactual Estricta:**
  - Modificación de premisa $A \to B$ aislada en mundo hipotético sin alterar el grafo de producción.

### G. Resumen Ejecutivo de la Suite Científica Auditada
*Auditado con `relic_benchmarks` (`cargo run --release -p relic_benchmarks`):*

| Benchmark | Métrica Clave | Resultado Medido | Garantía Formal |
| :--- | :--- | :---: | :--- |
| **1. Polisemia Cuántica** | Accuracy / Coverage / F1 | **100.00% / 100.00% / 1.0000** | Interferencia destructiva de fase vs baseline clásico |
| **2. Razonamiento / Alucinación** | Selective Acc / Hallucination Rate | **100.00% / 0.00%** | Cero respuestas falsas ante trampas epistémicas |
| **3. Telemetría Espectral** | Beacon Recall / FPR ($P_0$) | **100.00% / 0/10 000 ($\le 0.03\%$)** | Detección DFT Donoho-Stark y aislamiento topológico |
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
