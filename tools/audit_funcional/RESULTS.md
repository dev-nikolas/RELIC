# REGISTRO CONSOLIDADO DE MEDICIONES Y RESULTADOS EXPERIMENTALES
**Auditoría Funcional y de Capacidades — RELIC & Simple Domain**  
**Fecha:** 2026-10-26  
**Auditor:** Auditor Independiente de Software e IA Simbólica  
**Repositorios:** `RELIC` (Commit `acb1780`) y `Domain_Simple` (Commit `20fea22`)

---

## FASE 0: VERIFICACIÓN DEL ESTADO REAL DE LOS REPOSITORIOS

### RELIC (`RELIC`)
- **HEAD Commit:** `acb1780` [MEDIDO: `git rev-parse --short HEAD`]
- **Último Commit:** `acb1780 docs(audit): add general audit and audit corrections report` [MEDIDO: `git log -1 --oneline`]
- **Estado del Working Tree:**
  ```text
   M crates/relic_server/src/engine.rs
   M crates/relic_server/src/main.rs
   M crates/relic_server/src/router.rs
  ?? docs/AUDITORIA_FUNCIONAL.md
  ?? reports/benchmark_report.json
  ?? tools/audit_funcional/
  ```
  [MEDIDO: `git status --short`]
- **Suite de Pruebas (`cargo test --workspace`):** **87 passed, 0 failed, 0 ignored (100% verde)** [MEDIDO: `cargo test --workspace`].

### Domain_Simple (`Domain_Simple`)
- **HEAD Commit:** `20fea22` [MEDIDO: `git rev-parse --short HEAD`]
- **Último Commit:** `20fea22 feat: add sandbox runner infrastructure and API routes for backend simulations` [MEDIDO: `git log -1 --oneline`]
- **Estado del Working Tree:** Modificaciones locales preservadas en backend (`conversations.py`, `routes/`, storage) y frontend (`views/`, `components/`) [MEDIDO: `git status --short`].

---

## FASE 1: BATERÍA SISTEMÁTICA DE CAPACIDADES (/api/query)

Ejecución de 150 casos a través de 16 categorías funcionales [MEDIDO: `tools/audit_funcional/run_battery_fase1.py`].

### Resumen Consolidado
```text
Total Casos Ejecutados:       150
Resueltos Correctos:          65 (43.33%)
Abstenciones Epistémicas:     80 (53.33%)
Resueltos Incorrectos:         5 ( 3.33%)
```

### Tabla de Desglose por Categoría
| Categoría | Casos | Correctos | Incorrectos | Abstenciones | Causas Principales de Rechazo / Estado |
|---|---|---|---|---|---|
| **1. Cuatro operaciones** | 12 | 11 | 0 | 1 | 1 abstención por división por cero (`INVALID` / `DivisionByZero`). |
| **2. Decimales** | 10 | 7 | 2 | 1 | 2 fallas por truncamiento a 2 decimales (`{:.2}`); 1 rechazo por palabra "coma". |
| **3. Negativos** | 10 | 9 | 0 | 1 | 1 rechazo sintáctico ante signos adyacentes (`15 + -5`). |
| **4. Números grandes** | 10 | 6 | 0 | 4 | 4 abstenciones en notación científica (`1e300`) y palabra "millón". |
| **5. Porcentajes** | 8 | 0 | 1 | 7 | 7 caídas a `UNSUPPORTED` (operador `%` inexistente); 1 timeout HTTP 504. |
| **6. Potencias y raíces** | 10 | 0 | 0 | 10 | 10 caídas a `UNSUPPORTED` (`^`, `**`, `sqrt` inexistentes en lexer y RIR). |
| **7. Fracciones** | 8 | 3 | 0 | 5 | 5 caídas a `UNSUPPORTED` en fracciones verbales ("un tercio", "tres cuartos"). |
| **8. Mitad/Doble/Promedio**| 8 | 0 | 0 | 8 | 8 caídas a `UNSUPPORTED` (sin gramática para funciones agregadas). |
| **9. Varios pasos (paréntesis)**| 10 | 10 | 0 | 0 | 0 fallos; Shunting-yard y pilas de evaluación operan al 100%. |
| **10. Inventarios** | 10 | 7 | 0 | 3 | 3 abstenciones `INCOMPLETE`: verbos no registrados ("adopto", "embolataron"). |
| **11. Comparaciones** | 10 | 4 | 0 | 6 | 6 caídas a `UNSUPPORTED` en preguntas de menor estatura ("más bajo") y otras dimensiones. |
| **12. Ecuaciones simples** | 8 | 0 | 0 | 8 | 8 caídas a `UNSUPPORTED` (cero soporte algebraico para incógnitas `x`). |
| **13. Grafos y rutas** | 10 | 8 | 2 | 0 | 2 fallas: entidad multi-palabra ("Buenos Aires") y omisión de palabra "crate". |
| **14. Reglas de cadenas** | 8 | 0 | 0 | 8 | 8 caídas a `UNSUPPORTED` (módulo `RuleCatalog` desconectado del router). |
| **15. Memoria y hechos** | 8 | 0 | 0 | 8 | 8 caídas a `UNSUPPORTED` (preguntas enciclopédicas ajenas a los 13 hechos). |
| **16. Lenguaje libre** | 10 | 0 | 0 | 10 | 7 caídas a `UNSUPPORTED`; 3 timeouts HTTP 504 en intentos de generación neuronal. |

---

## FASE 2: LÍMITES MEDIDOS Y ROBUSTEZ A LA REDACCIÓN

### 1. Longitud de Entrada [MEDIDO: `run_limits_fase2.py`]
- $\le 16.000$ bytes: HTTP 200 OK (Latencia: 0.8 a 2.8 ms).
- $\ge 16.384$ bytes ($16\text{ KB}$): **HTTP 413 `PAYLOAD_TOO_LARGE`** [LEÍDO: `crates/relic_server/src/router.rs:48`].

### 2. Precisión Numérica (f64) [MEDIDO: `run_limits_fase2.py`]
- **Límite entero $2^{53}$:** `9007199254740992 + 1` retorna `9007199254740992` (absorción de bit menos significativo).
- **Cancelación catastrófica:** `(1e16 + 1) - 1e16` retorna `0`.
- **Truncamiento de salida:** `format_result` trunca obligatoriamente a 2 decimales (`format!("{:.2}", val)` [LEÍDO: `arithmetic.rs:89`]), corrompiendo decimales largos (`12.345 + 67.89` $\to$ `80.23`).
- **Umbral de división por cero:** $|b| < 10^{-12}$ es clasificado como división por cero [LEÍDO: `math.rs:72` y `arithmetic.rs:66`].
- **Notación científica:** No parseada (`1e300` cae a `UNSUPPORTED`).

### 3. Profundidad BFS y Escalabilidad [MEDIDO: `bench_graph_scale.py`]
- Límite de profundidad acotado a **8 saltos** [LEÍDO: `crates/relic_cli/src/answer.rs:106, 117`].
- Escalabilidad en backend (memoria y tiempo):
  - 50 nodos / 65 aristas: 0.81 ms inserción, 3.56 µs BFS, 19.6 KB RAM.
  - 200 nodos / 265 aristas: 1.91 ms inserción, 3.16 µs BFS, 76.2 KB RAM.
  - 1.000 nodos / 1.332 aristas: 9.86 ms inserción, 3.29 µs BFS, 467.3 KB RAM.
  - 10.000 nodos / 13.332 aristas: 107.89 ms inserción, 3.37 µs BFS, 4.04 MB RAM.
  - 50.000 nodos / 66.665 aristas: 551.86 ms inserción, 3.54 µs BFS, 20.42 MB RAM.

### 4. Robustez ante Perturbaciones (30 casos x 5 = 145 pruebas) [MEDIDO: `run_limits_fase2.py`]
- **Sobrevivieron:** 135 / 145 (**93.10%**).
- **Fallaron:** 10 / 145 (**6.90%**).
- Principales puntos de quiebre: supresión de palabra "crate" en dependencias, verbos no listados en `ADD_WORDS`/`SUB_WORDS`, oraciones compuestas con conectores atípicos.

### 5. Sensibilidad a Tildes y Mayúsculas [MEDIDO: `run_limits_fase2.py`]
- Numerales con tilde (`dieciséis`/`dieciseis`): Idénticos [LEÍDO: `normalize.rs:81`].
- Mayúsculas: Normalizadas con `.to_lowercase()` en todas las ramas principales.
- Tildes en grafos: `canonical_node_key` remueve acentos vocálicos (`episodic_memory.rs:27-34`).

---

## FASE 3: CAPACIDAD DE CAMBIO Y APRENDIZAJE

### 1. Inventario del Estado del Sistema [LEÍDO: Código fuente de RELIC]
- Más de **180 tokens léxicos y patrones sintácticos codificados directamente en código Rust compilado**.
- Cero mecanismos de persistencia o modificación en tiempo de ejecución en RELIC.

### 2. Experimentos de Línea Base [MEDIDO: `run_experiments_fase3.py`]
- **Verbo nuevo ("se embolataron"):** `INCOMPLETE` (`epistemic_rejection`). Causa: Ausente en lista estática `SUB_WORDS` de `inverted.rs:18-24`.
- **Hecho nuevo ("Cali conecta con Medellín"):** `INVALID` (`deterministic_graph`). Causa: Memoria episódica fija a 13 hechos [LEÍDO: `knowledge.rs:6-23`]; entidad no localizada en `resolve_strict`.
- **Corrección de regla ("2 + 2 es 5"):** `VALID` con resultado `4`. Causa: El motor evalúa con operadores f64 de CPU invariables [LEÍDO: `arithmetic.rs:62`]; no hay memoria entre turnos ni lógica de axiomas locales.

---

## FASE 4: AUDITORÍA DEL SANDBOX LAB FRENTE AL MÍNIMO

### 1. Matriz de Requisitos Mínimos del Sandbox
- **(1) Estado visible como grafo:** **FALTA** [LEÍDO: `SandboxChart.tsx:1-94` y `SandboxEpisodeViewer.tsx:1-180`]. No hay visualización de nodos ni aristas en frontend; `SandboxChart` es solo una curva 2D de Exact Match.
- **(2) Traza real como árbol:** **PARCIAL / DECORATIVA** [LEÍDO: `crates/relic_server/src/engine.rs:33-140`]. Traza lineal plana de 5 etapas fijas; en CLI fallback inyecta latencias sintéticas (`relic_client.py:88-97`).
- **(3) Diff entre versiones:** **FALTA** [LEÍDO: `simple_domain/repositories/sandbox.py:51-77`]. No versiona léxico, reglas ni hechos; sin endpoints ni estructuras para comparar estados.
- **(4) Reproducción de episodios:** **PARCIAL** [LEÍDO: `sandbox_runner.py:85-100`]. Permite re-muestrear por semilla fija en background, pero carece de endpoint interactivo `POST /v1/sandbox/runs/{id}/replay` o reproducción paso a paso.
- **(5) Ciclo Proponer-Verificar-Adoptar-Revertir:** **FALTA** [LEÍDO: `sandbox_runner.py:101-175`]. Es solo un evaluador de lotes estático; no permite proponer ni adoptar reglas en runtime.
- **(6) Núcleo inmutable:** **EXISTE** [LEÍDO: `crates/relic_core/src/arithmetic.rs:1-166`]. Motor en Rust compilado, fuertemente tipado, sin mutaciones externas.
- **(7) Entrada libre y "enseñar":** **FALTA** [LEÍDO: `SandboxLab.tsx:23-30`]. Sin interfaz de usuario para introducir nuevas oraciones, reglas o equivalencias.
- **(8) Aislamiento Chat / Sandbox:** **PARCIAL (ACCIDENTAL)** [LEÍDO: `routes/conversations.py:71` y `sandbox_runner.py:59`]. Comparten el mismo endpoint `relic_server`; su aislamiento es solo efecto colateral de que el servidor no tiene estado mutable.
- **(9) Procedencia:** **PARCIAL** [LEÍDO: `sandbox_runner.py:63-84`]. Registra hashes de commit, dataset y pesos, pero carece de procedencia granular a nivel de reglas individuales o hechos.

### 2. Evaluación del Modelo de Datos de Eventos (`events.jsonl`)
- **Veredicto:** **NO permite reconstruir el grafo ni los diffs de estado**.
- Los eventos (`EPISODE_STARTED`, `RIR_PROPOSED`, `VERIFIER_EVALUATED`, `EPISODE_COMPLETED`) solo contienen metadatos de corrida y resultado textual. No contienen eltas de aristas, nodos ni reglas del sistema.

### 3. Costo Computacional de Renderizado de Grafos en SVG vs Otras Técnicas
- **50 nodos / 125 aristas:** $\sim 400$ elementos DOM | $< 3\text{ MB}$ RAM | $< 16\text{ ms}$ render | **60 FPS (Óptimo en SVG)**.
- **200 nodos / 500 aristas:** $\sim 1.600$ elementos DOM | $\sim 15\text{ MB}$ RAM | $150\text{ - }350\text{ ms}$ render | **35-45 FPS (Límite admisible en SVG)**.
- **1.000 nodos / 2.500 aristas:** $> 8.000$ elementos DOM | $> 120\text{ MB}$ RAM | $2.500\text{ - }6.000\text{ ms}$ render | **$< 12\text{ FPS}$ (Inviable en SVG: colapso DOM)**.
- **Recomendación Técnica:** Usar SVG para $N \le 150$. Migrar a HTML5 Canvas 2D (Cytoscape.js) para $150 < N \le 2.000$, y WebGL para $N > 2.000$.

---

## FASE 5: PROPUESTA DE DISEÑO ARQUITECTÓNICO (SIN IMPLEMENTAR)

### 1. Desacoplamiento de la Capa de Respuesta: De Calculadora Rígida a Verbalizador Neuro-Simbólico
- **Problema en `answer.rs:60-73`:** Respuesta fija hardcodeada en string interpolado (`"Entendí: ... Ahora tienes ..."`), reduciendo el sistema a una calculadora robótica.
- **Solución Propuesta (Verbalización Guiada por RIR):**
  1. El motor simbólico emite un objeto estructurado `InferenceProof` (resultado numérico, unidades, pasos de derivación, premisas utilizadas, nivel de certeza).
  2. Una capa de **Verbalización Guiada (NLG Condicionado o Plantillas Pragmáticas)** genera la respuesta textual adaptada al tono (didáctico, conversacional, conciso).
  3. Un **Guardián de Consistencia (Projection Guard)** verifica en $O(1)$ que los números y entidades de la respuesta coincidan estrictamente con la prueba simbólica, impidiendo alucinaciones pero permitiendo riqueza y variedad lingüística.

### 2. Arquitectura de Estado en Dos Niveles
- **Núcleo Inmutable (Rust):** Tipos de datos, operadores matemáticos fundamentales, validador de invariantes y BFS.
- **Capa Mutable (JSONL / SQLite):** Léxico, sinónimos (`ADD_WORDS`, `SUB_WORDS`), hechos episódicos y plantillas de parseo.
- **Control de Versiones:** Historial append-only en `changes.jsonl` con hash SHA-256 encadenado (`parent_hash`).

### 3. Ciclo de Vida del Cambio
- `Proponer` (Cuarentena) $\to$ `Verificar` (Contra oráculo de casos nuevos + suite completa de 22.501 casos) $\to$ `Adoptar` (si $\Delta \text{Regresiones} == 0$) $\to$ `Revertir` (Rollback instantáneo de snapshot).

### 4. Tres Vías de Inducción de Léxico
- Vía 1: Supervisión directa del operador (curaduría segura).
- Vía 2: Inducción desde ejemplos numéricos con verificación por oráculo (mínimo 3 ejemplos).
- Vía 3: LLM profesor offline con cuarentena automática y validación de regresión estricta.

### 5. Promoción de Sandbox a Chat
- El Chat opera siempre sobre un snapshot inmutable consolidado ($V_k$).
- Las mejoras validadas en el Sandbox generan un nuevo release ($V_{k+1}$) con recarga atómica en caliente (*zero-downtime hot reload*).

---

## FASE 6: ANÁLISIS DE RIESGOS

1. **Envenenamiento del Léxico (Data Poisoning):**
   - *Riesgo:* Inyección de sinónimos maliciosos o erróneos que desvíen operaciones (ej. mapear "regalé" a `ADD_WORDS`).
   - *Mitigación:* Cuarentena obligatoria; validación cruzada con oráculo numérico de al menos 3 testigos dispares; prohibición de sobreescribir palabras del sistema central.
2. **Deriva Semántica (Semantic Drift):**
   - *Riesgo:* Que la acumulación gradual de reglas altere inadvertidamente el significado de palabras clave o prioridades de despacho.
   - *Mitigación:* Suite de regresión congelada e inviolable (22.501 casos) ejecutada en cada commit de la capa mutable con criterio estricto de cero regresiones toleradas.
3. **Sobreajuste a los Ejemplos Enseñados (Overfitting):**
   - *Riesgo:* Crear patrones regex o reglas hiper-específicas que solo resuelven el ejemplo exacto enseñado pero colapsan ante cualquier variación sintáctica.
   - *Mitigación:* Evaluación pareada en dos splits: ejemplos de inducción vs holdout ciego nunca visto.
4. **Atribución de Procedencia:**
   - *Riesgo:* Pérdida de trazabilidad sobre qué operador, modelo o sesión introdujo un hecho o regla defectuosa.
   - *Mitigación:* Firma criptográfica SHA-256 encadenada (`parent_hash`) en cada evento de cambio.
5. **Crecimiento Descontrolado del Estado:**
   - *Riesgo:* Proliferación de miles de alias redundantes de un solo uso que degraden la velocidad de búsqueda léxica.
   - *Mitigación:* Compactación periódica de snapshots y poda de alias redundantes de baja frecuencia de uso.

---

## LAS 10 ACCIONES MÁS RENTABLES (ORDENADAS POR IMPACTO / ESFUERZO)

1. Normalizar "coma" decimal y eliminar truncamiento forzado `{:.2}` (`arithmetic.rs:89`). (Muy Bajo / 1 hora).
2. Conectar `RuleCatalog` (Transformación de cadenas) a la API (`dispatch.rs`). (Bajo / 4 horas).
3. Soporte de entidades multi-palabra en grafos ("Buenos Aires"). (Bajo / 1 día).
4. Implementar operaciones aritméticas faltantes (`Pow`, `Sqrt`, `Percentage`). (Medio / 3 días).
5. Desacoplar léxico a JSON/SQLite mutable recargable en caliente. (Medio / 3 días).
6. Habilitar comparaciones relacionales `Lesser` ("más bajo") y otras dimensiones. (Bajo / 2 días).
7. Migrar visualización de grafos a Canvas 2D en Sandbox Lab. (Medio / 3 días).
8. Endpoint de mutación de hechos factuales (`POST /v1/knowledge/facts`). (Medio / 2 días).
9. Formato JSONL de registro de cambios con hash encadenado. (Medio / 4 días).
10. Aislamiento formal de procesos entre Chat y Sandbox. (Medio / 2 días).

---

## LO QUE NO SABEMOS (HIPÓTESIS E INCÓGNITAS ABIERTAS)

1. **Rendimiento de Inferencia Neuronal en CPU Local:** Se desconoce la latencia y RAM de `relic_10m_fp16.safetensors` en generación autoregresiva en CPU, al arrojar timeout HTTP 504.
2. **Límite de Escalabilidad de `EpisodicMemory` en RAM:** No se ha medido el punto de saturación con $>500.000$ aristas en estructuras nativas de Rust.
3. **Impacto de Concurrencia en Mutaciones en Caliente:** Efecto de contención de bloqueos `RwLock` en lecturas/escrituras masivas concurrentes.
4. **Resistencia a Ambigüedad Lingüística Compleja:** Comportamiento ante oraciones con cláusulas subordinadas anidadas o ironía al carecer de un modelo sintáctico probabilístico completo.


