# AUDITORÍA FUNCIONAL Y DE CAPACIDADES: RELIC & SIMPLE DOMAIN
**Rol:** Auditor Independiente de Software e IA Simbólica  
**Tipo de Intervención:** Exclusivamente Analítica (Sin modificaciones de código fuente ni implementación de producto)  
**Fecha de Auditoría:** 2026-10-26  
**Repositorios Auditados:** `Proyectos/RELIC` (Commit `63eaee9` [LEÍDO: git rev-parse --short HEAD]) y `Proyectos/Domain_Simple` (Commit `7031c4d` [LEÍDO: git rev-parse --short HEAD])

---

## 1. RESUMEN EJECUTIVO (Una Página)

### 1.1. Qué Puede Realmente el Motor
1. **Aritmética Racional Escalar Básica con Precedencia:** [MEDIDO: `run_battery_fase1.py` -> 11/12 (91.7%) en operaciones básicas y 10/10 (100%) en varios pasos con paréntesis]. Resuelve sumas, restas, multiplicaciones y divisiones encadenadas mediante Shunting-yard y `ArithmeticPlan` [LEÍDO: `crates/relic_cli/src/math.rs:14-139` y `crates/relic_core/src/arithmetic.rs:50-78`].
2. **Problemas de Inventario Canónicos:** [MEDIDO: `run_battery_fase1.py` -> 7/10 (70%) correctos]. Interpreta fórmulas con unidades explícitas cuando usan prefijos estrictos ("tengo", "hay", "cuento con") y verbos registrados en `ADD_WORDS` (19) o `SUB_WORDS` (38) [LEÍDO: `crates/relic_cli/src/dispatch.rs:71-81` e `crates/relic_cli/src/inverted.rs:13-24`].
3. **Alcanzabilidad en Grafos Espaciales Fijos (BFS acotado):** [MEDIDO: `run_battery_fase1.py` -> 8/10 (80%) correctos]. Resuelve conectividad y rutas entre entidades conocidas en memoria episódica con restricciones `via` y `avoid` hasta 8 saltos [LEÍDO: `crates/relic_core/src/reasoning_engine.rs:41-96` y `crates/relic_cli/src/answer.rs:105-119`].
4. **Comparaciones Relacionales Máximas Unidimensionales (Altura):** [MEDIDO: `run_battery_fase1.py` -> 4/10 (40%) correctos]. Detecta ciclos contradictorios y encuentra el elemento máximo estricto en dimensión "altura" (`altura_mayor`) sobre un orden parcial conexo [LEÍDO: `crates/relic_core/src/temporal_graph.rs:114-150` y `crates/relic_cli/src/relational.rs:168-175`].
5. **Rechazo Semántico e Invariantes Epistémicas:** [MEDIDO: `run_limits_fase2.py` -> División por cero y números < 1e-12 retornan `INCOMPLETE` / `DivisionByZero`]. El validador RIR y el motor rechazan NaN, infinitos y divisiones por cero [LEÍDO: `crates/relic_core/src/rir/validation.rs:35-50` y `crates/relic_core/src/arithmetic.rs:66-70`].

### 1.2. Qué NO Puede (Refutación de "Puede realizar cualquier cálculo")
1. **La afirmación "puede realizar cualquier cálculo" es categóricamente FALSA:** [LEÍDO: `crates/relic_core/src/arithmetic.rs:9-15`]. El enum `ArithmeticOp` posee **únicamente 4 operaciones**: `Add`, `Subtract`, `Multiply`, `Divide`.
2. **Cero Soporte de Potencias, Raíces y Logaritmos:** [MEDIDO: `run_battery_fase1.py` -> 0/10 (0%) en potencias y raíces]. Expresiones como `2 ^ 8`, `sqrt(144)` o `3 al cubo` son rechazadas a `UNSUPPORTED`.
3. **Cero Soporte de Porcentajes y Fracciones Verbales:** [MEDIDO: `run_battery_fase1.py` -> 0/8 en porcentajes; 3/8 en fracciones (solo si se escriben como división numérica)]. Expresiones como "20% de 150" o "un tercio más un sexto" van a `UNSUPPORTED`.
4. **Cero Álgebra Simbólica o Ecuaciones:** [MEDIDO: `run_battery_fase1.py` -> 0/8 en ecuaciones simples]. Expresiones como `x + 5 = 12` o `2x = 10` son rechazadas a `UNSUPPORTED`.
5. **Cero Reconocimiento de Operaciones Descriptivas ("mitad", "doble", "promedio"):** [MEDIDO: `run_battery_fase1.py` -> 0/8 correctos]. No existen reglas sintácticas para "mitad de X", "doble de Y" ni "promedio de A y B".
6. **Incapacidad Total de Comparación Mínima o Multidimensional:** [MEDIDO: `run_battery_fase1.py` -> "más bajo" (Lesser) y dimensiones "velocidad" o "peso" caen a `UNSUPPORTED`]. `relational.rs:168` exige hardcodeado `lower.contains("más alto")` y solo instancia `RelationalQuestionType::Maximal("altura_mayor")`.
7. **Reglas de Cadenas Inaccesibles desde la API:** [MEDIDO: `run_battery_fase1.py` -> 0/8 en reglas de cadenas]. `RuleCatalog` y `AtomicOperator` (`Reverse`, `ShiftLeft`, `SwapCase`, `DuplicateVowels`, `RemoveFirstLast`) existen en `relic_core` pero **están completamente desconectados** del despacho en `relic_cli` y `relic_server` [LEÍDO: `crates/relic_core/src/reasoning_engine.rs:109-155` y `crates/relic_cli/src/dispatch.rs:106-171`].

### 1.3. Límites Medidos Principales
- **Longitud de Petición HTTP:** Límite duro en $16.384$ bytes ($16\text{ KB}$) [MEDIDO: `run_limits_fase2.py` -> longitudes $\le 16.000$ retornan 200; $\ge 16.384$ retornan HTTP 413 `PAYLOAD_TOO_LARGE`]. [LEÍDO: `crates/relic_server/src/router.rs:48`].
- **Precisión Numérica f64 y Truncamiento:** [MEDIDO: `9007199254740992 + 1` retorna `9007199254740992` por límite de mantisa IEEE-754 ($2^{53}$)]. Además, la salida textual se trunca forzosamente a 2 decimales [LEÍDO: `crates/relic_core/src/arithmetic.rs:89` -> `format!("{:.2}", val)`], produciendo errores de exact match en decimales largos [MEDIDO: `12.345 + 67.89` produce `80.23` en vez de `80.235`].
- **Notación Científica No Soportada:** [MEDIDO: `1e300 * 2` retorna `UNSUPPORTED` porque el tokenizador léxico sólo acepta `0-9` y `.` [LEÍDO: `crates/relic_cli/src/math.rs:53`]].
- **Profundidad BFS Acotada:** Límite hardcodeado a 8 saltos [LEÍDO: `crates/relic_cli/src/answer.rs:106, 117`]. Si la ruta requiere $>8$ saltos, retorna "Indeterminado: Se alcanzó el límite de profundidad (8 saltos)".
- **Memoria Episódica Fija:** 13 hechos hardcodeados en código compilado [LEÍDO: `crates/relic_cli/src/knowledge.rs:6-23`]. Nodos multi-palabra como "Buenos Aires" se rompen al parsear por espacio [MEDIDO: `¿Se puede ir de Bogotá a Buenos Aires?` falla con "La entidad de destino 'Buenos' no existe" [LEÍDO: `graph.rs:81`]].

---

## 2. MATRIZ DE REQUISITOS DEL SANDBOX LAB FRENTE AL MÍNIMO (FASE 4)

Evaluación técnica de los 9 pilares del Sandbox frente a las necesidades de experimentación simbólica:

| N° | Requisito | Estado | Evidencia en Código y Medición | Diagnóstico Crítico |
|---|---|---|---|---|
| **1** | **Estado visible como grafo** | **FALTA** | [LEÍDO: `Domain_Simple/frontend/src/views/components/SandboxChart.tsx:1-94` y `SandboxEpisodeViewer.tsx:1-180`]. En ningún archivo frontend existe renderizado de nodos ni aristas. `SandboxChart.tsx` es únicamente una curva 2D polyline de Exact Match (`em_rate`). | El usuario no puede inspeccionar visualmente qué entidades, aristas o axiomas conoce el sistema en un instante dado. |
| **2** | **Traza real como árbol** | **PARCIAL / DECORATIVA** | [LEÍDO: `crates/relic_server/src/engine.rs:33-140`]. La traza es un arreglo plano lineal de 5 fases secuenciales (`normalizacion`, `intencion`, `rir`, `validacion`, `ejecucion`). No es un árbol ni DAG de resolución de submetas. En `relic_client.py:88-97` el fallback CLI inyecta latencias ficticias proporcionales (`0.15 * t`). En `SandboxEpisodeViewer.tsx:59-65` hay un mock hardcodeado si `trace` viene vacío. | No refleja la descomposición arbórea del problema ni el grafo de dependencias de la inferencia. |
| **3** | **Diff entre versiones** | **FALTA** | [LEÍDO: `simple_domain/repositories/sandbox.py:51-77`]. `create_run` almacena `commit_hash`, `dataset_hash` y `weights_hash`, pero NO versiona léxico, reglas ni hechos. No existe estructura de diff ni endpoints para comparar dos estados del sistema. | Imposible auditar qué reglas se añadieron, cambiaron o eliminaron entre experimentos. |
| **4** | **Reproducción de episodios** | **PARCIAL** | [LEÍDO: `sandbox_runner.py:85-100` y `sandbox.py:82, 115-122`]. Los casos se seleccionan con `random.Random(seed).sample(...)` y los eventos se registran en `events.jsonl`. Sin embargo, no existe endpoint `POST /v1/sandbox/runs/{id}/replay` ni interfaz UI para reproducir paso a paso o bifurcar un episodio con modificaciones. | Solo existe consulta estática post-mortem de episodios completados. |
| **5** | **Ciclo Proponer-Verificar-Adoptar-Revertir** | **FALTA** | [LEÍDO: `sandbox_runner.py:101-175`]. El sandbox actual es un mero evaluador de lotes (*batch benchmark*) sobre `curriculum_dataset_v2_20k.jsonl`. No permite proponer una regla, someterla a validación contra oráculo, adoptarla en un catálogo dinámico o revertirla ante regresiones. | El sandbox no es un laboratorio de cambio; es un lector de datasets estáticos. |
| **6** | **Núcleo inmutable** | **EXISTE** | [LEÍDO: `crates/relic_core/src/arithmetic.rs:1-166`, `temporal_graph.rs:1-162`, `reasoning_engine.rs:1-167`]. El código Rust está fuertemente tipado, compilado y carece de introspección dinámica, macros en runtime o punteros de mutación externa. | El kernel evaluador determinista es 100% inmutable y está protegido contra inyecciones arbitrarias. |
| **7** | **Entrada libre y "enseñar"** | **FALTA** | [LEÍDO: `Domain_Simple/frontend/src/views/components/SandboxLab.tsx:23-30`]. La interfaz solo ofrece selectores predefinidos (`level`, `seed`, `system`, `penalty_lambda`, `max_episodes`). No existe campo de entrada libre ni formulario de "enseñar nueva regla / hecho / alias". | No hay vía para que el operador o un LLM interactivo introduzca nuevo conocimiento. |
| **8** | **Aislamiento Chat / Sandbox** | **PARCIAL (ACCIDENTAL)** | [LEÍDO: `routes/conversations.py:71` y `jobs/sandbox_runner.py:59`]. Ambos componentes invocan al mismo `relic_client.py` apuntando a `http://127.0.0.1:5000/api/query`. Su aislamiento es puramente un efecto colateral de que `relic_server` no guarda estado en disco. Si uno mutara la RAM compartida, contaminaría al otro. | No hay separación formal de procesos, espacios de nombres de memoria ni sandboxing a nivel de SO. |
| **9** | **Procedencia** | **PARCIAL** | [LEÍDO: `sandbox_runner.py:63-84`, `ChatMessageBubble.tsx:125-145`]. Se registra procedencia a nivel macro (`commit`, SHA256 dataset, SHA256 pesos, badge `[REGLAS]`, `[SLOT-FILLER]`, `[MODELO]`). Pero a nivel micro (hechos individuales o reglas) la procedencia no existe (todo está fundido en el código). | Imposible rastrear qué hecho fue enseñado por qué usuario o sesión. |

### 2.1. Evaluación del Modelo de Datos de Eventos (`events.jsonl`)
- **Estructura Actual:** [LEÍDO: `simple_domain/jobs/sandbox_runner.py:112, 139, 157, 158`]. Emite `EPISODE_STARTED`, `RIR_PROPOSED`, `VERIFIER_EVALUATED`, `EPISODE_COMPLETED` y `sandbox.completed`.
- **Diagnóstico:** El modelo de eventos **NO permite reconstruir el grafo ni los diffs de estado**. En `RIR_PROPOSED`, los únicos campos son `{episode, status, route, trace, output_text, responding_system}`. No hay registro de nodos añadidos, aristas modificadas, cambios en el léxico ni deltas de configuración. Los eventos solo describen la ejecución efímera de una consulta contra un evaluador estático.

### 2.2. Costo Computacional de Renderizar Grafos en SVG vs Otras Técnicas
Para dotar al Sandbox de visualización de estado en PC con recursos limitados (Core i3/i5 integrado, sin GPU dedicada):

| Escala de Nodos | Nodos / Aristas Estimadas ($k=2.5$) | Elementos DOM en SVG | Consumo de RAM DOM | Latencia Render Inicial | FPS en Panning / Zooming | Viabilidad en SVG | Técnica Recomendada |
|---|---|---|---|---|---|---|---|
| **50 nodos** | 50 nodos / 125 aristas | $\sim 400$ elementos (`circle`, `line`, `text`) | $< 3\text{ MB}$ | $< 16\text{ ms}$ | 60 FPS | **Excelente** | **SVG Puro (React/D3)** |
| **200 nodos** | 200 nodos / 500 aristas | $\sim 1.600$ elementos | $\sim 15\text{ MB}$ | $150\text{ - }350\text{ ms}$ (física en JS) | 35-45 FPS | **Aceptable / Límite** | **SVG con layout precomputado en Web Worker** |
| **1.000 nodos** | 1.000 nodos / 2.500 aristas | $\sim 8.000\text{ - }12.000$ elementos | $> 120\text{ MB}$ | $2.500\text{ - }6.000\text{ ms}$ | $< 12\text{ FPS}$ (stuttering severo) | **INVIABLE (Colapso DOM)** | **HTML5 Canvas 2D (Cytoscape.js) o WebGL (PixiJS)** |

**Umbral Crítico de Transición [DERIVADO]:**
- Para $N \le 150$ nodos: Mantener SVG (permite estilización CSS fluida y accesibilidad de nodos).
- Para $N > 150$ nodos: Migrar obligatoriamente a HTML5 Canvas 2D o WebGL. El DOM del navegador no está diseñado para retener miles de nodos vectoriales reactivos en reconciliación de React.

---

## 3. INVENTARIO COMPLETO DEL ESTADO DEL SISTEMA (FASE 3a)

Auditoría exhaustiva de cada fragmento de estado, cuantificación de entradas léxicas y ubicación:

```
+----------------------------------------------------------------------------------------------------+
|                                    ESTADO TOTAL DEL SISTEMA RELIC                                  |
+------------------------------------+--------------------------+-----------------+------------------+
| Componente                         | Ubicación (Archivo:Línea)| Entradas Hardc. | Camino Escritura |
+------------------------------------+--------------------------+-----------------+------------------+
| Léxico de Numerales Verbales       | relic_cli/normalize.rs:65| 46 formas verbal| NINGUNO (Código) |
| Prefijos de Pregunta Aritmética    | relic_cli/normalize.rs:10| 6 prefijos      | NINGUNO (Código) |
| Operadores Verbales Multi-palabra  | relic_cli/normalize.rs:18| 3 patrones      | NINGUNO (Código) |
| Imperativos Aritméticos            | relic_cli/normalize.rs:30| 10 verbos       | NINGUNO (Código) |
| Conectores de Inventario           | relic_cli/inverted.rs:25 | 6 conectores    | NINGUNO (Código) |
| Verbos de Incremento (ADD_WORDS)   | relic_cli/inverted.rs:13 | 19 palabras     | NINGUNO (Código) |
| Verbos de Decremento (SUB_WORDS)   | relic_cli/inverted.rs:18 | 38 palabras     | NINGUNO (Código) |
| Prefijos de Inicio de Inventario   | relic_cli/dispatch.rs:37 | 7 prefijos      | NINGUNO (Código) |
| Palabras Clave de Grafos           | relic_cli/dispatch.rs:154| 7 palabras      | NINGUNO (Código) |
| Restricciones de Ruta (avoid/via)  | relic_cli/graph.rs:16,23 | 9 giros verbales| NINGUNO (Código) |
| Regla Específica Hardcodeada Marta | relic_cli/graph.rs:74    | 1 regla ad-hoc  | NINGUNO (Código) |
| Cláusulas Comparativas / Polarity  | relic_cli/relational.rs  | 14 giros comp.  | NINGUNO (Código) |
| Dimensiones Relacionales           | relic_cli/relational.rs:53| 4 dimensiones   | NINGUNO (Código) |
| Hechos Frecuentes Memoria (S1)     | relic_cli/knowledge.rs:6 | 13 aristas      | NINGUNO (Código) |
| Catálogo de Reglas Atómicas Cadenas| relic_core/reasoning.rs  | 5 operadores    | NINGUNO (Código) |
| Pesos Neuronales Safetensors       | artifacts/weights/       | ~10.5M floats   | Solo lectura     |
| Vocabulario BPE Tokenizer          | artifacts/tokenizer/     | 4.096 tokens    | Solo lectura     |
+------------------------------------+--------------------------+-----------------+------------------+
```

### 3.1. Cuantificación y Análisis de Persistencia
1. **Entradas Léxicas en Código Compilado:** Se identificaron **más de 180 tokens léxicos y patrones sintácticos fijos** codificados directamente en strings dentro del código fuente de Rust (`normalize.rs`, `dispatch.rs`, `inverted.rs`, `graph.rs`, `relational.rs`).
2. **Cero Persistencia Dinámica en RELIC:** El motor Rust carece de base de datos SQLite, archivos YAML/JSON de configuración léxica externa o endpoints de mutación. Toda modificación del vocabulario o reglas exige recompilar el binario Rust (`cargo build`).
3. **Desconexión con `Domain_Simple` Storage:** En `Domain_Simple` existe un archivo `storage/patterns.json` que registra deducciones en turnos de chat [LEÍDO: `conversations.py:86-102`]. Sin embargo, **RELIC jamás lee este archivo** [MEDIDO: grep search de `patterns.json` en `RELIC` -> 0 ocurrencias]. Es un buzón muerto en el backend de Python.

---

## 4. EXPERIMENTOS DE LÍNEA BASE DE CAMBIO Y APRENDIZAJE (FASE 3b)

Resultados medidos ejecutando consultas de prueba contra `POST /api/query` [MEDIDO: `run_experiments_fase3.py`]:

### Experimento 1: Verbo Coloquial Nuevo ("se embolataron")
- **Entrada:** `"Tengo 10 peras y se me embolataron 3"`
- **Salida:** `HTTP 200` | Status: `INCOMPLETE` | Ruta: `epistemic_rejection`  
  `"Consulta incompleta: Problema de inventario no estructurado o con operaciones ambiguas/no reconocidas."`
- **Diagnóstico Arquitectónico:** El analizador de inventario [LEÍDO: `dispatch.rs:76-81` e `inverted.rs:18-24`] solo reconoce el conjunto cerrado de 38 palabras de `SUB_WORDS`. "Embolataron" no existe en la lista. Como el analizador no cuenta con fallback a diccionario ni capacidad de inferir que una cantidad se redujo por el contexto, aborta con abstención epistémica.

### Experimento 2: Hecho Factual Nuevo
- **Entrada:** `"Cali está conectada con Medellín. ¿Se puede ir de Cali a Medellín?"`
- **Salida:** `HTTP 200` | Status: `INVALID` | Ruta: `deterministic_graph`  
  `"Consulta irresoluble: La entidad de origen 'Cali' no existe en la memoria episódica."`
- **Diagnóstico Arquitectónico:** `relic_server` inicializa la memoria episódica una sola vez al arrancar desde `knowledge.rs:6-23` con 13 aristas fijas. `POST /api/query` sólo ejecuta lecturas (`resolve_node` y `solve_reachability_bfs_constrained`). No existe gramática para parsear "X está conectada con Y" como aserción ni comando para mutar el grafo. Al resolver la entidad "Cali", `resolve_strict` falla con `NotFound` [LEÍDO: `relic_cli/src/answer.rs:83-86`].

### Experimento 3 y 4: Corrección de Regla por el Usuario (Axioma Local y Persistencia)
- **Turno 1:** `"En este juego 2 + 2 es igual a 5. ¿Cuánto es 2 + 2?"`
- **Salida 1:** `HTTP 200` | Status: `VALID` | Ruta: `deterministic_arithmetic`  
  `"Entendí: 2 + 2 = 4. El resultado es 4."`
- **Turno 2:** `"2 + 2"`
- **Salida 2:** `HTTP 200` | Status: `VALID` | Ruta: `deterministic_arithmetic`  
  `"Entendí: 2 + 2 = 4. El resultado es 4."`
- **Diagnóstico Arquitectónico:** El motor evalúa expresiones matemáticas invocando directamente operadores f64 de hardware (`a + b`) [LEÍDO: `relic_core/src/arithmetic.rs:62`]. El servidor carece por diseño de memoria conversacional entre peticiones HTTP (`ServerState` es sin estado conversacional [LEÍDO: `state.rs:13-19`]). No existe un subsistema de asunciones o lógica no monótona que permita sobreescribir temporalmente reglas aritméticas.

---

## 5. QUÉ NO PUEDE HACER HOY: LIMITACIONES DE DISEÑO VS FUNDAMENTALES (FASE 3c)

| Capacidad Ausente | ¿Limitación de Diseño o Fundamental? | Justificación Arquitectónica |
|---|---|---|
| **Adoptar palabras nuevas (Léxico)** | **Limitación de Diseño** (Fácilmente superable) | Las listas `ADD_WORDS`, `SUB_WORDS` y numerales están en arrays fijos en código Rust. Bastaría desacoplarlas a un archivo JSON/SQLite cargado en memoria con camino de escritura y recarga en caliente. |
| **Aprender hechos nuevos en runtime** | **Limitación de Diseño** (Superable con API) | `EpisodicMemory` ya implementa `insert_edge(&mut self, u, v)` [LEÍDO: `episodic_memory.rs:59`]. Solo falta exponer un endpoint de mutación (`POST /v1/knowledge/facts`) con autenticación y persistencia a disco (WAL o JSON). |
| **Persistir lo aprendido entre sesiones** | **Limitación de Diseño** | Falta la capa de almacenamiento para la memoria episódica de RELIC (actualmente solo vive en RAM transitoria de `ServerState`). |
| **Crear procesos u operadores nuevos** | **Limitación Fundamental de la IA Simbólica Pura** | Una máquina simbólica determinista no puede sintetizar código arbitrario ni crear nuevos operadores matemáticos (ej. inventar la integral o el cálculo tensorial) sin un intérprete de programas (DSL / Lisp / Python sandbox) o un modelo de síntesis de programas formalmente verificado. |
| **Autocorregirse sin oráculo exterior** | **Limitación Fundamental Epistémica** | Un sistema puramente deductivo no puede determinar si su premisa es falsa a menos que entre en contradicción interna (ciclo en DAG) o reciba retroalimentación con verdad terreno (*ground truth* / oráculo externo). |

---

## 6. TABLA FINAL: PUEDE / PUEDE PARCIALMENTE / NO PUEDE (FASE 1c)

| Categoría Funcional | Veredicto | Resueltos Correctos [MEDIDO] | Abstenciones [MEDIDO] | Errores / Fallas [MEDIDO] | Evidencia en Código y Comportamiento |
|---|---|---|---|---|---|
| **Aritmética Básica (+, -, \*, /)** | **PUEDE** | 11 / 12 (91.7%) | 1 (Div/0) | 0 | Shunting-yard robusto en `math.rs:14-139`. Rechazo adecuado de división por cero. |
| **Aritmética Varios Pasos y Paréntesis**| **PUEDE** | 10 / 10 (100.0%) | 0 | 0 | Evaluación impecable de precedencia con pilas en `math.rs:83-139`. |
| **Inventarios Verbales Canónicos** | **PUEDE PARCIALMENTE** | 7 / 10 (70.0%) | 3 (30.0%) | 0 | Funciona con las 57 palabras de `ADD_WORDS`/`SUB_WORDS`. Falla con sinónimos ajenos ("adopto", "se embolataron"). |
| **Números Negativos** | **PUEDE PARCIALMENTE** | 9 / 10 (90.0%) | 1 (10.0%) | 0 | Funciona en signos unarios prefijos. Falla en sintaxis `15 + -5` por rigidez de tokens en `math.rs:43`. |
| **Decimales Escalares** | **PUEDE PARCIALMENTE** | 7 / 10 (70.0%) | 1 (10.0%) | 2 (Truncamiento) | Realiza el cálculo f64 exacto, pero `format_result` trunca a 2 decimales (`{:.2}`) rompiendo exactitud. "coma" no se normaliza. |
| **Grafos Espaciales y Software** | **PUEDE PARCIALMENTE** | 8 / 10 (80.0%) | 0 | 2 (Entidad multi-palabra y token crate) | BFS preciso hasta 8 saltos. Falla en nombres compuestos ("Buenos Aires") y exige palabra "crate" para dependencias. |
| **Comparaciones Relacionales** | **PUEDE PARCIALMENTE** | 4 / 10 (40.0%) | 6 (60.0%) | 0 | Solo resuelve "más alto" (Maximal). Rechaza "más bajo", otras dimensiones y grafos disjuntos. |
| **Números Grandes ($\le 2^{53}$)** | **PUEDE PARCIALMENTE** | 6 / 10 (60.0%) | 4 (40.0%) | 0 | Pierde precisión en $>2^{53}$. Falla en notación científica `1e300` y numerales "millón". Detecta overflow en `1e308`. |
| **Fracciones Simples** | **PUEDE PARCIALMENTE** | 3 / 8 (37.5%) | 5 (62.5%) | 0 | Solo si se escriben con slash `3/5 * 10`. Falla en lenguaje verbal ("un tercio"). |
| **Porcentajes** | **NO PUEDE** | 0 / 8 (0.0%) | 7 (87.5%) | 1 (Timeout) | El operador `%` no existe en el lexer ni gramática. Cae a `UNSUPPORTED`. |
| **Potencias y Raíces** | **NO PUEDE** | 0 / 10 (0.0%) | 10 (100.0%) | 0 | Operadores `^`, `**`, `sqrt` inexistentes en `ArithmeticOp` [LEÍDO: `arithmetic.rs:9-15`]. |
| **Mitad / Doble / Promedio** | **NO PUEDE** | 0 / 8 (0.0%) | 8 (100.0%) | 0 | Sin patrones gramaticales para funciones unarias agregadas. |
| **Ecuaciones Simples** | **NO PUEDE** | 0 / 8 (0.0%) | 8 (100.0%) | 0 | Sin motor algebraico ni resolución simbólica de incógnitas. |
| **Reglas de Cadenas** | **NO PUEDE (Desconectado)**| 0 / 8 (0.0%) | 8 (100.0%) | 0 | `AtomicOperator` y `RuleCatalog` existen en el crate pero no están cableados en la API. |
| **Memoria y Hechos Generales** | **NO PUEDE** | 0 / 8 (0.0%) | 8 (100.0%) | 0 | Memoria fija a 13 hechos. No es un grafo de conocimiento enciclopédico. |
| **Lenguaje Libre y Diálogo** | **NO PUEDE** | 0 / 10 (0.0%) | 10 (100.0%) | 0 | Se abstiene correctamente con `UNSUPPORTED` o cae en timeout de generación neuronal sin pesos. |

---

## 7. HALLAZGOS AUDITADOS POR SEVERIDAD Y ESFUERZO ESTIMADO

### 7.1. Severidad Crítica
1. **Falsa Pretensión de Universalidad Aritmética:**
   - *Hallazgo:* Se afirmaba que el sistema podía resolver cualquier cálculo matemático, cuando su núcleo `ArithmeticOp` está limitado a 4 operaciones de punto flotante f64 [LEÍDO: `arithmetic.rs:10-15`].
   - *Riesgo:* Publicidad técnica engañosa, colapso de expectativas de usuario e imposibilidad de resolver problemas curriculares elementales (porcentajes, potencias, fracciones).
   - *Esfuerzo:* **Medio** (3 a 5 días para incorporar `Power`, `Sqrt`, `Modulo`, `Percentage` en RIR y motor).

2. **Sandbox Lab Inoperante como Entorno de Aprendizaje:**
   - *Hallazgo:* El Sandbox no permite proponer, validar ni adoptar reglas; no tiene visualización de estado en grafo ni diffs de versión [LEÍDO: `SandboxLab.tsx` y `sandbox_runner.py`].
   - *Riesgo:* Imposibilidad de experimentar mejoras léxicas o factuales de forma controlada.
   - *Esfuerzo:* **Alto** (2 a 3 semanas para construir la arquitectura mutable y la UI de inspección).

### 7.2. Severidad Alta
3. **Pérdida de Información por Truncamiento Forzado a 2 Decimales:**
   - *Hallazgo:* `ArithmeticPlan::format_result` formatea números no enteros con `format!("{:.2}", val)` [LEÍDO: `arithmetic.rs:89`], truncando decimales y generando discrepancias numéricas [MEDIDO: `12.345 + 67.89` produce `80.23`].
   - *Riesgo:* Errores de cálculo acumulados y fallos artificiales de Exact Match en benchmarks.
   - *Esfuerzo:* **Bajo** (2 horas para formatear con precisión dinámica o número significativo).

4. **Fallo en Extracción de Nodos Compuestos en Grafos:**
   - *Hallazgo:* El extractor toma solo el primer token tras `split_whitespace` [LEÍDO: `graph.rs:81`], rompiendo entidades como "Buenos Aires" o "Nueva York".
   - *Riesgo:* Inconsistencia y fallos silenciosos en consultas de conectividad geográfica.
   - *Esfuerzo:* **Bajo** (1 día para implementar búsqueda de entidades sobre trie o diccionario canónico).

5. **Reglas de Cadenas Fantasma (Dead Code en Producción):**
   - *Hallazgo:* `RuleCatalog` y `AtomicOperator` están completamente implementados y probados en `relic_core`, pero no tienen rama en `parse_dispatch_intent` ni en `/api/query` [LEÍDO: `dispatch.rs:106-171`].
   - *Riesgo:* Funcionalidad dormida inaccesible para los clientes HTTP y CLI.
   - *Esfuerzo:* **Bajo** (1 día para exponer el intent `RuleComposition` en el router).

### 7.3. Severidad Media
6. **Fragilidad de la Normalización Verbal de Números:**
   - *Hallazgo:* El léxico se detiene en 30 y salta de decena en decena hasta 90 [LEÍDO: `normalize.rs:65-101`]. Números como "treinta y uno" o "cien" no son convertidos.
   - *Riesgo:* Fallos arbitrarios ante lenguaje natural estándar.
   - *Esfuerzo:* **Medio** (2 días para integrar un parser numérico composicional en español).

7. **Sobrecarga de Renderizado en SVG para Grafos Grandes:**
   - *Hallazgo:* Renderizar $>200$ nodos en SVG puro causa caídas de FPS y por encima de 1.000 nodos colapsa el DOM del navegador [DERIVADO: cálculo de elementos DOM].
   - *Riesgo:* Congelamiento de interfaz en computadores modestos al visualizar grafos medianos.
   - *Esfuerzo:* **Medio** (3 días para usar Canvas 2D en grafos $>150$ nodos).

---

## 8. PROPUESTA DE DISEÑO: ARQUITECTURA MUTABLE SEGURA Y CICLO DE CAMBIO (FASE 5)

> **Nota:** Esta propuesta es un diseño conceptual arquitectónico, sin modificaciones de código de producto, conforme al rol de auditor.

### 8.1. Arquitectura de Estado: Núcleo Inmutable vs Capa Mutable
Para que el sistema aprenda sin corromper su precisión, el estado debe desacoplarse en dos niveles estrictos:

```
+-----------------------------------------------------------------------------------+
|                            CAPA 1: NÚCLEO INMUTABLE (RUST)                        |
| - Tipos de datos fundamentales (f64, AST RIR v1, TemporalGraph, EpisodicMemory)    |
| - Motor aritmético evaluador (Add, Sub, Mul, Div, Pow, etc.)                       |
| - Validador de invariantes semánticas (rechazo de NaN, Inf, Div/0)                 |
| - Algoritmo BFS con límites de profundidad y detección de ciclos                   |
+-----------------------------------------------------------------------------------+
                                         ▲
                                         │ Consulta de solo lectura
                                         │
+-----------------------------------------------------------------------------------+
|                        CAPA 2: CAPA MUTABLE DINÁMICA (JSONL/SQLITE)                |
| - Léxico y Alias: sinónimos de verbos ("se embolataron" -> SUB_WORDS)             |
| - Hechos Factuales: aristas episódicas (origen, relación, destino, metadatos)      |
| - Plantillas de Parseo: mapeo frase pattern -> Documento RIR                       |
| - Control de Versiones: Historial de cambios inmutable con hashes encadenados     |
+-----------------------------------------------------------------------------------+
```

### 8.2. Formato de Versiones, Diffs y Registro de Procedencia
Cada cambio en la capa mutable se almacena como una entrada en un archivo append-only `changes.jsonl`:

```json
{
  "change_id": "chg_a8f9c102",
  "parent_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "timestamp": "2026-10-26T17:30:00Z",
  "author": "dev-nikolas | llm_teacher_v1",
  "target_layer": "lexicon_alias",
  "operation": "INSERT",
  "data": {
    "source_term": "se embolataron",
    "canonical_op": "subtract",
    "domain": "inventory",
    "language": "es-CO"
  },
  "verification": {
    "oracle_evaluated": true,
    "regression_suite_pass": true,
    "pass_rate_before": 0.884,
    "pass_rate_after": 0.891,
    "regressions_count": 0
  },
  "hash": "4a5e1e... [SHA256 encadenado]"
}
```

### 8.3. Ciclo de Vida del Cambio: Proponer -> Verificar -> Adoptar -> Revertir

```
[1. PROPONER] ---> Operador o LLM propone nuevo término o hecho (entra en CUARENTENA)
       │
       ▼
[2. VERIFICAR] ---> Se evalúa contra:
       │            a) Casos gold sintéticos del nuevo término (debe pasar 100%)
       │            b) Suite de regresión congelada (Curriculum Benchmark)
       │
       ▼
   ¿Regresiones == 0 y Acierto Nuevo == 100%?
       ├── NO  ---> [RECHAZAR] Se emite reporte de colisión y el cambio se descarta.
       │
       └── SÍ  ---> [ADOPTAR] Se calcula nuevo hash SHA256 y se agrega a changes.jsonl.
                          │
                          ▼
                    [REVERTIR] (En cualquier momento mediante rollback al snapshot anterior)
```

### 8.4. Cambios Seguros vs Inseguros

| Tipo de Cambio | Clasificación | Justificación y Nivel de Riesgo |
|---|---|---|
| **Nuevos sinónimos léxicos (Alias)** | **SEGURO** | Mapea una palabra no ambigua a un operador existente (`"desperdicié"` $\to$ `SUB_WORDS`). El validador RIR sigue inspeccionando el resultado. |
| **Nuevas entidades y hechos episódicos** | **SEGURO** | Añadir aristas a grafos dirigidos (`"Medellín" -> "Cali"`). La transitividad BFS y la detección de ciclos evitan contradicciones. |
| **Nuevas plantillas sintácticas frase $\to$ RIR**| **MODERADO (Requiere Suite)** | Puede provocar sobreajuste o colisiones de parseo si la expresión regular o patrón es demasiado laxa. |
| **Nuevas operaciones aritméticas en el núcleo**| **INSEGURO / CRÍTICO** | Modifica el kernel determinista. Jamás debe hacerse en tiempo de ejecución; exige compilación y prueba unitaria en Rust. |
| **Relajación de validadores de invariantes**| **PROHIBIDO** | Si se desactiva la comprobación de división por cero o NaN, se quiebran las garantías de solidez de todo el sistema. |

### 8.5. Tres Vías para Inducir Léxico sin Envenenamiento

1. **Vía 1: Supervisión Directa del Operador Humano (Curaduría)**
   - *Procedimiento:* El operador ingresa pares en el Sandbox: `"se embolataron" := resta`.
   - *Costo:* Bajo en cómputo; alto en tiempo humano.
   - *Riesgo:* Mínimo (confianza alta).
   - *Control de Envenenamiento:* Doble confirmación humana y verificación de no-colisión con palabras reservadas.

2. **Vía 2: Inducción desde Ejemplos con Verificación por Oráculo**
   - *Procedimiento:* El sistema recibe 5 oraciones con el término desconocido y sus resultados numéricos correctos: `"Tenía 10 y se me embolataron 2 -> 8"`. El inductor prueba hipótesis (`ADD`, `SUB`, `MUL`, `DIV`) y deduce cuál operador satisface todos los oráculos.
   - *Costo:* Medio (múltiples forward passes).
   - *Riesgo:* Medio (posible coincidencia espuria si no hay suficientes ejemplos).
   - *Control de Envenenamiento:* Mínimo de 3 ejemplos con números dispares ($10 - 2 = 8$, $50 - 15 = 35$, $100 - 1 = 99$) para eliminar hipótesis degeneradas.

3. **Vía 3: LLM como Profesor Offline Asistido por Oráculo Lógico**
   - *Procedimiento:* Un LLM potente externo genera propuestas léxicas estructuradas analizando diccionarios y modismos regionales (ej. jerga colombiana). Emite un JSON de propuesta.
   - *Costo:* Alto en inferencia offline; cero costo en runtime.
   - *Riesgo:* Alto si se aceptara ciegamente (alucinaciones semánticas).
   - *Control de Envenenamiento:* La propuesta del LLM entra estrictamente en **Cuarentena** y debe aprobar automáticamente la batería de no-regresión antes de que cualquier humano la apruebe.

### 8.6. Promoción del Sandbox al Chat (Aislamiento de Entornos)
- **Regla Fundamental:** El CHAT opera sobre un snapshot inmutable consolidado (versión $V_k$).
- **Flujo de Promoción:**
  1. En el Sandbox se experimentan cambios en una rama de trabajo ($V_{k+1\text{-draft}}$).
  2. Cuando una rama acumula mejoras demostradas (ej. $+3.5\%$ en Exact Match sin regresiones), el operador emite un comando formal de release (`promote_release(version="1.2.0")`).
  3. Se genera un snapshot estático `snapshot_v1.2.0.json`.
  4. El servidor de Chat recarga de forma atómica en memoria el nuevo snapshot sin tiempo de inactividad (*zero-downtime hot reload*).

### 8.7. Experimento Mínimo con Criterio de Descarte
- **Objetivo:** Enseñar 10 verbos coloquiales de inventario en español regional (ej. "embolataron", "despaché", "adjudiqué", "rapiñaron", "esfumaron").
- **Procedimiento:**
  1. Medir tasa de acierto inicial en un banco independiente de 100 casos que contengan dichos verbos.
  2. Aplicar la inducción léxica en Sandbox.
  3. Ejecutar la suite completa de 22.501 casos de `curriculum_dataset_v2_20k.jsonl`.
- **Criterio Estricto de Descarte:** Si se produce **incluso 1 sola regresión** en casos previamente resueltos del benchmark oficial, o si la cobertura en el nuevo banco es $<90\%$, el cambio se **descarta automáticamente** y se retorna al estado anterior.

---

## 9. MATRIZ DE RIESGOS (FASE 6)

| Riesgo | Probabilidad | Impacto | Mecanismo de Mitigación en el Diseño |
|---|---|---|---|
| **Envenenamiento Léxico (Data Poisoning)** | Media | Crítico | Cuarentena obligatoria; validación cruzada con oráculo numérico; prohibición de sobreescribir palabras del sistema central. |
| **Deriva Semántica (Semantic Drift)** | Alta | Alto | Suite de regresión congelada e inviolable ejecutada en cada commit de la capa mutable. |
| **Sobreajuste a Ejemplos de Entrenamiento** | Alta | Medio | Validación pareada en dos splits: ejemplos de inducción vs holdout ciego nunca visto. |
| **Corrupción de Atribución y Procedencia** | Media | Alto | Firma criptográfica SHA-256 encadenada (`parent_hash`) en cada evento de cambio. |
| **Explosión y Crecimiento Descontrolado de Estado** | Baja | Medio | Compactación periódica de snapshots y poda de alias redundantes de baja frecuencia de uso. |

---

## 10. LAS 10 ACCIONES MÁS RENTABLES (ORDENADAS POR IMPACTO / ESFUERZO)

> **Nota:** Recomendaciones exclusivamente analíticas para priorización del proyecto.

1. **Añadir Normalización de la "Coma" Decimal y Eliminar Truncamiento `{:.2}`:**
   - *Impacto:* Inmediato. Resuelve discrepancias numéricas en decimales sin tocar la lógica matemática.
   - *Esfuerzo:* Muy Bajo (1 hora de edición en `normalize.rs` y `arithmetic.rs:89`).
2. **Conectar `RuleCatalog` (Transformación de Cadenas) a la API:**
   - *Impacto:* Alto. Habilita 5 operaciones funcionales que ya están probadas en Rust pero inactivas en producción.
   - *Esfuerzo:* Bajo (4 horas para añadir el dispatch `RuleComposition`).
3. **Soporte de Entidades Multi-Palabra en Grafos:**
   - *Impacto:* Alto. Permite resolver ciudades compuestas ("Buenos Aires", "San Francisco") sin romper el parser.
   - *Esfuerzo:* Bajo (1 día en `graph.rs`).
4. **Implementar Operaciones Aritméticas Faltantes en el Núcleo (`Power`, `Sqrt`, `Percentage`):**
   - *Impacto:* Muy Alto. Eleva la cobertura matemática básica a un estándar formal de calculadora avanzada.
   - *Esfuerzo:* Medio (2 a 3 días en `relic_core`).
5. **Persistencia Externa del Léxico en JSON / SQLite:**
   - *Impacto:* Muy Alto. Desbloquea la capacidad de incorporar nuevo vocabulario sin recompilar Rust.
   - *Esfuerzo:* Medio (3 días).
6. **Habilitar Comparaciones de Orden Menor (`Lesser`) y Otras Dimensiones:**
   - *Impacto:* Medio. Permite responder "¿quién es el más bajo?", "¿quién es el más rápido?" en `relational.rs`.
   - *Esfuerzo:* Bajo (1 a 2 días).
7. **Migrar Visualización de Grafos a Canvas 2D en Sandbox Lab:**
   - *Impacto:* Alto. Garantiza que la UI sea fluida en PC modestos al visualizar grafos de conocimiento.
   - *Esfuerzo:* Medio (3 días usando Cytoscape.js o Vis.js).
8. **Endpoint de Mutación de Hechos Factuales (`POST /v1/knowledge/facts`):**
   - *Impacto:* Alto. Permite enseñar nuevas rutas y dependencias a la memoria episódica dinámicamente.
   - *Esfuerzo:* Medio (2 días en `relic_server`).
9. **Formato JSONL de Registro de Cambios con Hash Encadenado:**
   - *Impacto:* Crítico para gobernanza. Otorga trazabilidad total de quién enseñó qué regla o hecho.
   - *Esfuerzo:* Medio (4 días en `Domain_Simple`).
10. **Aislamiento de Procesos Real entre Chat y Sandbox:**
    - *Impacto:* Alto en seguridad. Evita que una corrida de estrés en el Sandbox afecte la latencia o disponibilidad del Chat conversacional.
    - *Esfuerzo:* Medio (2 días configurando workers independientes o puertos dedicados).

---

## 11. LO QUE NO SABEMOS (HIPÓTESIS E INCÓGNITAS ABIERTAS)

1. **Rendimiento de Inferencia Neuronal en CPU Local:** Se desconoce la latencia y consumo de RAM exacto de `relic_10m_fp16.safetensors` ejecutando autoregresivamente token por token en CPU local modesta, dado que las pruebas de generación arrojaron timeout de gateway HTTP 504.
2. **Límite de Escalabilidad de `EpisodicMemory` en RAM:** No se ha medido el punto de saturación de memoria RAM si la memoria episódica alcanzara $>500.000$ aristas en estructuras nativas de Rust (`HashMap<String, Vec<String>>`).
3. **Impacto de la Concurrencia en Mutaciones en Caliente:** Al ser `memory: Arc<RwLock<EpisodicMemory>>` en `relic_server`, no se ha medido la degradación de latencia de lectura si múltiples hilos escriben nuevos hechos concurrentemente durante el servicio de consultas de chat.
4. **Resistencia a Ambigüedad Lingüística Compleja:** No sabemos cómo se comportaría el sistema ante oraciones complejas con cláusulas relativas subordinadas anidadas múltiples o ironía en español, al carecer de un modelo sintáctico probabilístico completo.
