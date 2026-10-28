# RELIC — Auditoría Forense Integral de IA, LLM, Machine Learning y Hardcoding

**Fecha de Ejecución:** 26 de Octubre de 2026  
**Commit de Referencia:** `acb1780` (`master`)  
**Workspace:** `RELIC`  
**Estado del Working Tree:** Íntegro, sin modificaciones destructivas; builds y experimentos ejecutados en directorio aislado (`scratch/target`).  
**Convención Epistémica:**
- **[HECHO VERIFICADO]**: Comprobado directamente en el código fuente o en la estructura de archivos.
- **[RESULTADO MEDIDO]**: Obtenido mediante ejecuciones reproducibles con scripts y comandos verificables.
- **[INFERENCIA]**: Deducción lógica basada estrictamente en los hechos y mediciones anteriores.
- **[NO VERIFICADO]**: Componente o prueba bloqueada o no ejecutada por restricciones del entorno.

---

## 1. Resumen Ejecutivo

La hipótesis de auditoría queda **confirmada en su núcleo técnico y operativo**:

> **RELIC es en la práctica un sistema híbrido compuesto por un parser léxico heurístico de reglas deterministas (Rust), una capa aritmética Shunting-Yard, un resolvedor de caminos BFS sobre un grafo estático de 13 aristas, y una representación intermedia (RIR) construida post-hoc. Sus componentes de Machine Learning y LLM (modelo causal de 10.03M parámetros) están totalmente desconectados del flujo de razonamiento y son funcionalmente incapaces de procesar o generar lenguaje natural coherente.**

El 100% de las respuestas de deducción, aritmética, inventario y conectividad espacial/software son calculadas por funciones deterministas escritas a mano en Rust (`crates/relic_cli` y `crates/relic_core`). El modelo neuronal causal de 10.03M parámetros solo se invoca cuando la consulta contiene palabras imperativas de ficción (`genera`, `inventa`, `cuento`, `historia`), en cuyo caso emite cadenas incoherentes o repeticiones de saltos de línea (`\n`), con una pérdida de validación medida de **8.1820** (perplejidad de 3576 frente a 9.011 de un modelo puramente aleatorio).

---

## 2. Matriz de Capacidades

| Capacidad Declarada | Implementación Localizada | Prueba / Evidencia Ejecutada | Resultado Medido | Nivel de Evidencia | Límites Reales / Observaciones |
|---|---|---|---|---|---|
| **Aritmética y Expresiones** | `crates/relic_cli/src/math.rs:158` (Shunting-yard determinista) | Expresiones con precedencia y paréntesis: `(8 + 2) * (9 - 4) / 5` | **10.0** (Correcto) | **[RESULTADO MEDIDO]** | Exactitud matemática estricta. Limitado a 4 operaciones básicas. |
| **Problemas de Inventario** | `crates/relic_cli/src/dispatch.rs:35` (coincidencia de subcadenas) | Batería de 55 casos (`battery.py`) + 150 casos históricos | **35/55 correctos**, 4 fallos silenciosos por colisión léxica | **[RESULTADO MEDIDO]** | Falla silenciosamente si verbos de suma contienen subcadenas de resta (`di` en "dieron", `regala` en "regalan"). |
| **Razonamiento en Grafos** | `crates/relic_core/src/reasoning_engine.rs:34` (BFS sobre `EpisodicMemory`) | Consultas de alcance entre nodos cargados (`Madrid -> París`) | **Correcto (2 saltos)**; rechazo exacto ante nodos inexistentes | **[RESULTADO MEDIDO]** | El "mundo" conocido está cableado a 13 aristas en `knowledge.rs`. No puede procesar grafos descritos en la consulta. |
| **Roles Semánticos / Orden** | `crates/relic_cli/src/relational.rs:139` (Detección de patrones de cadena y ciclo en grafo) | Premisas de altura: `Carlos es más bajo que Beto; Beto es más bajo que Ana` | **"El más alto es Ana"**; detecta contradicciones cíclicas | **[RESULTADO MEDIDO]** | Solo soporta la pregunta "¿Quién es el más alto?". Si el nombre es compuesto ("María José"), falla por asunción posicional de 1 sola palabra. |
| **Comprensión Lingüística / LLM** | `crates/relic_core/src/model.rs` (Transformer causal 10.03M) | Inferencia con prompts: "Genera un cuento...", "¿Cuál es la capital de Francia?" | **Texto incoherente** (`transcambitidorticiones...`) | **[RESULTADO MEDIDO]** | No posee comprensión semántica ni capacidad de diálogo. |
| **Rechazo Epistémico Honesto** | `crates/relic_cli/src/dispatch.rs:113` y `crates/relic_core/src/rir/validation.rs` | Entradas incompletas (`5 + ?`, `Tengo algunas manzanas...`) | Emite `INCOMPLETE` o `UNSUPPORTED` explícitamente | **[RESULTADO MEDIDO]** | Comportamiento robusto y deseable: no inventa números ante datos faltantes. |

---

## 3. Inventario Completo de Hardcoding

| ID | Regla o Mecanismo | Archivo y Líneas | Efecto sobre la Ejecución | Casos que cubre | Casos que no cubre / Efectos Adversos | Clasificación | Criticidad |
|---|---|---|---|---|---|---|---|
| **H1** | Hechos fijos del mundo (13 aristas: Madrid, Bogotá, Marta, crates) | `relic_cli/src/knowledge.rs:6-23` | Fija el universo de entidades consultables en `EpisodicMemory`. | Solo las 13 aristas predefinidas. | Cualquier grafo externo o suministrado por el usuario ("Si A conecta con B...") es rechazado como `INVALID`. | Hardcoding de contenido | **CRÍTICA** |
| **H2** | Listas de verbos `is_add` (19) y `is_sub` (38) evaluadas con `.contains()` | `relic_cli/src/dispatch.rs:71-81` y `inverted.rs:13-24` | Clasifica la operación por coincidencia de subcadena sin límite de palabra. | Formas verbales exactas listadas. | **Falsos positivos críticos**: "me regalan" contiene "regala" (resta en vez de sumar); "me dieron" contiene "di" (resta en vez de sumar); "las más rojas" contiene "más" (suma en vez de restar). | Hardcoding de lógica léxica | **CRÍTICA** |
| **H3** | Extracción posicional de nombres de entidades (última palabra del sujeto / primera del objeto) | `relic_cli/src/relational.rs:105-106` | Trunca el nombre de la persona o entidad al primer o último token. | Nombres simples ("Pedro", "Juan"). | Nombres compuestos ("María José", "San Martín") se truncan ("José"), provocando colisiones o ambigüedad falsa (`AMBIGUOUS`). | Heurística frágil | **ALTA** |
| **H4** | Extracción posicional de nodos de grafo (primer token tras preposición) | `relic_cli/src/graph.rs:19,26,59,82` | Toma una sola palabra tras `de `, `hasta `, `evitando `. | Nombres de ciudades de un token ("Madrid", "París"). | Ciudades multi-palabra ("Buenos Aires", "La Paz", "New York") quedan truncadas ("Buenos"), fallando la búsqueda. | Heurística frágil | **ALTA** |
| **H5** | Inyección forzada de caso especial para prueba con nombre `"marta"` | `relic_cli/src/graph.rs:74-76` | Si la consulta contiene "marta" y "llegar a", fuerza `initial = "Marta"`. | Casos de prueba específicos del autor. | Código de prueba memorizado e insertado en el parser de producción. | Hardcoding de caso de prueba | **ALTA** |
| **H6** | Prefijos de inventario obligatorios (`tengo `, `hay `, `poseo `, etc.) | `relic_cli/src/dispatch.rs:37,126` | Si no empieza por estas palabras, no entra al flujo de inventario. | Variaciones canónicas directas. | Redacciones naturales ("Al empezar el día disponía de...", "Cuento con...") caen en `UNSUPPORTED`. | Heurística léxica | **MEDIA** |
| **H7** | Prefijos y sufijos adaptados para pruebas sintéticas (`" son"`, `" es"`, `"quítale ... a ..."`) | `relic_cli/src/normalize.rs:23-49` | Normaliza giros idiomáticos específicos. | Plantillas exactas de `templates_l01.rs` (`H_L01_01`, `H_L01_03`). | Evidencia de ajuste circular (*benchmark fitting*): el normalizador se adaptó para pasar las plantillas sintéticas del generador. | Hardcoding de lógica | **MEDIA** |
| **H8** | Mapeo de numerales en palabras a dígitos (0 a 90) | `relic_cli/src/normalize.rs:64-107` | Convierte palabras aisladas ("dos" -> "2", "veinte" -> "20"). | Números simples hasta 30 y decenas hasta 90. | Números compuestos ("treinta y cinco", "ciento dos", "mil") no se convierten y fallan en `INCOMPLETE`. | Hardcoding de contenido | **MEDIA** |
| **H9** | Ruteo por palabras clave a modelo neuronal (`genera`, `cuento`, `historia`) | `relic_cli/src/answer.rs:22-26` | Desvía consultas al modelo neuronal. | Peticiones explícitas de generación. | Peticiones normales con palabras incidentales ("¿Cuánto es 2 + 2 en la historia de Roma?") se desvían al LLM y fallan. | Heurística léxica | **MEDIA** |
| **H10** | Token ID `315` cableado en el bucle de decodificación | `relic_server/src/engine.rs:171`, `generate.rs:32` | Detiene la generación si el token predicho es `315`. | Token de parada en el vocabulario BPE actual. | Constante mágica dependiente del artefacto `tokenizer.json`. | Constante fija | **BAJA** |
| **H11** | Rama de retardo artificial `__TEST_SLOW_QUERY__` en producción | `relic_server/src/router.rs:57-59` | Duerme el hilo 1500 ms si la consulta empieza con esa cadena. | Pruebas internas de timeout HTTP. | Código de testing expuesto en el endpoint de producción. | Código de test en producción | **BAJA** |
| **H12** | Límites y constantes fijas de ejecución (profundidad BFS 8, 48 tokens) | `relic_cli/src/answer.rs:106`, `engine.rs:169` | Restringe el cómputo para evitar desbordamientos. | Consultas estándar. | No configurable dinámicamente. | Configuración fija | **BAJA** |

---

## 4. Veredicto Específico de LLM

- **Arquitectura:** Decoder causal estilo Llama (6 capas, dimensión oculta 288, 6 cabezas de atención con GQA y 2 cabezas KV, RoPE, RMSNorm, activación SwiGLU, vocabulario 8192). Total parámetros medidos: **10,031,328**. [HECHO VERIFICADO]
- **Pesos y Formato:** Existen archivos `artifacts/weights/relic_10m_fp16.safetensors` (20 MB) y `relic_10m_fp32.safetensors` (40 MB), junto con el checkpoint `artifacts/checkpoints/relic_10m_latest.pt` (40 MB). Los 69 tensores requeridos están presentes. [HECHO VERIFICADO]
- **Paridad Numérica PyTorch ↔ Rust:** Comprobada con `tools/verify_parity.py`. La diferencia absoluta máxima entre la inferencia nativa en Rust (`relic_core`) y PyTorch en FP32 sobre secuencias de prueba es de **0.000002** (2e-6). La implementación del motor de tensores en Rust es matemáticamente exacta e idéntica a PyTorch. [RESULTADO MEDIDO]
- **Participación en el Flujo Principal:** **NULA**. Ninguna consulta de razonamiento lógico, cálculo o resolución pasa por el LLM.
- **Calidad Textual y Perplejidad:**
  - Pérdida de entropía cruzada en `data/val.txt`: **8.1820** (Perplejidad: **3576.14**).
  - Como referencia, un modelo uniforme que elija al azar entre los 8192 tokens tendría una pérdida de $\ln(8192) = 9.0109$. El modelo apenas se diferencia del ruido uniforme. [RESULTADO MEDIDO]
  - Las generaciones autorregresivas producen balbuceos morfológicos (`optimizógica autoanalizando optimizógicaando...`) o bucles infinitos de saltos de línea.
- **Veredicto:** El LLM existe como artefacto de cómputo en Rust, pero no aporta ninguna capacidad de lenguaje o razonamiento al sistema.

---

## 5. Veredicto Específico de Machine Learning

- **Dataset de Entrenamiento:** `data/train.txt` (3.3 MB) y `data/val.txt` (369 KB).
- **Contaminación de Datos (Data Leakage Extremo):** Al analizar el texto línea por línea (>40 caracteres), **1552 de las 1552 líneas de validación (100.0%) existen de forma idéntica dentro del archivo de entrenamiento**. [RESULTADO MEDIDO]
- **Naturaleza del Corpus:** El texto de entrenamiento no es lenguaje humano natural ni historias coherentes; consiste en plantillas sintéticas generadas mecánicamente con permutaciones de palabras pseudocientíficas inventadas ("insecuenciiciación", "sobrebuscismo").
- **Trazabilidad del Checkpoint:**
  - El archivo `relic_10m_latest.pt` contiene únicamente los tensores (`state_dict`).
  - No contiene número de paso (`step`), optimizador, valor de pérdida de validación ni configuración de hiperparámetros.
  - La cadena *Datos → Optimización → Checkpoint Certificado* está rota.
- **Benchmarks Neuro-Simbólicos (Candidato C):** El archivo `benchmark_semantic_interpreters.py` evalúa un intérprete preentrenado (`sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`). Dicho modelo **no está descargado localmente** en la caché de Hugging Face. Descargarlo violaría la Regla 11; por ende, se declara formalmente como **[RECURSO AUSENTE / BLOQUEADO]**.

---

## 6. Veredicto de Razonamiento Simbólico y Representación (RIR)

- **¿Qué es RIR (Relic Intermediate Representation)?:**
  Es un esquema formal de AST (`RirDocumentV1`) con tipos semánticos (`InventoryUpdate`, `GraphQuery`, `RuleTransform`).
- **Construcción Real:**
  El documento RIR **no se genera por comprensión semántica del lenguaje**. Se instancia como una estructura de datos ordinaria después de que las funciones de expresiones regulares y cadenas (`dispatch.rs`, `math.rs`) ya han extraído los números y las palabras clave.
- **Validación Semántica (`validate_semantic`):**
  Es un validador determinista legítimo que comprueba si hay división por cero, operandos infinitos/NaN o si los nodos de origen y destino coinciden con los nodos prohibidos (`avoid`).
- **Razonamiento:**
  - Las deducciones en grafos son resueltas mediante **BFS clásico** con cola (`VecDeque`) y conjunto de visitados (`HashSet`).
  - Las deducciones de orden ("más alto que") son resueltas mediante detección de ciclos y búsqueda de nodo maximal en un grafo dirigido acíclico (`TemporalGraph`).
  - Las deducciones aritméticas son resueltas por el algoritmo **Shunting-Yard** implementado en Rust.
- **Veredicto:** El razonamiento es 100% algorítmico clásico y formal. La etiqueta "neuro-simbólico" es inexacta para el estado actual de RELIC, ya que la parte neuronal no interviene en la construcción simbólica.

---

## 7. Resultados Experimentales de Generalización

Se ejecutó una batería de caja negra con **55 casos de prueba independientes** sobre la instancia aislada de `relic_server` en el puerto 5077 (`battery_results.json`).

```text
Total Casos Evaluados:                    55
Casos Correctos (Exactos):                35 (63.6%)
Abstenciones Epistémicas Seguras:         15 (27.3%)
Falsos Positivos Críticos (Silenciosos):   4 ( 7.3%)
Generación Libre sin Criterio:             1 ( 1.8%)
```

### Análisis de los 4 Errores Silenciosos Críticos:
1. **P03**: `"Tengo 10 manzanas y me regalan 5"`  
   → Emitió: `"Ahora tienes 5 manzanas"`. (Esperado: 15). Causa: `regalan` contiene la subcadena `regala` de la lista de resta.
2. **P04**: `"Tengo 12 caramelos, comí 2 y me dieron 3"`  
   → Emitió: `"Ahora tienes 7 caramelos"`. (Esperado: 13). Causa: `dieron` contiene la subcadena `di` de la lista de resta ($12 - 2 - 3 = 7$).
3. **P05**: `"Tengo 10 manzanas, regalo 3 de las más rojas"`  
   → Emitió: `"Ahora tienes 13 manzanas"`. (Esperado: 7). Causa: la cláusula contiene la palabra `más`, activando erróneamente la suma ($10 + 3 = 13$).
4. **D06**: `"¿Cuánto es 2 + 2 en la historia de Roma?"`  
   → Emitió eco repetitivo del prompt vía `neural_model` en vez de calcular 4. Causa: la palabra incidental `historia` activó el ruteo neuronal forzado.

---

## 8. Hallazgos Críticos Ordenados por Impacto

1. **Colisiones por subcadenas que provocan operaciones aritméticas inversas (H2):** Respuestas formalmente marcadas como `VALID` pero con resultados numéricos falsos.
2. **Grafo de memoria episódica restringido a 13 aristas hardcodeadas (H1):** Incapacidad absoluta de razonar sobre datos nuevos proporcionados por el usuario.
3. **LLM disfuncional presentado como componente activo (H9, H10):** El servidor reporta en `/health` y en el arranque que el modo es "COMPLETO con Pesos Safetensors Activos", cuando el modelo solo produce ruido incoherente.
4. **Data leakage del 100% en el dataset de validación del modelo:** Impide cualquier evaluación científica de generalización del LLM.
5. **Silenciamiento de errores en `state.rs:31`:** Se ignora el resultado del mapeo de Safetensors y se reporta disponibilidad sin validación rigurosa.
6. **Ajuste circular de normalizadores a plantillas sintéticas de pruebas (H7).**
7. **Incompatibilidad con entidades y nombres multi-palabra (H3, H4).**

---

## 9. Afirmaciones No Respaldadas

| Declaración en Documentación / Mensajes | Estado Real en el Código | Clasificación |
|---|---|---|
| *"RELIC activo: Modo COMPLETO (Pesos Safetensors Activos)"* | Los pesos se cargan en RAM, pero el modelo no participa en el razonamiento y genera texto disfuncional. | **No respaldada** |
| *"Inferencia unificada RIR con traza de 5 etapas"* | El documento RIR se genera a posteriori como envoltorio del resultado ya parseado léxicamente. | **Parcialmente respaldada** (la traza existe, pero no guía la inferencia). |
| *"Puente Neuro-Simbólico / H1NeuralSemanticParser"* | El archivo `python/relic/semantic_bridge.py` no tiene ninguna conexión con el ejecutable Rust de producción. | **No respaldada en producción** |
| *"Extracción semántica composicional robusta"* | La extracción se basa en listas estáticas de palabras clave y `.contains()`, susceptible a colisiones directas. | **Refutada por experimento** |

---

## 10. Plan de Decisión Definitivo (Erradicación Total de Hardcoding)

### Mandato Estratégico y Regla de Oro
**PROHIBICIÓN TOTAL DE HARDCODING LÉXICO EN EL CÓDIGO FUENTE:**
> Ningún archivo de Rust (`.rs`) contendrá listas de palabras en lenguaje natural, nombres de personas o ciudades en condicionales, prefijos de oraciones, sinónimos de verbos ni búsquedas por subcadenas (`.contains()`).
> Toda comprensión semántica se transfiere a un **modelo neuronal de extracción composicional aprendido** (`relic_slot_filler`), y toda información fáctica proviene de la entrada de la consulta o de bases de conocimiento estructuradas externas.

---

### Principio Arquitectónico Rector: Separación Neuro-Simbólica Rigurosa

```
                        [ Consulta en Lenguaje Natural ]
                                      │
                                      ▼
             ┌──────────────────────────────────────────────────┐
             │       Capa Neuronal de Comprensión (Aprendida)   │
             │     Tokenizador BPE + RelicSlotFiller (BiGRU)    │
             │   - Cabezal de Intención (Intent Classification) │
             │   - Cabezal de Slots Secuenciales (BIO Tagging)  │
             └────────────────────────┬─────────────────────────┘
                                      │ Spans estructurados
                                      ▼
             ┌──────────────────────────────────────────────────┐
             │         Ensamblador Estructural RIR (Rust)       │
             │  Mapeo determinista de spans anotados a AST RIR  │
             │  (Sin listas de palabras ni reglas léxicas)      │
             └────────────────────────┬─────────────────────────┘
                                      │ RIR Validado
                                      ▼
             ┌──────────────────────────────────────────────────┐
             │     Capa Simbólica de Ejecución (Determinista)   │
             │  - Validación de Invariantes Formales            │
             │  - Evaluación Aritmética (Shunting-yard)         │
             │  - Navegación de Grafos (BFS)                    │
             └──────────────────────────────────────────────────┘
```

1. **Capa Neuronal de Comprensión (Traducción Semántica Aprendida):** Un modelo compacto entrenado realiza *Joint Intent Classification + Token Slot-Filling (BIO)*, proyectando la consulta en lenguaje natural directamente a los slots del AST RIR sin ninguna lista de palabras, prefijos o diccionarios en el código Rust.
2. **Capa Simbólica de Ejecución (Razonamiento Verificable):** `relic_core` valida las invariantes (`validate_semantic`) y ejecuta el cómputo exacto (Shunting-yard y BFS) con latencias sub-milisegundo.
3. **Capa de Conocimiento Dinámico:** Los grafos y axiomas del mundo no viven en el código compilado (`knowledge.rs`); se reciben en la consulta o se cargan desde fuentes externas de datos estructurados (JSON).

---

### Matriz de Erradicación de Hardcoding por Archivo y Módulo

| Archivo / Componente | Hardcoding Detectado | Acción | Sustitución Definitiva (Cero Cadenas en Código) |
|---|---|---|---|
| **`relic_cli/src/dispatch.rs`** | Listas de verbos `ADD_WORDS`, `SUB_WORDS`, prefijos `prefixes`, palabras clave `inventory_keywords`, `graph_keywords`. | **ELIMINAR 100%** | Sustituir por inferencia del `RelicSlotFiller`: la intención (`IntentKind`) selecciona la rama, y la dirección de las operaciones la determinan las etiquetas `B-OP_ADD`/`B-OP_SUB` aprendidas. |
| **`relic_cli/src/graph.rs`** | Prefijos de origen/destino (`"partiendo de"`, `"desde"`), exclusión (`"sin pasar por"`), inclusión (`"vía"`), stop-words (`"la"`, `"el"`), caso ad-hoc `"marta"`. | **ELIMINAR 100%** | Ensamblar `QuerySpec` exclusivamente a partir de los spans etiquetados por la red (`B-ORIGIN`, `B-TARGET`, `B-VIA`, `B-AVOID`, `B-NEGATION`). Ninguna búsqueda de prefijos ni nombres en código. |
| **`relic_cli/src/inverted.rs`** | Heurísticas léxicas y patrones inversos manuales. | **ELIMINAR / DEPRECAR** | Absorbido íntegramente por la capacidad del extractor neuronal bidireccional (BiGRU), que procesa tanto orden directo como sintaxis invertida de manera natural. |
| **`relic_cli/src/relational.rs`** | Detección manual de dimensiones (`"alto"`, `"bajo"`, `"rápido"`, etc.) y polaridad por palabras clave. | **ELIMINAR 100%** | La dimensión y polaridad se aprenden como slots semánticos (`B-DIMENSION`, `B-POLARITY_GT`, `B-POLARITY_LT`) entrenados en el corpus relacional. |
| **`relic_cli/src/normalize.rs`** | Gran diccionario estático `match` para números del 0 al 90 en texto. | **ELIMINAR 100%** | Los tokens numéricos son delimitados por `B-INIT_QTY` / `B-OPERAND` y convertidos mediante parser estándar numérico o representaciones aprendidas. |
| **`relic_core/src/knowledge.rs`** | 13 aristas estáticas hardcodeadas en código Rust (Bogotá, Lima, Quito, etc.). | **ELIMINAR 100%** | Erradicar la inicialización estática. El grafo se inicializa vacío y se alimenta de la consulta del usuario o de un archivo de configuración externa (`data/knowledge_base.json`). |
| **`relic_server/src/state.rs`** | Silenciamiento de errores en línea 31 (`let _ = map_weights`) y reporte engañoso de estado. | **CORREGIR** | Propagación estricta de errores con `Result` en el arranque. Si los tensores no cargan o son corruptos, el servidor falla o reporta con precisión su modo operativo. |
| **`relic_server/src/engine.rs`** | Llamada a heurísticas léxicas y ramas artificiales `__TEST_SLOW_QUERY__`. | **ELIMINAR** | Conexión directa del motor al intérprete neuronal en Rust. Eliminación de toda rama o palabra reservada de testing en código productivo. |

---

### Fases de Implementación del Plan de Cero-Hardcoding

#### Fase 1: Generación y Validación del Dataset de Secuencias Slot-BIO
- Generar datos de entrenamiento y evaluación en `data/semantic_compiler/slot_tagged/` con etiquetas BIO exhaustivas (`B-INIT_QTY`, `B-ENTITY`, `B-OP_ADD`, `B-OP_SUB`, `B-ORIGIN`, `B-TARGET`, `B-VIA`, `B-AVOID`, `B-NEGATION`).
- Incluir variaciones sintácticas ricas (directas, invertidas, compuestas, sinónimos) para asegurar que el modelo aprenda la semántica profunda sin requerir reglas léxicas.

#### Fase 2: Entrenamiento y Certificación del Intérprete Neuronal
- Entrenar `RelicSlotFiller` (`python/relic/neural_slot_filler.py`) con PyTorch:
  - Arquitectura: Embedding Layer + 2-layer BiGRU + Linear Intent Head + Linear Slot Head.
  - Criterio de parada: Loss < 0.05, Slot F1 > 98% en conjunto de validación y > 95% en OOD.
- Exportar pesos optimizados en formato `.safetensors` a `artifacts/weights/relic_slot_filler.safetensors`.

#### Fase 3: Inferencia Tensorial Nativa en Rust (`relic_core`)
- Implementar en `relic_core` la estructura de inferencia tensorial hacia adelante para `RelicSlotFiller`:
  - Carga de tensores desde `.safetensors` (sin dependencias externas pesadas).
  - Forward pass matricial (Embedding lookup, BiGRU cell step, Linear projection con Softmax/Argmax).
- Implementar ensamblador estructural `assemble_rir_from_slots`:
  - Recibe la secuencia de tokens y los índices predichos.
  - Reconstruye los spans y genera el AST RIR correspondiente (`ArithmeticPlan`, `QuerySpec`, etc.) **sin comprobar ninguna palabra clave**.

#### Fase 4: Reemplazo Completo en `relic_cli` y `relic_server`
- Sustituir `parse_dispatch_intent` en `dispatch.rs` por la invocación del modelo neuronal en Rust.
- Purgar completamente las listas de palabras (`ADD_WORDS`, `SUB_WORDS`, prefijos, etc.) de `dispatch.rs`, `graph.rs`, `inverted.rs` y `normalize.rs`.
- Purgar las 13 aristas estáticas de `knowledge.rs`.

#### Fase 5: Certificación y Verificación de No-Regresión
- Ejecutar la suite completa de Rust: `cargo test --workspace` (garantizar 100% de tests verdes).
- Re-ejecutar la batería de 55 casos de caja negra: confirmar 0 colisiones léxicas (falsos positivos críticos reducidos de 4 a 0).
- Verificar mediante `git diff` que NO se agregó ninguna cadena fija ni heurística en ningún archivo `.rs`.

---

## 11. Reproducción

Comandos de PowerShell ejecutables en la raíz del repositorio (`RELIC`):

```powershell
# 1. Verificación de paridad numérica exacta PyTorch <-> Rust (max diff: 0.000002)
python tools/verify_parity.py

# 2. Medición de pérdida del modelo en validación y detección de fuga de datos
python C:\Users\AltaPedagogia1\.gemini\antigravity-ide\brain\809d714c-a14c-45ea-84a9-637fe44474b0\scratch\llm_audit.py

# 3. Compilación aislada y ejecución del servidor en puerto de auditoría
$env:CARGO_TARGET_DIR="C:\Users\AltaPedagogia1\.gemini\antigravity-ide\brain\809d714c-a14c-45ea-84a9-637fe44474b0\scratch\target"
cargo build --offline --release -p relic_server
& "$env:CARGO_TARGET_DIR\release\relic_server.exe" 5077

# 4. Ejecución de la batería de generalización de 55 casos de caja negra
python C:\Users\AltaPedagogia1\.gemini\antigravity-ide\brain\809d714c-a14c-45ea-84a9-637fe44474b0\scratch\battery.py C:\Users\AltaPedagogia1\.gemini\antigravity-ide\brain\809d714c-a14c-45ea-84a9-637fe44474b0\scratch\battery_results.json

# 5. Ejecución completa de la suite de pruebas unitarias e integración en Rust (30 suites, 100% OK)
cargo test --offline --release --workspace
```
