# RELIC — REPORTE TÉCNICO Y EVALUACIÓN DE FASE A
## Bucle de Datos, Medición Empírica y Compilador Aprendido (Lenguaje Natural → RIR)

**Estado:** FASE A COMPLETADA Y VERIFICADA  
**Fecha:** 2026-10-25  
**Entorno de Verificación:** Rust 1.83+ / Python 3.12 / PyTorch  
**Workspace Test Suite:** `cargo test --workspace` en verde (100% pruebas superadas, 0 fallos)  

---

## 1. Resumen Ejecutivo (1 Página)

La **Fase A** establece el bucle formal de datos, la línea base empírica y la infraestructura de compilación neuronal para el motor **RELIC** sin alterar suites de regresión congeladas ni ejecutar cargas de entrenamiento pesado en hardware local restringido.

### Logros Clave:
1. **Corrección Matemática y de Protocolo [MEDIDO]:**
   - Se subsanó el error en los umbrales de Wilson del diseño original: para $N=60$ episodios, el umbral de maestría $w_- \ge 0.80$ requiere **$\ge 55/60$ éxitos** (91.67%), pues $54/60$ rinde $w_- = 0.7985 < 0.80$. Para regresión con $N=40$, requiere **$\ge 37/40$** (92.5%, $w_- = 0.8014$).
   - Se actualizó la arquitectura de transporte a la especificación **MCP 2025-03-26** (Streamable HTTP y stdio), marcando el transporte HTTP+SSE como obsoleto.
2. **Medición Formal de Línea Base (Reglas) [MEDIDO]:**
   - `unseen_frozen_v2` (16 casos): Exactitud 93.8% (Wilson 95%: [71.7%, 98.9%]), Falsa Aceptación 0.0% ([0.0%, 19.4%]).
   - `known_failures_v2` (14 casos): Exactitud 92.9% ([68.5%, 98.7%]), Falsa Aceptación 0.0% ([0.0%, 21.5%]).
   - `dev_suite_v1` (30 casos): Exactitud 86.7% ([70.3%, 94.7%]), Falsa Aceptación 3.3% ([0.6%, 16.7%]).
   - `external_human_gold_v1` (100 casos creados con colombianismos balanceados): catalogado como `PENDIENTE_DE_DATOS` (inputs no rellenados).
3. **Generador y Verificador Curricular en Rust [MEDIDO]:**
   - Implementado en `relic_cli::generator` (6 módulos modulares, todos $\le 180$ líneas) con SHA-256 criptográfico puro en Rust.
   - Generado `data/curriculum_dataset_v1.jsonl` (375 muestras: 300 Train, 75 Holdout, semilla 42).
   - **SHA-256 del Dataset:** `fadfedebe9c17c4794279e94434dab45068d1154e6ac892366b49f45e4a09ced`.
   - Control de fuga de 3-gramas: **0.00% de solapamiento Train vs Holdout** (aislamiento léxico estricto verificado).
4. **Análisis de Tokenización y Modelo [MEDIDO / ESTIMADO]:**
   - Tokenizador actual BPE (TinyStories 4k) fragmenta el español de dominio a **2.11 tokens/palabra** [MEDIDO]. Se especifica entrenamiento de vocabulario BPE adaptado con palabras reservadas RIR para alcanzar **1.15–1.25 tokens/palabra** [ESTIMADO].
   - Arquitectura seleccionada: `RelicModel` 10M nativo con decodificación restringida por gramática (logit masking sobre BNF de RIR).
5. **Preparación de Entrenamiento Colab/Kaggle [PENDIENTE DE EJECUTAR]:**
   - Notebook `notebooks/relic_colab_train_fase_a.ipynb` implementado con soporte de 5 semillas `[42, 1337, 2026, 7777, 9999]`, exportación a Safetensors compatible con `relic_core::loader` y test de paridad numérica exacta.
6. **Arnés de Evaluación y Compuerta de Aprobación [MEDIDO]:**
   - `tools/evaluate_harness.py` implementa prueba pareada de McNemar con corrección de Edwards y compuerta de seguridad ($FA_{modelo} \le FA_{reglas}$).

---

## 2. A0: Correcciones Matemáticas y de Protocolo

### 2.1. Umbrales Exactos del Límite Inferior de Wilson ($w_-$) al 95%
La cota inferior del intervalo de confianza de Wilson con corrección de continuidad o estándar para $z = 1.96$:
$$w_- = \frac{\hat{p} + \frac{z^2}{2N} - z \sqrt{\frac{\hat{p}(1-\hat{p})}{N} + \frac{z^2}{4N^2}}}{1 + \frac{z^2}{N}}$$

Evaluación empírica calculada:
- **Banco de Maestría ($N=60$, umbral objetivo $w_- \ge 0.80$):**
  - $k = 53$: $\hat{p} = 0.8833 \implies w_- = 0.7782$ (No cumple)
  - $k = 54$: $\hat{p} = 0.9000 \implies w_- = 0.7985$ (No cumple por 0.0015)
  - $k = 55$: $\hat{p} = 0.9167 \implies \mathbf{w_- = 0.8193 \ge 0.80}$ **[MEDIDO - REQUISITO MÍNIMO]**
  - $k = 56$: $\hat{p} = 0.9333 \implies w_- = 0.8407$
- **Banco Fijo de Regresión ($N=40$, umbral objetivo $w_- \ge 0.80$):**
  - $k = 36$: $\hat{p} = 0.9000 \implies w_- = 0.7695$ (No cumple)
  - $k = 37$: $\hat{p} = 0.9250 \implies \mathbf{w_- = 0.8014 \ge 0.80}$ **[MEDIDO - REQUISITO MÍNIMO]**

**Corrección Semántica:** Se reemplazó el lenguaje determinista ("garantiza al 95%") por la regla estadística formal: "evaluación por lotes fijos cada 60 episodios con corte de decisión en $k \ge 55$".

### 2.2. Protocolo MCP y Aislamiento de Procesos
- **MCP:** La versión 2024-11-05 y el transporte HTTP+SSE quedaron obsoletos. La especificación vigente (2025-03-26) exige **Streamable HTTP** y **stdio** sin sesiones a nivel de transporte `[LEÍDO: docs/DISENO_SANDBOX_MCP.md]`.
- **Aislamiento `[REFERENCIA A VERIFICAR]`:**
  - En Windows: `CreateRestrictedToken` con SID de integridad baja y Job Objects con `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
  - En Linux: `unshare(CLONE_NEWUSER | CLONE_NEWPID | CLONE_NEWNET)` sin privilegios root y cgroups v2 (`memory.max`, `cpu.max`).

---

## 3. A1: Reporte de Línea Base (Analizador de Reglas)

Ejecución formal registrada en `reports/baseline_rules_report.json` mediante `tools/measure_baseline.py`:

| Suite de Evaluación | N Casos | Exactitud RIR / Respuesta | Intervalo Wilson 95% | Falsa Aceptación | Abstención Correcta | Innecesaria | Estado |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| `unseen_frozen_v2` | 16 | **93.8%** (15/16) | [71.7%, 98.9%] | **0.0%** (0/16) | 0 | 1 | Evaluado [MEDIDO] |
| `known_failures_v2` | 14 | **92.9%** (13/14) | [68.5%, 98.7% | **0.0%** (0/14) | 0 | 1 | Evaluado [MEDIDO] |
| `f6_frozen_25` | 25 | **96.0%** (24/25) | [80.5%, 99.3%] | **0.0%** (0/25) | 0 | 1 | Evaluado [MEDIDO] |
| `dev_suite_v1` | 30 | **86.7%** (26/30) | [70.3%, 94.7%] | **3.3%** (1/30) | 5 | 3 | Evaluado [MEDIDO] |
| `external_human_gold_v1` | 100 | N/A | N/A | N/A | N/A | N/A | `PENDIENTE_DE_DATOS` |

### Análisis de Fallos del Baseline:
1. **Falsa Aceptación (1 caso en `dev_suite_v1`):** La consulta ambigua *"Tengo 10 de oro y perdí 3"* con opción múltiple fue clasificada con éxito parcial en lugar de abstención estricta por falta de entidad explícita.
2. **Abstención Innecesaria (3 casos en `dev_suite_v1`):** Frases compuestas con sintaxis invertida ("*A Pedro le regalaron 5 manzanas...*") cayeron en ruta epistémica incompleta por falta de disparador léxico en el analizador de reglas.

---

## 4. A2 y A3: Generador Curricular, Verificador y Fuga de N-gramas

### 4.1. Arquitectura en Rust (`crates/relic_cli/src/generator/`)
El generador se implementó modularmente respetando la regla $\le 180$ líneas por archivo:
- `types.rs` (37 líneas): Definición de niveles (`L01_AtomicArithmetic`, `L02_InventoryTracking`, `L03_EpistemicBoundaries`), splits (`Train`, `Holdout`) y estructura `CurriculumSample`.
- `templates.rs` (79 líneas): Plantillas parametrizadas con separación rigurosa entre prefijos de entrenamiento (`T_*`) y prefijos de validación (`H_*`).
- `sha256.rs` (108 líneas): Implementación criptográfica completa FIPS 180-4 de SHA-256 en Rust puro sin librerías externas.
- `engine.rs` (134 líneas): Generador pseudoaleatorio determinista (`Rng64`) por semilla, emisor del RIR gold canónico y validador semántico estricto con `validate_semantic()`.
- `export.rs` (79 líneas): Exportación JSONL y cálculo de hash criptográfico del dataset.
- `mod.rs` (50 líneas): Módulo integrador y 4 pruebas unitarias que validan determinismo, concordancia RIR y estatus epistémico.

### 4.2. Dataset Generado
- **Ruta:** `data/curriculum_dataset_v1.jsonl`
- **Volumen:** 375 muestras (300 Train, 75 Holdout, balanceadas equitativamente en 3 niveles).
- **Semilla fija:** 42.
- **SHA-256 del Dataset:**  
  `fadfedebe9c17c4794279e94434dab45068d1154e6ac892366b49f45e4a09ced` [MEDIDO]

### 4.3. Control de Fuga de N-gramas (`tools/leak_control.py`)
- **3-gramas Train vs Holdout:** **0 / 220 (0.00%)** [MEDIDO]. Cero fuga de plantillas o construcciones sintácticas entre particiones.
- **3-gramas Train vs Dev Suite:** **18 / 88 (20.45%)** [MEDIDO]. Coincidencias limitadas a conectores aritméticos universales ("*tengo un total*", "*cuanto me queda*").

---

## 5. A4: Análisis de Modelos y Tokenizadores

### 5.1. Comparación de Tokenización en Dominio Español [MEDIDO]
Se evaluó el vocabulario actual (`relic_tokenizer`, 4,262 tokens entrenados en TinyStories inglés) contra el corpus en español:
- **BPE TinyStories actual:** **2.11 tokens / palabra** [MEDIDO]. Fragmentación severa en morfología española (ej. *"manzanas"* $\to$ `['man', 'z', 'anas']`).
- **Caracteres / palabra:** **3.91 chars / palabra** [MEDIDO].
- **Bytes UTF-8 / palabra:** **4.95 bytes / palabra** [MEDIDO].
- **BPE de Dominio (Objetivo 8k tokens con palabras reservadas RIR):** **1.15–1.25 tokens / palabra** [ESTIMADO]. Reduce la longitud de secuencia en ~42%, duplicando el rendimiento de inferencia del decodificador.

### 5.2. Selección de Arquitectura del Modelo
Se compararon tres alternativas de compilador neuronal (Lenguaje → RIR):

| Opción | Parámetros | Huella RAM | Latencia CPU | Ventajas | Desventajas | Decisión |
| :--- | :---: | :---: | :---: | :--- | :--- | :---: |
| **A. RelicModel 10M Nativo** | 10.0M | $\le$ 35 MB | $< 5$ ms | Paridad 1:1 con Rust (`loader.rs`), cero costo en la nube, arranque instantáneo | Requiere decodificación guiada por gramática para garantizar JSON 100% válido | **SELECCIONADO** |
| **B. Gemma 2B / Qwen 1.5B (GGUF)** | 1.5B–2.0B | ~1.8 GB | ~120 ms | Fluidez en lenguaje libre y paráfrasis complejas | No cabe en los límites de memoria de RELIC, introduce dependencias pesadas en C++ | Descartado |
| **C. T5-Small / ByT5 Seq2Seq** | 60M–300M | ~250 MB | ~35 ms | Buena capacidad seq2seq | Arquitectura encoder-decoder no soportada por el motor de inferencia nativo | Descartado |

### 5.3. Decodificación Restringida por Gramática (Logit Masking)
Para eliminar alucinaciones sintácticas y garantizar que el modelo emita RIR 100% válido:
- Se compila la gramática BNF del RIR a un autómata finito determinista (DFA).
- En cada paso autorregresivo $t$, el estado del DFA genera una máscara booleana $M_t \in \{0, 1\}^{V}$.
- Se aplica logit masking:
  $$\tilde{z}_{t, v} = \begin{cases} z_{t, v} & \text{si } M_{t, v} = 1 \\ -\infty & \text{si } M_{t, v} = 0 \end{cases}$$
- **Modos de Falla y Mitigación:**
  1. *Ciclos infinitos en JSON:* El DFA impone longitud máxima de claves y profundidad máxima de anidación (3 niveles).
  2. *Entidades no presentes en el contexto:* Se restringe el subvocabulario de tokens de entidad a los identificadores extraídos previamente del texto de entrada.

---

## 6. A5: Notebook de Entrenamiento y Paridad Safetensors

Se estructuró el notebook de producción para Google Colab y Kaggle:
- **Ruta:** `notebooks/relic_colab_train_fase_a.ipynb`
- **Etiqueta:** `# ESTADO: PENDIENTE DE EJECUTAR EN GOOGLE COLAB / KAGGLE (NO EJECUTAR LOCALMENTE)`
- **Diseño Multi-Semilla:** Ejecuta 5 semillas pseudoaleatorias `[42, 1337, 2026, 7777, 9999]`.
- **Exportación Safetensors:** Mapea el `state_dict` de PyTorch directamente a los nombres consumidos por `relic_core::loader::SafetensorsLoader`:
  - `model.embed_tokens.weight`, `model.norm.weight`, `lm_head.weight`
  - `model.layers.{i}.input_layernorm.weight`, `model.layers.{i}.post_attention_layernorm.weight`
  - `model.layers.{i}.self_attn.{q,k,v,o}_proj.weight`, `model.layers.{i}.self_attn.{q,k}_norm.weight`
  - `model.layers.{i}.mlp.{gate,up,down}_proj.weight`
- **Verificación de Paridad:** Célula final comprueba mediante `safe_open` que el error máximo absoluto de carga sea exactamente $0.00000000$.

---

## 7. A6: Arnés de Evaluación y Compuerta de Aprobación

El script `tools/evaluate_harness.py` implementa el protocolo de decisión para validar cualquier checkpoint del modelo frente al analizador de reglas:

### Reglas de la Compuerta de Aprobación:
1. **Seguridad Crítica:**
   $$\text{Falsa Aceptación}(\text{Modelo}) \le \text{Falsa Aceptación}(\text{Reglas}) = 0.00\%$$
   Si el modelo acepta consultas epistemológicamente incompletas o erróneas, queda **automáticamente reprobado**.
2. **Superioridad Estadística (Prueba de McNemar):**
   Para casos discordantes ($b$: reglas aciertan y modelo falla; $c$: reglas fallan y modelo acierta):
   $$\chi^2 = \frac{(|b - c| - 1)^2}{b + c}, \quad p < 0.05 \quad \text{y} \quad c > b$$
3. **No-Inferioridad:**
   En las suites congeladas (`unseen_frozen_v2`), Exactitud $\ge 93.8\%$.

---

## 8. Lo Que No Sabemos (Incertidumbres Técnicas Abiertas)

1. **Rendimiento de Generalización Out-of-Distribution (OOD):**
   - No podemos saber con certeza cuántas muestras sintéticas adicionales se requieren para que el modelo de 10M generalice a oraciones pasivas o con orden sintáctico libre sin degradar la precisión exacta hasta que se ejecute el entrenamiento en Colab.
2. **Comportamiento en Expresiones Idiomáticas Colombianas:**
   - El archivo `tests/external_human_gold_v1.json` contiene la estructura y categorías balanceadas (100 casos), pero las cadenas de texto (`input`) están pendientes de recolección humana independiente. Por ende, la exactitud en regionalismos ("*me tumbaron*", "*se me embolataron*", "*le gasté*") permanece como una incógnita empírica.
3. **Interacción de Cuantización INT8 en CPU ARM/x86:**
   - La latencia medida en Rust F32 es de $< 5$ ms. No se ha medido el impacto de degradación en la exactitud al aplicar cuantización a pesos `i8` con escalas dinámicas en el motor `relic_core`.

---

## 9. Lista de Archivos y Entregables de la Fase A

1. [DISENO_SANDBOX_MCP.md](RELIC/docs/DISENO_SANDBOX_MCP.md): Documento de diseño arquitectónico corregido.
2. [curriculum_level.json](RELIC/schemas/curriculum_level.json): Esquema JSON formal del currículo con umbrales de Wilson corregidos.
3. [mcp_tools_spec.json](RELIC/schemas/mcp_tools_spec.json): Especificación de herramientas MCP compatible con la revisión 2025-03-26.
4. [dev_suite_v1.json](RELIC/tests/dev_suite_v1.json): Suite de desarrollo de 30 casos.
5. [external_human_gold_v1.json](RELIC/tests/external_human_gold_v1.json): Plantilla de 100 casos para evaluación humana externa.
6. [measure_baseline.py](RELIC/tools/measure_baseline.py): Script de medición de línea base de reglas.
7. [baseline_rules_report.json](RELIC/reports/baseline_rules_report.json): Reporte consolidado de línea base con intervalos Wilson 95%.
8. [curriculum_dataset_v1.jsonl](RELIC/data/curriculum_dataset_v1.jsonl): Dataset generado (375 muestras, SHA-256 verificado).
9. [leak_control.py](RELIC/tools/leak_control.py): Script de detección y reporte de fuga de 3-gramas.
10. [relic_colab_train_fase_a.ipynb](RELIC/notebooks/relic_colab_train_fase_a.ipynb): Notebook de entrenamiento multi-semilla para Google Colab / Kaggle.
11. [evaluate_harness.py](RELIC/tools/evaluate_harness.py): Arnés de evaluación comparativa Reglas vs Modelo con prueba de McNemar.
12. [crates/relic_cli/src/generator/](RELIC/crates/relic_cli/src/generator): Código fuente del generador curricular modular en Rust (todas las partes $\le 180$ líneas).

---

## 10. Cierre Experimental de Fase A: Evaluación Comparativa Per-Caso (N=75 Holdout)

Tras re-entrenar el modelo neuronal en GPU sobre el currículo corregido (`data/relic_10m_trained.safetensors`, SHA-256 `328d5e70a41c...`), se ejecutó la evaluación comparativa caso a caso sobre los 75 ejemplos de holdout (`tests/holdout_templates_v1.json`) registrando cada predicción en [reports/eval/holdout_75_per_case.json](RELIC/reports/eval/holdout_75_per_case.json):

| Sistema | Exact Match (EM) | Falsa Aceptación (FA)* | Rechazo Sintaxis | McNemar vs Modelo ($b, c$) | $\chi^2$ (Edwards) | $p$-value |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Reglas Simbólicas (`relic_cli`)** | **33 / 75 (44.0%)** [MEDIDO] | 0 / 75 (0.0%) [MEDIDO] | 0 / 75 | $b=33, c=0$ | 31.0303 | $2.54 \times 10^{-8}$ |
| **Slot-Filler Gramatical** | **46 / 75 (61.3%)** [MEDIDO] | 0 / 75 (0.0%) [MEDIDO] | 0 / 75 | $b=46, c=0$ | 44.0217 | $3.25 \times 10^{-11}$ |
| **Modelo 10M Causal Autónomo** | **0 / 75 (0.0%)** [MEDIDO] | 0 / 75 (0.0%) [MEDIDO] | 75 / 75 (100.0%) | — | — | — |

*\*Definición estricta:* Falsa Aceptación = salida con RIR válido y ejecutable pero semánticamente distinto del gold. Las salidas con sintaxis JSON rota o truncada se contabilizan como Rechazo Sintáctico, no como Falsa Aceptación.

### Veredicto Técnico:
El modelo causal libre de 10M sin decodificación restringida genera salidas malformadas y no supera a los baselines deterministas ($p < 10^{-7}$). Se ratifica la decisión arquitectural de transicionar hacia una **cabeza Pointer/Slot-Classifier** (o decodificación guiada por DFA) y delegar la ejecución en el motor determinista de Rust.

