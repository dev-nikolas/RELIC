# RELIC — Batería de Benchmarks y Evaluación Empírica

Este directorio contiene los arneses de benchmarking nativos en Rust y suites empíricas para auditar memoria, latencia, alucinación cero, consistencia causal y resistencia adversarial de RELIC.

---

## 1. Binarios Nativos de Medición (`crates/relic_benchmarks`)

| Binario | Comando de Ejecución | Fenómeno Auditado y Alcance |
| :--- | :--- | :--- |
| **`relic_benchmarks`** | `cargo run --release -p relic_benchmarks --bin relic_benchmarks` | Suite científica consolidada de los 6 benchmarks auditados. |
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

## 2. Resultados Empíricos Detallados (Última Ejecución)

### A. Memoria Escalar y Huella en Heap (50.000 Hechos Relacionales)
*Auditado con `measure_scale` y `measure_structures`:*

| Métrica de Escala | Estado Inicial | Estado Baseline (`u32`) | Estado Compacto | **Medido Actual** | Mejora Total |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Private Heap Delta** | +39.59 MB | +18.94 MB | +14.96 MB | **+14.87 MB** | **-62.4%** |
| **RSS Delta** | +38.36 MB | +17.49 MB | +13.62 MB | **+13.65 MB** | **-64.4%** |
| **Consumo por Hecho** | ~830 bytes/hecho | ~378 bytes/hecho | ~299 bytes/hecho | **~297 bytes/hecho** | **-64.2%** |
| **Latencia BFS Reachability (Media)** | 1.531.54 µs | 31.14 µs | 28.46 µs | **32.71 µs** | **46.8x más rápido** |

**Desglose de Memoria Viva por Componente (50k hechos / 100k nodos):**
- Memoria Viva Motor: 41.70 MB (874.6 bytes/hecho)
- String Arena Contiguo + Offsets: 1.67 MB (4.0%)
- Tablas Hash (Buckets 2^N): 8.50 MB (20.4%)
- Adyacencia CompactAdj (Inlined): 6.10 MB (14.6%)
- Subvectores Resolutor / Interner: 8.58 MB (20.6%)
- Capacidad ociosa Vec/Hash (Slop): 16.85 MB (40.4%)
- SearchBuffers por hilo: 2.80 MB (1.37 KB por nodo tras prepare)

### B. Razonamiento en Grafos: Horizonte Elástico
*Auditado con `measure_horizon`:*

| Escenario Topológico | Métrica Unidireccional | Métrica Bidireccional | Ganancia |
| :--- | :---: | :---: | :--- |
| **b=3, k=6..10** | 300 ops (21.90 µs) | **57 ops** (5.60 µs) | **5.3x menos ops** |
| **b=4, k=6..10** | 298 ops (27.70 µs) | **54 ops** (5.40 µs) | **5.5x menos ops** |
| **b=6, k=6..10** | 345 ops (24.80 µs) | **66 ops** (4.50 µs) | **5.2x menos ops** |

**Latencias en Cadenas $k=2..100$ (Búsqueda Zero-Copy en IDs):**
- $k=2$: Mediana **0.90 µs** | p99: **2.70 µs**
- $k=5$: Mediana **1.00 µs** | p99: **3.10 µs**
- $k=10$: Mediana **1.10 µs** | p99: **3.00 µs**
- $k=25$: Mediana **1.70 µs** | p99: **5.30 µs**
- $k=50$: Mediana **2.60 µs** | p99: **16.50 µs**
- $k=75$: Mediana **3.30 µs** | p99: **10.80 µs**
- $k=100$: Mediana **4.40 µs** | p99: **6.80 µs**

**Desglose k=100 (Búsqueda en IDs vs Formateo a String):**
- Búsqueda Pura en IDs: 3.30 µs (mediana) | 16.40 µs (p99)
- Formateo a Vec<String>: 13.50 µs (mediana) | 53.80 µs (p99)
- Total Combinado: 17.20 µs (mediana) | 78.90 µs (p99)

### C. Compuerta Transaccional: Commit In-Place y Regresión Causal
*Auditado con `measure_commit_breakdown`:*

| Escala del Grafo | Total Commit In-Place Causal | In-Place + Undo Setup | Oráculo | Regresión Causal | Append + SHA256 |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **5.000 Hechos** | **47.04 µs** | 6.15 µs (13.1%) | 1.75 µs (3.7%) | 23.71 µs (50.4%) | 15.42 µs (32.8%) |
| **50.000 Hechos** | **84.71 µs** | 8.00 µs (9.4%) | 0.85 µs (1.0%) | 58.23 µs (68.7%) | 17.63 µs (20.8%) |

**Escalabilidad de Regresión (Ciega $O(R)$ vs Causal Invertida $O(|Cone|)$):**
- $R = 200$: Ciega 3.08 ms | Causal **9.86 µs** (**312.7x de aceleración**)
- $R = 1.000$: Ciega 15.57 ms | Causal **10.08 µs** (**1.545.0x de aceleración**)
- $R = 10.000$: Ciega 94.06 ms | Causal **8.77 µs** (**10.720.3x de aceleración**)

### D. Diagnóstico Espectral y Validación de Martingala ($10^5$ Trazas)
*Auditado con `measure_sprt_validation`:*
- **Trazas Evaluadas:** 100.000 secuencias sintéticas bajo hipótesis nula $\Theta_0$ (39.78 s)
- **Falsos Positivos Empíricos (FPR):**
  - SPRT A=100: 0.00563 (Cota Ville ≤ 0.05000, Cumple: true)
  - SPRT A=100: 0.00083 (Cota Ville ≤ 0.01000, Cumple: true)
- **Familias Nulas (10.000 trazas c/u):**
  - Poisson/Exponencial: 0.00% FPR
  - Ráfagas (Burst renewal): 0.00% FPR
  - Pareto (α=1.5): 0.02% FPR [CP95: 0.00%, 0.05%]
  - Mezcla bimodal: 0.00% FPR
  - Gamma (k=2): 1.53% FPR [CP95: 1.29%, 1.77%]
  - Weibull (k=1.5): 1.41% FPR [CP95: 1.18%, 1.64%]
  - Casi-periódico jitter 60%: 53.35% FPR (fuera de Θ0)

### E. Consistencia Causal (MQuAKE) y Evaluación Paralela (MS-BFS)
*Auditado con `measure_mquake_edits` y `measure_ms_bfs_batch`:*

**Edición Causal Continua (100 mutaciones MQuAKE):**
- Latencia media por edición: **82.45 µs**
- Retención de consistencia transitiva: **100.00% (100/100)**
- Tasa de aristas fantasma: **0.00% (0/100)**
- Latencia media por consulta multi-hop (3 saltos): **1.148.54 µs**

**Evaluación Paralela de 256 Mundos Contrafactuales (MS-BFS SIMD AVX2):**
- Tiempo Lote MS-BFS (256 mundos): **8.11 µs**
- Tiempo Acumulado Secuencial: 2.338.07 µs
- Tiempo Medio por Mundo Individual: 9.13 µs
- Aceleración (Speedup): **288.29x**
- Ratio Lote vs Consulta Individual: 135.00x
- Aceleración Lote vs Secuencial: 1.90x

### F. Suite de Originalidad y Certificados Epistémicos
*Auditado con `measure_relic_originality`:*
- **Certificados Inductivos Duales:**
  - Búsqueda BFS con corte inductivo: 50 µs
  - Verificación formal del certificado: 2 µs
  - Validez matemática del testigo negativo: **true**
  - Detección de certificado manipulado: **true**
- **Paralelismo de 64 Hipótesis Contrafactuales:**
  - Tiempo de evaluación simultánea: **19 µs**
  - Mundos con ruta viable: 48/64
  - Mundos con corte estricto: 16/64
  - Coincidencia exacta con álgebra booleana: **true**
- **Reemplazo Atómico Causal:**
  - Verificación arista fantasma tras mutación: 4 µs
  - Consulta tras mutación: Alcanzable = true (distancia: 3)

### G. Resumen Ejecutivo de la Suite Científica Auditada
*Auditado con `relic_benchmarks`:*

| Benchmark | Métrica Clave | Resultado Medido | Garantía Formal |
| :--- | :--- | :---: | :--- |
| **1. Polisemia Cuántica** | Accuracy / Coverage / F1 | **100.00% / 100.00% / 1.0000** | Interferencia destructiva de fase (Baseline: 0.00% / 0.00% / 0.0000) |
| **2. Razonamiento / Alucinación** | Selective Acc / Hallucination Rate | **100.00% / 0.00%** | Cero respuestas falsas (Coverage: 90.91%, Latencia: 112.68 µs / 1355.70 µs) |
| **3. Telemetría Espectral** | Beacon Recall / FPR | **100.00% / 0.00%** | Detección DFT Donoho-Stark (Latencia: 45.94 µs / 187.70 µs) |
| **4. Aprendizaje Continuo** | CLUTRR Multi-Hop / Retención | **100.00% / 100.00%** | Cero olvido catastrófico (Latencia: 14.32 µs / 58.30 µs) |
| **5. Transición Kuramoto** | AURC (Risk-Coverage) | **0.0022** | Sincronización r → 1.0 (Kc ≈ 0.25) |
| **6. Robustez Jitter** | Distancia Donoho-Stark | **5.7000** | Abstención espectral (Tolerancia: 0% del periodo, FPR: 0/10000) |

**Memoria Física Activa (RSS):** 4.99 MB

---

## 3. Ejecución de Tests y Benchmarks

### Ejecutar Todos los Benchmarks
```bash
./run_benchmarks.sh          # Linux/Mac
```

### Ejecutar Benchmarks Individuales
```bash
# Suite científica (6 benchmarks)
cargo run --release -p relic_benchmarks --bin relic_benchmarks

# Escala y memoria
cargo run --release -p relic_benchmarks --bin measure_scale

# Horizonte elástico BFS
cargo run --release -p relic_benchmarks --bin measure_horizon

# Commit in-place y regresión
cargo run --release -p relic_benchmarks --bin measure_commit_breakdown

# Validación SPRT
cargo run --release -p relic_benchmarks --bin measure_sprt_validation

# Ediciones MQuAKE
cargo run --release -p relic_benchmarks --bin measure_mquake_edits

# Estructuras de memoria
cargo run --release -p relic_benchmarks --bin measure_structures

# MS-BFS batch
cargo run --release -p relic_benchmarks --bin measure_ms_bfs_batch

# Originalidad y certificados
cargo run --release -p relic_benchmarks --bin measure_relic_originality
```

### Ejecutar Tests
```bash
./run_tests.sh              # Linux/Mac
# o
cargo test --release -p relic_core --lib
cargo test --release -p relic_core --test architecture_evolution_benchmarks
```

### Ejecutar Todo (Verificación Completa)
```bash
./run_all.sh                # Linux/Mac
run_all.bat                 # Windows
```

---

## 4. Arneses Complementarios Offline (Python)

| Script | Propósito y Evaluación |
| :--- | :--- |
| [`benchmark.py`](benchmark.py) | Punto de entrada unificado y despacho de benchmarks offline. |
| [`benchmark_cognitive_primitives.py`](benchmark_cognitive_primitives.py) | Evaluación de primitivas cognitivas (aritmética exacta, transitividad). |
| [`benchmark_episodic_reasoning.py`](benchmark_episodic_reasoning.py) | Evaluación de razonamiento episódico y memoria asociativa de grafos temporales. |
| [`benchmark_llm_comparison.py`](benchmark_llm_comparison.py) | Comparativa directa frente a LLMs (exactitud, alucinación, latencia). |
| [`benchmark_cross_domain_transfer.py`](benchmark_cross_domain_transfer.py) | Pruebas de transferencia de conocimiento a dominios cruzados. |
| [`benchmark_semantic_error_matrix.py`](benchmark_semantic_error_matrix.py) | Matriz de confusión y categorización formal de causas de fallo o ambigüedad. |
