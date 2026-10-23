# ESTADO Y ORIENTACION (2026-10-22)

> Documento auto-exigido de analisis y medicion de RELIC. Cada cifra lleva etiqueta:
> [MEDIDO] = corrida local verificable hoy; [HISTORICO] = cifras congeladas de checkpoints previos;
> [ESTIMADO] = inferencia explicita con aritmetica visible; [REFERENCIA] = de terceros, cita obligatoria;
> [PENDIENTE DE EJECUTAR] = experimento scripteado pero no corrido en este hardware;
> [DESIGN] = decision de conjunto/metodologia del agente (no cifra de verdad).
>
> Hardware: CPU solamente (2 hilos en `/proc`/`%PROCESSOR_COUNT%`: PyTorch reporta 2 threads), sin GPU.
> La estrategia de medicion local es: ejecutables Rust en `--release`, Python para analisis y baselines baratos;
> LLMs externos SOLO via plantilla Colab [PENDIENTE DE EJECUTAR]. Nada local inventa GPU.

## 1. Que queda medido del pipeline determinista Rust (T1)

Harness: `crates/relic_cli/examples/eval_dump.rs` (15 repeticiones/caso, binario `--release`,
memoria fresca por repeticion, `load_default_knowledge`). Analisis: `tools/eval/analyze_dump.py`
(taxonomia de error (i)-(v) y abstencion). Dumps crudos: `reports/eval/dump_release_*.json`.

| Suite (casos) | [MEDIDO] precision | precision ext.* | determinismo | lat p50 | lat p95 max | RSS max |
|---|---|---|---|---|---|---|
| known_failures (14) | 78.6% (11/14) | 92.9% (+2 abstenciones correctas) | 100% | 42 us | 234 us | 3,924 KB |
| unseen_frozen_v2, CONGELADO (16) | 87.5% (14/16) | 100% (+2 abstenciones correctas) | 100% | 41 us | 949 us | 3,956 KB |
| unseen_independent v1 CONTAMINADO (7) | 42.9% | 57.1% | 100% | 43 us | 181 us | 3,852 KB |
| semantic_queries_es (14) | 35.7% (5/14) | 35.7% | 100% | 47 us | 298 us | 3,920 KB |
| semantic_adversarial_es (16) | 25.0% (4/16) | 25.0% | 100% | 90 us | 1,256 us | 3,960 KB |
| probe abstencion interna, [DESIGN] (30) | 6.7% intento directo | 96.7% (27 abstenciones correctas + 2 correctos) | 100% | 57 us | 2,023 us | 4,140 KB |

*precision ext.* = correcto + abstencion epistemica cuando el oro la pide (INVALID/INCOMPLETE).
El "aimshoot directo" de la sonda de abstencion es EXPECTED: casi todo ahi debe abstenerse; medirlo
como precision classica seria malinterpretar. Las latencias son 2-3x menores a las [HISTORICAS]
de `tests/unseen_frozen_v2.json` baseline (max 4,780 us) porque ahora el despacho es la ruta de
reglas sin rechazo por re-escaneo; compararlo con [HISTORICO] requiere el mismo hardware.

Hallazgos puntuales del lado Rust [MEDIDO]:
1. `unseen_v1`: los 3 fallos son numerales en palabras NO fuera del subconjunto codificado
   ("veintitres mas cuatro", "diez"/"tres", "sumame") → abstencion innecesaria (v).
2. `semantic_queries/adversarial`: el parser de reglas Rust solo cubre ~1 familia sintactica de
   grafo ("...de X a Y...") y la de dependencias; 9/14 y 12/16 terminan en (v) o (i). Ningun
   caso con `via/avoid/negacion` --| ejemplo: adv_waypoint_01 "Puede Marta llegar...".
3. known_failures 1 "fallo" (iii) es oro mal definido: el caso Bogota→Madrid evitando Tokio;
   el sistema abstiene por 'Tokio' no existente (comportamiento epistemicamente correcto pero
   contradice la aguja "Bogota" del oro). No es regresion; es definicion de oro incompleta.
4. RSS max del pipeline completo: bajo 4,2 MB en todas las suites.

## 2. Rendimiento linguistico: tokenizador y checkpoint 10M (T1 b)

[MEDIDO] `reports/eval/loss_validacion.json` + `artifacts/tokenizer/tokenizer.json`:
- Tokenizador BPE: 4,262 tokens (+3 added); ln(4262) = 8.3575 (referencia uniforme).
- El checkpoint `relic_10m_latest.pt` (step=10 [MEDIDO]: hay que documentar que es un
  checkpoint con 10 pasos) sobre val.txt (63,987 tokens, secuencias de 256):
  val_loss = 8.8962 nats/token, ppl = 7304.
- Referencias baratas en el MISMO val: uniforme-tokenizador = 8.3575; uniforme-vocab-modelo
  (8192) = 9.0109; un BIGRAMA (Laplace +1, entrenado solo en train.txt) = 1.4768 (ppl 4.4).
  [MEDIDO]: el checkpoint grande actual es PEOR que uniforme-tokenizador y destruido por un
  bigrama. No existe ninguna evidencia local de que el checkpoint transmita lenguaje.
- Mismas herramientas: si re-entrenabamos fresco (smoke 20 pasos [MEDIDO]
  `reports/llm_comparison_report.json`), loss train 8.56 → 3.17 / val 2.96 en 20K tokens:
  el checkpoint guardado NO es el artefacto entrenado. Hay que descartarlo o re-entrenar.

## 3. Metrica congelada del nucleo H1 (102,720 params) (T2)

[HISTORICO] `artifacts/checkpoints/relic_core_h1/metrics.json` (CONGELADO, no re-entrenable):
dyck 100%/100%, graph 53.5%/55.0%(OOD), rules 10%/0%. Confirmado RUNNING hoy [MEDIDO la suite]:
`tests/unseen_frozen_v2` = 14/16 + 2 abstenciones correctas = 100% con precision extendida.
Interpretacion honesta: H1 es un nucleo DETERMINISTA fuerte solo en grammar polacas
(dyck/graph hasta ~10 nodos) y NO extiende a reglas (10%) ni lenguaje libre.

## 4. Matriz de parsers semantico (Python) y su invalidacion por codigo (T3)

[MEDIDO, REPORTADO POR LA HERRAMIENTA] `reports/semantic_error_matrix_report.json`
(`tools/benchmark_semantic_error_matrix.py`), sobre semantic_adversarial_es (16 casos):
| Metodo | Tupla e2e | Intencion | Origen | Destino | Latencia media |
|---|---|---|---|---|---|
| A Naive-Regex | 6.2% | 37.5% err | 56.2% err | 62.5% err | 285 us |
| B Grammar-Rules | 100% | 0% | 0% | 0% | 531 us |
| C "H1 Neural" | 100% | 0% | 0% | 0% | 39 us |

[INVALIDACION POR CODIGO, no cifra]: "Metodo C" no es un modelo neural: en
`python/relic/semantic_bridge.py` la clase `H1NeuralSemanticParser.parse` DELEGA en
`GrammarRuleParser` con un comentario "simulación de cuello de botella"; su 100% es B
re-etiquetado. Tratar la fila C como NO-VALIDA hasta re-implementarla con PyTorch
checkeando pesos de H1 congelado (o usar el ruina estable relativa). Escencial para no
montar claims sobre C.

## 5. Abstencion: sonda interna [DESIGN] y su limite

Sonda: `tools/eval/probe_abstencion_interno.json` (30 casos compuestos por el agente;
NOTA: NO sustituye a un conjunto de terceros): 27/30 abstiene correctamente frente
entidades desconocidas + dominios externos; 1/30 acepta de mas ("resta 5 y 8" → formula
declarada) y 2/30 son consultas contrastadas VALID. Con la definicion "interpretacion
transparente declarada es admisible", la sonda quedaría en 29/30+1. El punto cientifico
no es el 100%: es que el pipeline se MUESTRA CERCA de abstencion-generosa, con una tasa
de falsa aceptacion de ~3% [MEDIDO] en 3 tipos de cebo. Requiere pagar el precio en
cobertura de parafraseos (seccion 1, punto 2) y terceros lo mas pronto posible.

## 6. Donde esta el cuello de botella cuantificado (T3 respuesta)

1. COBERTURA SINTATICA DEL DESPACHO RUST: 4 correcto/16 en adversarial vs 100% del parser
   de reglas Python en el MISMO banco [MEDIDO]. El motor determinista no falla por logica
   (cuando parsea no falla ninguna ejecucion): falla por NO LLEGAR a construir el RIR.
   Anchura sintactica = literalmente numero de plantillas escritas a mano: lineal, no
   composicional. [MEDIDO, via dumps]
2. NUMERALES EN PALABRAS: un vocabulario cerrado cubre solo hasta ~12 y desde decimos
   las tablas manuales; respuestas failures 3/7 v1 [MEDIDO].
3. CHECKPOINTS LANGUAGE: el 10M guardado no vale (seccion 2). Eligir drop/re-train; el
   potencial computacional del 10M no ha sido, en absoluto, demostrado localmente.
4. EVIDENCIA EXTERNA FALTANTE: no hay ni un dato LLMexterno [PENDIENTE DE EJECUTAR].
   Mas arriba del cuello de botella: RELIC-H1 vs LLMs BASICOS a otras similitudes no
   comparar AUN cierto, y se debe montar plantilla Colab (Qwen3-0.6B, temp 0, grader
   needle-match, 3 repeticiones) con hardware del autor NO involucrado.
5. Reclamacion NPCs del README a suavizar [DESIGN]:
   a) "RELIC rinde 40-90x mas rapido que GPT": sin datos externos ni citas → cambiar a:
      deterministic pipeline vs campana LLM en hardware del autor con Idiation
      pledge [PENDIENTE DE EJECUTAR].
   b) "80-90% de reduccion de requisitos de datos": reclamo que no acompa�amos usando
      [MEDIDO] trial baselines; convertir en claim silencioso!
   c) ln(vocab): voy CD a 65% NOTAS CAPACIDAD/OVERHEAD [ESTIMADO], [
      rest label [PENDIENTE]] antes que acotar porcentajes.
   d) "Rule ML automatico": no existe implementacion in-repo; pedir, no declarar.

## 7. Orientacion de investigacion (T4 respuesta): camino corto por dentro

Dado que el parser de reglas Python cubre 100% del banco adversarial y el despacho Rust
~25-35% [MEDIDO], la decision arquitectonica mas rentable NO es ampliar plantillas de
cadena en Rust: es construir un PUENTE H1->RIR que produzca el QuerySpec / plan aritmetico
directamente desde el nucleo congelado H1, con post-proceso SIN cambios del ejecutor
determinista (el determinismo queda aislado: no se re-entrena nada del corpus). Ruta
concreta guiada por evidencia:
   1. [DESIGN] Entrenar el SemanticCompiler semantico: input latente H1 -> campos del RIR.
   2. [PENDIENTE DE EJECUTAR] Plantilla Colab/Kaggle (Qwen3-0.6B base, temp 0, 3
      repeticiones, grader por aguja: needle-match + ruta requerida; resultados a CSV).
   3. [DESIGN] Definir "tercero": 50 frases EN/ES generadas por NADIE del repo, versionadas
      en git con firma sha256 del archivo, con baseline medido por pieza del banco Gold.

## 8. Verificacion y cumplimiento

- Estado del repo: `cargo test --workspace` verde, incluida la suite congelada 16/16 [MEDIDO].
- Herramientas de medicion con <180 lineas por modulo; dumps crudos en `reports/eval/`.
- Cambio de `.gitignore`: se versionan JSONs chicos de `reports/` y solo ellos (los pesos
  y eventos grandes siguen excluidos).
- Claims del README: corregir segun seccion 6 (a)-(d), sin tocar tests congelados ni anadir
  funcionalidad inmediata al motor.

## 9. Veredicto corto final

- El pipeline determinista H1->RIR->ejecutor es 100% determinista [MEDIDO] y abstiene
  generosamente (~97% correcto en sonda interna [DESIGN]); a cambio su cobertura sintactica
  es ~25-35% en bancos de parafraseo [MEDIDO]. NO falla por logica cuando parsea: falla por
  no parsear (0 ejecuciones con RIR malo en adversarial Rust).
- El checkpoint 10M guardado es inutilizable como LM (peor que uniforme supuesto del vocab;
  un bigrama lo aplasta 8.90 vs 1.48 [MEDIDO]) y las suites del README citan agradecimiento
  a nombres del mismo pipeline [HISTORICO] cuya amplificaron 40-90x no esta medida hoy.
- Solicitud VERIFICABLE apuntada a terceros: los 4 bloques y las plantillas Colab queden
  listos y marcados [PENDIENTE DE EJECUTAR] porque no hay hardware externo aqui.

