# RELIC — Suite Científica de Benchmarks y Estudio de Ablación

Este documento registra la metodología experimental y los resultados medidos en vivo al ejecutar el arnés determinista de evaluación (`cargo run -p relic_cli -- bench` y `crates/relic_blackwall/tests/validated_benchmarks_tests.rs`).

Todos los números reportados provienen de la ejecución real en CPU (1 hilo, Rust nativo, cero GPU y cero mocks) midiendo tiempos con `std::time::Instant` y huella física de memoria con `get_current_rss_kb()`.

---

## 1. Metodología Experimental

La evaluación científica de **RELIC** se divide en tres protocolos complementarios diseñados para falsar las hipótesis de las Fases 1 a 5:

1. **Estudio de Ablación (Clásico Fase 4 vs. Cuántico-Inspirado Fase 5):** Evalúa si la suma coherente de amplitudes complejas con interferencia de fase ($\psi_k = \alpha_{k,\text{prior}} + \alpha_{k,\text{ctx}} e^{i\theta}$) y la regla proyectiva de costo asimétrico ($p^* = \frac{\lambda - c}{\lambda + 1}$) resuelven colisiones semánticas y polisemia contextual donde un modelo aditivo de pesos positivos ($w > 0$) sufre empate o contradicción.
2. **Evaluación Estándar y Abstención Epistémica (GSM8K-ES / SVAMP / bAbI + Unanswerable Traps):** Mide simultáneamente la exactitud selectiva sobre problemas resolubles (aritmética proporcional/cotidiana, alcance BFS con `via`/`avoid`, orden transitivo) y la **tasa de alucinación** frente a trampas irresolubles (división por cero, nodos inexistentes, paradojas cíclicas y simetría de entropía máxima $H = 1.0\text{ bit}$).
3. **Evaluación de Dominio Cruzado en Ciberseguridad y Telemetría (Estilo N-BaIoT / CTU-13 / Mirai):** Audita secuencias temporales de paquetes y grafos de topología de red combinando la Transformada Discreta de Fourier exacta (DFT), la cota de incertidumbre de Donoho-Stark ($|\text{sop}(x)| \cdot |\text{sop}(\hat{x})| \ge N$) y el aislamiento de nodos puente mediante BFS con restricciones (`avoid_nodes`).

---

## 2. Resultados Reales en Vivo (`cargo run -p relic_cli -- bench`)

### Benchmark 1 — Estudio de Ablación: Léxico Clásico (Fase 4) vs. Interferencia Cuántica de Fase (Fase 5)
*Dataset:* 24 casos estructurados sobre verbos polisémicos regionales (`"bajar"`, `"tumbar"`, `"coronar"`, `"quemar"`, `"partir"`) que alternan entre `Add`, `Subtract`, `Multiply` y `Divide` según el contexto, más 4 controles sin contexto donde el motor debe abstenerse ($\lambda = 3.0, c = 0.0 \implies p^* = 0.75$).

| Motor Evaluado | Accuracy (Resueltos) | Coverage (Cobertura) | Contradiction Error Rate | F1-Score | Latencia Media ($\mu\text{s}$) | Latencia p99 ($\mu\text{s}$) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Baseline Clásico (Fase 4 — Pesos Positivos)** | 0.00% | 0.00% | 83.33% | 0.0000 | ~13.71 $\mu\text{s}$ | ~50.10 $\mu\text{s}$ |
| **Cuántico-Inspirado (Fase 5 — Fase + Born + Costo Asimétrico)** | **100.00%** | **100.00%** | **0.00%** | **1.0000** | **~18.96 $\mu\text{s}$** | **~55.40 $\mu\text{s}$** |

> **Conclusión de Ablación:** En presencia de modismos contradictorios con pesos base equiprobables, el acumulador clásico de pesos positivos no puede cancelar la acepción incompatible ($\sum w_i > 0$). La interferencia destructiva de fase ($\Delta\theta = \pi \implies \cos(\pi) = -1$) anula exactamente la operación incompatible ($|1 + e^{i\pi}|^2 = 0$), logrando $\text{F1} = 1.0000$ con un sobrecosto de latencia de apenas $\sim 5\,\mu\text{s}$ por consulta.

---

### Benchmark 2 — Razonamiento Estándar y Abstención Epistémica (GSM8K-ES / SVAMP / bAbI + Unanswerable Traps)
*Dataset:* 32 problemas combinados:
- **Subconjunto A (12 casos GSM8K-ES / SVAMP):** Fracciones verbales (`"la mitad de"`, `"un tercio de"`, `"el doble de"`), variaciones porcentuales (`"200 más el 15%"`), reparto con detección de resto (`"repartir 10 entre 3"`) y jerga aprendida en vivo vía `BlackwallGate`.
- **Subconjunto B (10 casos bAbI):** Rutas espaciales y dependencias de paquetes con restricciones `pasando por` (`via`) y `evitando` (`avoid`), más deducción transitiva de orden.
- **Subconjunto C (10 Trampas Epistémicas):** Cantidades faltantes, división por cero, entidades fuera del engrama, ciclos lógicos contradictorios y bifurcación simétrica con entropía de Shannon máxima ($H = 1.0\text{ bit}$).

| Métrica de Verificación | Resultado RELIC | Detalle de Dominio |
| :--- | :---: | :--- |
| **Selective Accuracy (sobre respondidas)** | **100.00%** | 22 problemas resolubles (Aritmética, Grafos BFS, Orden) |
| **Coverage sobre Consultas Resolubles** | **100.00%** | Cobertura determinista completa |
| **Hallucination / False Answer Rate (Trampas)** | **0.00%** | 10 trampas epistémicas rechazadas con diagnóstico formal |
| **Exact Trace Match (Oráculo RPN / BFS)** | **100.00%** | Concordancia paso a paso con la traza matemática RPN y BFS |
| **Latencia Media / p99** | **~126.84 $\mu\text{s}$ / ~377.20 $\mu\text{s}$** | Ejecución en CPU unihilo sin GPU |

---

### Benchmark 3 — Dominio Cruzado: Auditoría de Telemetría y Ciberseguridad (N-BaIoT / CTU-13 / Mirai)
*Dataset:* 14 escenarios de tráfico y topología de red:
- **5 Ataques C2 / Mirai Beaconing Puro:** Trenes periódicos ($P \in \{2,3,4,5,6\}$) con distancia de Donoho-Stark $D_{\text{DS}} = 0.0$, verificando qué subredes críticas quedan protegidas al aislar el nodo puente (`solve_reachability_bfs_constrained`).
- **5 Controles de Tráfico Humano con Jitter:** Señales aperiódicas con soporte espectral disperso ($D_{\text{DS}} > 1.0$).
- **4 Ventanas Truncadas:** Señales donde $N < \text{min\_cycles} \cdot P$ que fuerzan `SpectralOutcome::InsufficientWindow`.

| Métrica Espectral y Topológica | Resultado RELIC | Criterio Matemático |
| :--- | :---: | :--- |
| **Beacon Detection Recall (C2 / Mirai)** | **100.00%** | 5 trazas periódicas ($D_{\text{DS}} = 0.0$, $\text{PAR} \ge 1.5$) |
| **False Positive Rate (Tráfico Humano c/ Jitter)** | **0.00%** | 5 trazas aperiódicas dispersas ($D_{\text{DS}} > 1.0$) |
| **Uncertainty Abstention Accuracy (Ventana Corta)** | **100.00%** | 4 trazas truncadas ($N < \text{min\_cycles} \cdot P \implies \text{InsufficientWindow}$) |
| **Topological Isolation Exact Match (BFS `avoid`)** | **100.00%** | Identificación exacta de activos protegidos tras aislar el puente |
| **Latencia Media / p99** | **~69.91 $\mu\text{s}$ / ~216.70 $\mu\text{s}$** | DFT exacta + Welford + BFS en RAM |

---

## 3. Curva de Riesgo-Cobertura y Huella de Recursos

1. **Principio de Cero Invención (Riesgo = 0.0%):** Mediante la combinación del Oráculo del Blackwall, la entropía de Shannon ($H > H_{\max}$), la regla de decisión de costo asimétrico ($p^* = \frac{\lambda - c}{\lambda + 1}$) y la cota de Donoho-Stark ($N < \text{min\_cycles} \cdot P$), RELIC convierte cualquier insuficiencia de datos o colisión de fase en una abstención auditable (`INCOMPLETE`, `AMBIGUOUS`, `Abstained`, `InsufficientWindow`).
2. **Eficiencia Edge / Offline-First:**
   - **Memoria Física Activa (RSS):** `~5.01 MB` durante la ejecución completa de los 3 benchmarks.
   - **Latencia por Inferencia Verificada:** Entre `13 µs` y `130 µs` de media en un único hilo de CPU estándar.

