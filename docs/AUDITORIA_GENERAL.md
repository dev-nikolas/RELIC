# AUDITORÍA GENERAL DE SOFTWARE Y MACHINE LEARNING
**Proyectos:** RELIC & Simple_Domain  
**Fecha de Auditoría:** 2026-10-25 / 2026-10-26  
**Rol:** Auditor Independiente de Software y ML  
**Tipo de Intervención:** Exclusivamente Analítica (Sin modificaciones de código fuente ni refactorizaciones)

---

## 1. RESUMEN EJECUTIVO (Una Página)

### 1.1. Qué está sólido [LEÍDO: crates/relic_core/src/lib.rs:1-120]
1. **Motor Determinista Simbólico de RELIC:** [MEDIDO: `cargo test --workspace` -> 129 passed en 0.15s]. El razonador simbólico en Rust (`relic_core` y `relic_cli`) resuelve con exactitud matemática operaciones aritméticas básicas, compuestas y recorridos de grafos con restricciones (`relic_core::bfs`).
2. **Conformidad Estricta de RIR:** [MEDIDO: `crates/relic_core/tests/rir_conformance_tests.rs` -> 14 passed]. El validador rechaza con precisión divisiones por cero, operandos no finitos (NaN/Inf) y documentos incompletos.
3. **Servicio HTTP Liviano y Autónomo:** [MEDIDO: `crates/relic_server/src/main.rs:1-69`]. Servidor HTTP multihilo montado sobre `tiny_http` que levanta en <2 ms, soporta inferencia de `/api/query` y streaming SSE sin depender de Python.
4. **Suite de Integración y Tests Unitarios:** [MEDIDO: 129 pruebas automáticas en Rust y 65 pruebas en pytest para Simple_Domain pasando al 100%]. La regresión histórica se mantiene verde.
5. **Compilación de Producción Frontend:** [MEDIDO: `npm run build` en `Simple_Domain/frontend` -> `dist/assets/index-vu8_9OyS.js` 251.43 kB generado en 8.27s sin errores de tipos TypeScript].

### 1.2. Qué NO está sólido y por qué [LEÍDO: Simple_Domain/backend/src/simple_domain/jobs/sandbox_runner.py:57-120]
1. **Evaluación Sesgada e Inválida en Sandbox Lab:** [LEÍDO: `sandbox_runner.py:62-70`]. El runner extrae secuencialmente los primeros casos del archivo `curriculum_dataset_v2_20k.jsonl`, evaluando 100% datos del split `train` y 0% del split `holdout_template`.
2. **Parámetro `seed` Inoperante:** [MEDIDO: `audit_sandbox_reproducibility.py`]. El parámetro `seed` es ignorado en `load_cases()`. Correr con semilla 42 o semilla 999 produce exactamente los mismos casos.
3. **Falsa Atribución y Enmascaramiento de Fallos (Riesgo Crítico de Procedencia):** [LEÍDO: `relic_client.py:23-38` y `sandbox_runner.py:93`]. Si `relic_server` no está levantado, `relic_client.py` enmascara la caída y ejecuta silenciosamente `relic_cli` vía CLI. Además, `sandbox_runner.py` ignora el sistema seleccionado (`rules`, `slot_filler`, `model`), atribuyendo métricas al sistema equivocado.
4. **Traza Sintética / Decorativa:** [LEÍDO: `relic_client.py:73-88` y `engine.rs:61-71`]. En el fallback de CLI, la traza reporta latencias hardcodeadas ficticias (`0.12 ms`, `0.25 ms`, `0.38 ms`). En el servidor Rust, la etapa `rir` sólo convierte el enum de intención a texto sin emitir un AST ni validar conformidad RIR real.
5. **Colapso de E/S en Persistencia de Sandbox:** [DERIVADO: $O(N^2)$ escrituras en disco]. Cada evento emitido (4 por episodio) dispara `append_event`, el cual lee y reescribe completamente `events.json` desde disco, provocando gigabytes de reescritura innecesaria en corridas medianas.
6. **Manejo de Errores y Fuga de EventSource en Frontend:** [LEÍDO: `SandboxLab.tsx:53-73`]. Si el usuario desmonta la vista durante una corrida, el `EventSource` no se destruye. Si la conexión se pierde, la interfaz se detiene silenciosamente sin avisar al usuario.
7. **Ausencia de Artefactos del Modelo Autónomo:** [MEDIDO: `reports/eval/holdout_75_per_case.json` y `notebooks/relic_colab_train_fase_a.ipynb`]. El modelo neuronal relic-10M obtiene 0/50 Exact Match por rechazo sintáctico total; en el repositorio no existe registro de curvas de pérdida ni métricas de convergencia del Colab.

---

## 2. ESTADO DEL REPOSITORIO (FASE 1)

### 2.1. Git Status y Commits [MEDIDO: comandos git ejecutados sobre ambos repositorios]
- **RELIC:**
  - Rama: `master` [MEDIDO: `git -C RELIC status`].
  - Estado local: Limpio (sin archivos modificados ni sin seguimiento sin commit) [MEDIDO: `git status -s` -> salida vacía].
  - Estado remoto: **1 commit por delante de `origin/master`** (`63eaee931cafcaed43d2575e37e80a2a024e2892`) [MEDIDO: `git log origin/master..HEAD`].
  - Etiqueta de sincronización: **[SOLO LOCAL]** para todo el contenido del commit `63eaee9`.
- **Simple_Domain:**
  - Rama: `main` [MEDIDO: `git -C Simple_Domain status`].
  - Estado local: Limpio [MEDIDO: `git status -s` -> salida vacía].
  - Estado remoto: **Al día con `origin/main`** (Commit `7031c4db4a4260ad139bf0e9e44677eb693e32f3` pusheado) [MEDIDO: `git log origin/main..HEAD` -> 0 commits].

### 2.2. Verificación de Entregables Clave
1. **Sandbox Lab (Backend y Frontend):**
   - Backend (`sandbox_runner.py`, `sandbox.py`, `routes/sandbox.py`): Commiteado y pusheado en Simple_Domain `7031c4d` [LEÍDO: Simple_Domain commit 7031c4d].
   - Frontend (`SandboxLab.tsx`, `SandboxChart.tsx`, `SandboxEpisodeViewer.tsx`): Commiteado y pusheado en Simple_Domain `7031c4d` [LEÍDO: Simple_Domain commit 7031c4d].
   - Script de prueba de humo (`tools/smoke_test_sandbox_lab.py`): Commiteado en RELIC `63eaee9`, **[SOLO LOCAL]** [LEÍDO: RELIC commit 63eaee9].
   - Documentación (`docs/SANDBOX_LAB.md`): Commiteado en RELIC `63eaee9`, **[SOLO LOCAL]** [LEÍDO: RELIC commit 63eaee9].
2. **Extensión "trace" en `/api/query`:**
   - Implementado en `crates/relic_server/src/types.rs:17-48`, `crates/relic_server/src/engine.rs:20-108` y `crates/relic_server/src/router.rs:95-103`.
   - Estado: Commiteado en RELIC `63eaee9`, **[SOLO LOCAL]** [LEÍDO: RELIC git log].
3. **Insignias de Procedencia en UI:**
   - Implementado en `Simple_Domain/frontend/src/components/ChatMessageBubble.tsx:125-145` y `backend/src/simple_domain/service/relic_client.py:84`.
   - Estado: Commiteado y pusheado en Simple_Domain `7031c4d` [LEÍDO: Simple_Domain commit 7031c4d].

---

## 3. VERIFICACIÓN DE AFIRMACIONES DEL ÚLTIMO REPORTE (FASE 2)

### 3.1. Re-ejecución de Suites y Salidas Reales
1. **`cargo test --workspace`:**
   - **Salida Real:** [MEDIDO: 129 passed; 0 failed; 0 ignored en 0.15s].
   - **Desglose por Crate:**
     - `relic_cli`: 55 pruebas (unitarias: 5, suites de integración: 50) [MEDIDO].
     - `relic_core`: 42 pruebas (unitarias: 12, `bfs_constraint`: 10, `core_engine`: 6, `rir_conformance`: 14) [MEDIDO].
     - `relic_mcp`: 8 pruebas (`protocol_tests.rs`: 8) [MEDIDO].
     - `relic_server`: 20 pruebas (`server_integration`: 9, suites congeladas y regresión: 11) [MEDIDO].
     - `relic_tokenizer`: 4 pruebas [MEDIDO].
     - Doc-tests: 0 pruebas [MEDIDO].
     - **Total:** 55 + 42 + 8 + 20 + 4 = 129 pruebas [DERIVADO].
   - **Reconciliación de discrepancias (87 vs 124 vs 129):**
     - El reporte antiguo de 87 pruebas correspondía a la etapa previa a la incorporación de `relic_mcp` (8 pruebas), sin los tests de servidor `server_integration_tests.rs` (9 pruebas), sin `relic_tokenizer` (4 pruebas) y con menos suites en `relic_cli` [DERIVADO].
     - La corrida independiente de 124 pruebas se ejecutó en el commit `66d579d`, antes de agregar `crates/relic_core/tests/core_engine_tests.rs` (6 pruebas) y ajustar un test interno [DERIVADO: 124 - 1 + 6 = 129].
2. **`pytest`:**
   - En `RELIC`: [MEDIDO: `pytest` -> Exit Code 1]. Falla inmediatamente en la recolección con `ModuleNotFoundError: No module named 'torch'` en `tests/test_rir_conformance.py` y `tools/test_consensus_crt.py` debido a que el entorno global y el venv no tienen PyTorch instalado.
   - En `Simple_Domain`: [MEDIDO: `Simple_Domain/.venv/Scripts/python -m pytest` -> Exit Code 0]. 65 passed, 105 warnings en 18.80s.
3. **`npm run build`:**
   - En `Simple_Domain/frontend`: [MEDIDO: `npm run build` -> Exit Code 0 en 8.27s]. Genera `dist/index.html` (1.10 kB), `dist/assets/index-D8S8Wu4m.css` (32.12 kB) y `dist/assets/index-vu8_9OyS.js` (251.43 kB).
4. **`tools/smoke_test_sandbox_lab.py`:**
   - [MEDIDO: Ejecutado con Python de Simple_Domain -> Exit Code 0 en 12.14s]. Valida la corrida `sb_5697ae13`: 5 episodios, 21 eventos SSE recibidos, Exact Match 40.0% (2/5), Wilson 95% `[11.8%, 76.9%]`, Falsa Aceptación 0.0%, Recompensa media -2.60.

### 3.2. Discrepancia de Hashes de Pesos de Modelo
- **Hashes Identificados en el Proyecto:**
  - Hash A (`328d5e70a41c6a1e3a26e01d7c04d6e144e8710b4589f2a40c4000e2cb5b29dc`):
    - [MEDIDO: `Get-FileHash` sobre `RELIC/data/relic_10m_trained.safetensors` y `RELIC/artifacts/weights/relic_10m_trained.safetensors`].
    - Tamaño: 40,132,704 bytes (40.1 MB en float32) [MEDIDO].
    - Registrado en: `docs/FASE_A.md:198` y `docs/SANDBOX_LAB.md:71`.
  - Hash B (`2f8485429c7e2ecf349336a0054ae49b64f940be8aa34d1d60d15daa987a5f5b`):
    - [LEÍDO: `tools/download_weights.py:10` y `reports/fase_a_closure_report.json:6`].
    - Ficha como "checkpoint oficial esperado para descarga".
- **Explicación de la Discrepancia:** [LEÍDO: `reports/fase_a_closure_report.json:7`]. El hash `2f8485...` quedó fijado en el script de descarga como la referencia remota esperada; el archivo presente físicamente en el disco (`328d5e...`) corresponde al checkpoint exportado tras el intento de entrenamiento en CPU/Colab.
- **Rendimiento de Ambos Checkpoints:** [LEÍDO: `reports/fase_a_closure_report.json:7`]. Ambos checkpoints obtienen 0/50 Exact Match en resolubles y 100% de rechazo sintáctico por incapacidad de generar RIR válido autónomamente.

### 3.3. Resultados de Entrenamiento en Google Colab
- **Búsqueda en el Repositorio:** [MEDIDO: Búsqueda exhaustiva en `RELIC/notebooks/`, `RELIC/reports/` y `RELIC/data/`].
- **Cuaderno Colab:** [LEÍDO: `RELIC/notebooks/relic_colab_train_fase_a.ipynb`]. Consta de 8 celdas de código, todas con `outputs: []` (el cuaderno se guardó limpio sin salidas).
- **Curvas de Pérdida y Desglose Dentro/Fuera de Plantillas:** **NO ESTÁN EN EL REPOSITORIO** [NO VERIFICADO: no existen archivos CSV, JSON ni gráficos con la curva de pérdida de las épocas ni la tasa de acierto del modelo fuera de plantillas; el único registro de validación es `reports/eval/loss_validacion.json` en el paso 10 con `val_loss_nats_token: 8.8962`].

---

## 4. AUDITORÍA DEL SANDBOX LAB - BACKEND (FASE 3)

### a) Subconjunto Evaluado por SandboxRunner [LEÍDO: sandbox_runner.py:33, 57-71]
- El runner apunta directamente a `RELIC/data/curriculum_dataset_v2_20k.jsonl`.
- [MEDIDO: Análisis del dataset]:
  - Total registros: 22,500 filas.
  - Registros `train`: 18,000 filas (filas 0 a 17,999).
  - Registros `holdout_template`: 4,500 filas (filas 18,000 a 22,499).
- [LEÍDO: `sandbox_runner.py:62-70`]: La función `load_cases(level, seed, limit=max_episodes)` lee el archivo línea por línea desde el inicio y se detiene cuando alcanza `limit` (por defecto 20 a 30 episodios).
- **Veredicto Cuantificado:** El runner evalúa **100% split `train`** y **0% split `holdout`** para cualquier corrida con `max_episodes <= 18,000`. Reportar estas métricas como evaluación de generalización es **metodológicamente engañoso**.

### b) Definiciones Métricas vs. `fase_a_closure_report.json`
- **Rechazo Sintáctico (`syntax_rejection`):** [LEÍDO: `sandbox_runner.py:145-151`]. **NO EXISTE** como métrica en `SandboxRunner`. Mientras que el reporte de Fase A lo define como "Salida vacía, truncada o no parseable en JSON/RIR" y reporta 1.0000 para el modelo neuronal, el runner no lo cuenta en `final_metrics`.
- **Exact Match (EM):** [LEÍDO: `sandbox_runner.py:110-116`]. En el runner se implementa como un regex numérico laxo (`abs(float(m_nums[0]) - float(o_nums[0])) < 1e-4`) o coincidencia de subcadena (`oracle.lower() in text.lower()`). Ignora completamente el AST de RIR (`gold_rir` se lee en la línea 85 y nunca se usa).
- **Falsa Aceptación (FA):** [LEÍDO: `sandbox_runner.py:108` vs `117-120`]. En el runner, `is_fa` únicamente se activa si el caso es epistémico y el sistema no se abstiene. En casos resolubles donde el sistema da un número equivocado, el runner asigna recompensa $-\lambda$, pero **no incrementa `fa_count`** (`fa_count` permanece en 0). Por ende, `false_acceptance_rate` está subestimado y no mide errores de cálculo en resolubles.

### c) Fallback Local y Enmascaramiento de Fallos (Procedencia)
- [LEÍDO: `relic_client.py:23-38`]:
  ```python
  try:
      ... urllib.request.urlopen(req, timeout=3.0) ...
  except Exception:
      return self._query_cli_fallback(query_text, enable_trace=enable_trace)
  ```
  Si `relic_server` no está levantado, `relic_client.py` atrapa cualquier excepción y llama a `cargo run --bin relic_cli`.
- [LEÍDO: `sandbox_runner.py:93`]: `res = self.client.query(prompt, enable_trace=True)`. El parámetro `system` recibido en `run(..., system: str, ...)` **nunca se envía al cliente**.
- **Impacto:** Si un usuario selecciona evaluar el sistema `model` o `slot_filler`, pero `relic_server` está apagado, el sistema ejecuta las reglas deterministas de `relic_cli` y almacena la corrida en `sandbox_runs.json` con `"system": "model"`. La procedencia queda falseada.
- **Registro Real:** Solo queda registrado indirectamente si se inspecciona el campo `route: "relic_cli_direct"` dentro del episodio, pero el manifiesto principal atribuye los resultados al sistema configurado.

### d) Recompensa Asimétrica y Wilson CI
- **Recompensa Asimétrica:** [LEÍDO: `sandbox_runner.py:104-121`].
  - Acierto resoluble: $+1.0$.
  - Abstención correcta (AC): $+0.50$.
  - Abstención innecesaria (AI): $-0.20$.
  - Falsa aceptación epistémica: $-\lambda$ (por defecto $-5.0$).
  - Error sintáctico: $-2.0$.
  - Coincide con la formulación teórica de `docs/SANDBOX_LAB.md:49-62`.
- **Intervalo de Confianza de Wilson (95%):** [MEDIDO: `tools/audit/audit_sandbox_reproducibility.py`].
  - Implementación en `sandbox_runner.py:18-24`:
    - $k=8, n=50 \rightarrow [0.0834, 0.2851]$ (Coincide exactamente con el reporte oficial).
    - $k=21, n=50 \rightarrow [0.2938, 0.5577]$ (Coincide).
    - $k=25, n=25 \rightarrow [0.8668, 1.0000]$ (Coincide).
  - La fórmula matemática es exacta e independiente.

### e) Seguridad y Robustez del Backend
1. **Validación de Entradas:** [LEÍDO: `routes/sandbox.py:17-23`]. `level` y `system` son strings libres sin validación `Enum` en Pydantic. Se puede enviar `system: "destruir"` y el backend lo acepta.
2. **Riesgo de Path Traversal:** [LEÍDO: `repositories/sandbox.py:22-25`]. `_run_dir(run_id)` concatena directamente `self.sandbox_dir / run_id`. Si un atacante llama a `GET /v1/sandbox/runs/..%2F..%2Fetc`, puede consultar rutas arbitrarias del disco. Además, `_run_dir` ejecuta `mkdir(parents=True, exist_ok=True)` en lecturas `get_run()`, permitiendo crear carpetas arbitrarias con peticiones GET no existentes.
3. **Concurrencia sin Límites:** [LEÍDO: `routes/sandbox.py:54-63`]. `create_sandbox_run` lanza un `asyncio.create_task` por cada llamada sin semáforo ni límite de tareas simultáneas. Un bombardeo de peticiones agotaría los descriptores de archivo y la memoria.
4. **Fallo a Mitad de Corrida:** [LEÍDO: `sandbox_runner.py:80-142`]. El bucle `for` no está protegido por un bloque `try...except`. Si un caso arroja una excepción no controlada, la corrida queda en estado `"running"` para siempre en la base de datos y el cliente SSE se cuelga.
5. **Degradación de E/S por Reescritura Cuadrática:** [LEÍDO: `repositories/sandbox.py:97-103`]. En cada evento emitido, `append_event` lee el archivo `events.json` completo y lo reescribe entero. Para 1.000 episodios (4.000 eventos), esto requiere reescribir 4.000 veces un archivo incremental, generando escrituras de orden $O(N^2)$ en disco.

### f) Reproducibilidad [MEDIDO: tools/audit/audit_sandbox_reproducibility.py]
- **Corrida 1 (`seed=42`) vs Corrida 2 (`seed=42`):** Idénticas (100% reproducible en prompts y recompensas).
- **Corrida 1 (`seed=42`) vs Corrida 3 (`seed=999`):** **IDÉNTICAS.**
- **Causa:** [LEÍDO: `sandbox_runner.py:57-71`]. El parámetro `seed` figura en la firma de `load_cases(self, level: str, seed: int, limit: int = 30)`, pero dentro del cuerpo de la función **nunca se utiliza** (no hay muestreo ni barajado aleatorio).

---

## 5. BATERÍA DE SONDEO COGNITIVO RELIC (FASE 4)

[MEDIDO: `tools/audit/run_reasoning_probe.py` contra `http://127.0.0.1:5000/api/query` con `enable_trace: true`].  
Reporte completo guardado en: `reports/audit/reasoning_probe.json`.

### 5.1. Métricas Globales de Sondeo (N=38)
- **Total Casos:** 38 consultas.
- **Resueltos con Éxito (`status: VALID`):** 12 (31.6%) [MEDIDO].
- **Abstenciones Estructuradas (`status: INCOMPLETE / UNSUPPORTED`):** 25 (65.8%) [MEDIDO].
- **Errores / Irresolubles (`status: INVALID`):** 1 (2.6%) [MEDIDO].
- **Respuestas Falsas Seguras (Alucinaciones):** 0 (0.0%) [MEDIDO]. El sistema se abstiene antes de inventar.

### 5.2. Hallazgos por Categoría
1. **Aritmética Estándar:**
   - `"cuánto es 15 por 3"` $\rightarrow$ VALID: `Entendí: 15 * 3 = 45. El resultado es 45.`
   - `"veintiuno más nueve"` $\rightarrow$ VALID: `Entendí: 21 + 9 = 30. El resultado es 30.`
   - `"parce cuánto es 7 más 8"` $\rightarrow$ VALID: `Entendí: 7 + 8 = 15. El resultado es 15.`
   - `"10 / 0"` $\rightarrow$ INCOMPLETE: `Rechazo semántico RIR: Operación no permitida o división por cero.`
   - `"resta 3 y 10"` $\rightarrow$ VALID: `Entendí: 3 - 10 = -7.` (Interpreta estrictamente el orden secuencial).
2. **Memoria y Grafos:**
   - `"¿Depende el crate relic_cli del paquete memmap2?"` $\rightarrow$ VALID: `Sí, es posible alcanzar memmap2 desde relic_cli (Ruta: relic_cli -> relic_core -> memmap2, 2 saltos).`
   - `"¿Depende el crate Relic_Cli del paquete memmap2?"` $\rightarrow$ VALID (Normalización insensible a mayúsculas funciona).
   - `"¿Depende el crate relic_cli del paquete tokio?"` $\rightarrow$ INVALID: `Consulta irresoluble: La entidad de destino 'tokio' no existe en el grafo.`
3. **Calidad de Mensajes de Rechazo (Falsos Diagnósticos):**
   - Consulta: `"¿Se puede ir de Bogotá a Lima?"`
   - Respuesta del sistema: `Consulta incompleta: Consulta de ruta incompleta: falta especificar origen o destino.`
   - **Evaluación Crítica:** El mensaje es **técnicamente falso y confuso**. El usuario especificó claramente origen ("Bogotá") y destino ("Lima"). La causa real es que las entidades no existen en el grafo estático de ciudades, pero el parser de intenciones diagnostica erróneamente que "falta especificar origen o destino".
   - Lo mismo ocurre con `"¿Depende relic_cli de memmap2?"`: emite `falta especificar origen o destino` porque exige la frase literal `"del paquete"` o `"del crate"`.
4. **Fidelidad de la Traza (`enable_trace`):**
   - [LEÍDO: `relic_server/src/engine.rs:40-70`].
   - `normalizacion` e `intencion`: Registran tiempos reales de CPU (`0.03 ms - 0.06 ms`) y salida útil.
   - `rir`: **Sintético / Decorativo**. La etapa toma `0.0007 ms` porque no compila ningún AST ni genera RIR real; simplemente formatea el enum de intención con `format!("{:?}", intent)`.
   - `validacion`: **Decorativo**. Solo repite el valor del status final (`{"semantic_validity": status}`).
   - `ejecucion`: El campo `latency_ms` reporta el acumulado total desde `t0`, no el delta de la etapa.

---

## 6. AUDITORÍA UX/UI DEL FRONTEND - SIMPLE_DOMAIN (FASE 5)

### 6.1. Registro de Capturas Generadas
[MEDIDO: Todas capturadas mediante script CDP en `reports/audit/screenshots/`]:
- `01_estado_vacio_1440x900.png` y `01_estado_vacio_375x812.png`
- `02_curva_en_vivo_en_curso_1440x900.png` y `02_curva_en_vivo_en_curso_375x812.png`
- `03_curva_en_vivo_terminada_1440x900.png` y `03_curva_en_vivo_terminada_375x812.png`
- `04_tabla_comparativa_corridas_1440x900.png` y `04_tabla_comparativa_corridas_375x812.png`
- `05_visor_episodio_con_trace_1440x900.png` y `05_visor_episodio_con_trace_375x812.png`
- `06_burbuja_chat_insignias_1440x900.png` y `06_burbuja_chat_insignias_375x812.png`
- `07_estado_error_relic_apagado_1440x900.png` y `07_estado_error_relic_apagado_375x812.png`

### 6.2. Violaciones de Accesibilidad (WCAG) [MEDIDO: reports/audit/accessibility_report.json]
1. **Regiones Dinámicas sin `aria-live`:** [SEVERIDAD: MEDIA]. El flujo SSE de episodios y métricas dinámicas no posee atributo `aria-live="polite"`. Los lectores de pantalla no anuncian el progreso ni los episodios completados.
2. **Color como Único Indicador de Estado:** [SEVERIDAD: BAJA]. En el gráfico SVG y los badges de episodios, los estados EM y FA dependen exclusivamente del color (verde esmeralda vs rosa) sin diferenciador de icono ni texto en el nodo gráfico.
3. **Navegación por Teclado Incompleta:** [SEVERIDAD: ALTA]. Las filas de la tabla de corridas (`tr.cursor-pointer`) carecen de `tabindex="0"` y manejadores de evento `onKeyDown` (Enter/Espacio). Un usuario dependiente de teclado no puede seleccionar corridas históricas.
4. **Tipografía Infranqueable en Móvil:** [SEVERIDAD: MEDIA]. Se detectan clases `text-[9px]` y `text-[10px]` en telemetría y badges (ej. `ChatMessageBubble.tsx:126`, `SandboxLab.tsx:143`), violando el tamaño mínimo legible recomendado de 12px.

### 6.3. Análisis de Código del Sandbox Lab
- **Manejo de EventSource:** [LEÍDO: `SandboxLab.tsx:53-73`]. El listener SSE no tiene mecanismo de desconexión en el desmontaje (`useEffect cleanup`).
- **Resiliencia de Conexión:** [LEÍDO: `SandboxLab.tsx:73`]. En `es.onerror`, el código cierra la conexión y desactiva el spinner en silencio. No hay reintento con backoff ni advertencia visual de pérdida de enlace.
- **Rendimiento ante 1.000 Episodios:** [DERIVADO: 2.000 re-renders]. En una corrida de 1.000 episodios, se emiten 1.000 eventos `EPISODE_COMPLETED` y 1.000 eventos `RIR_PROPOSED`. Cada uno invoca un `setState` que recrea el array completo (`[...prev, item]`), provocando 2.000 renders sucesivos del componente padre, el gráfico SVG y el visor de episodios sin virtualización de DOM.

### 6.4. Terminología Visible sin Explicación
[LEÍDO: `SandboxLab.tsx:128-146`]:
- **`EM` / `Exact Match`:** Sin definición en pantalla (no explica si exige igualdad textual o semántica).
- **`FA Rate`:** Sin explicación de "Falsa Aceptación".
- **`Wilson 95%`:** Sin aclaración matemática sobre el intervalo de confianza binomial.
- **`λ`:** Sin texto explicativo de la penalización asimétrica.
- **`Eps`:** Abreviatura cruda de episodios.
- **`RIR`:** Sin desglose de "Relic Intermediate Representation".

### 6.5. Puntuación de Vistas (Escala 1 a 5)

| Vista | Claridad | Jerarquía Visual | Legibilidad Móvil | Accesibilidad | Manejo de Errores | Justificación |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| **Home / Chat (`HomeView`)** | 4/5 | 4/5 | 3/5 | 3/5 | 2/5 | Interfaz conversacional clara; telemetría detallada pero apretada en 375px; si el backend cae, la burbuja no aclara el motivo del corte. |
| **Sandbox Lab (`SandboxLab`)** | 3/5 | 4/5 | 2/5 | 2/5 | 2/5 | Excelente disposición de dashboard en desktop, pero abrumador en móvil (tabla requiere scroll forzado); siglas técnicas crudas; desconexión silenciosa sin alertas. |

---

## 7. AUDITORÍA DE AFIRMACIONES Y DEUDA TÉCNICA (FASE 6)

### 7.1. Tabla de Auditoría de Afirmaciones

| Afirmación en Documentación | Evidencia Disponible | Veredicto | Texto Propuesto de Reemplazo |
| :--- | :--- | :---: | :--- |
| *"Carga de pesos Safetensors vía memmap2 implementado"* (`README.md:47`) | Archivo `relic_10m_trained.safetensors` existe en disco (SHA256 `328d5e...`), pero el script `download_weights.py` espera `2f8485...` y no hay Releases públicas. [MEDIDO] | **PARCIAL** | *"Cargador Safetensors implementado y probado localmente; distribución de pesos oficiales pendiente de publicación en GitHub Releases."* |
| *"Front y back de demostración: Hecho, por conectar"* (`README.md:57`) | Backend FastAPI (`run_server.py`) y frontend Vite (`SandboxLab.tsx`) están construidos y conectados funcionalmente vía API REST y SSE. [MEDIDO] | **RESPALDADA** | *"Frontend y backend integrados: Simple Domain provee interfaz gráfica conversacional y Sandbox Lab conectado a RELIC."* |
| *"Evaluación sobre 75 casos de holdout"* (`docs/FASE_A.md:198`) | `reports/fase_a_closure_report.json` documenta los 75 casos per-caso con el veredicto negativo del modelo neuronal. [LEÍDO] | **RESPALDADA** | *(Mantener texto actual; está documentado con rigor).* |
| *"Sandbox evalúa el currículo de aprendizaje"* (`docs/SANDBOX_LAB.md:43`) | `sandbox_runner.py` solo carga los primeros $N$ casos secuenciales del split `train`; no evalúa holdout ni utiliza la semilla aleatoria. [LEÍDO] | **SIN RESPALDO** | *"El Sandbox Lab actual ejecuta un sondeo secuencial sobre el split de entrenamiento; la partición holdout y el muestreo aleatorio por semilla están pendientes de implementación."* |
| *"Vocabulario de 8.192 tokens"* (`README.md:94`) | `relic_tokenizer` define vocabulario en 8.192 en config, pero `reports/eval/loss_validacion.json:13` reporta `vocab_tokenizer: 4262` y `vocab_modelo: 8192`. [LEÍDO] | **PARCIAL** | *"Arquitectura dimensionada para 8.192 tokens; el tokenizador entrenado actual contiene 4.262 tokens con padding hasta 8.192."* |

### 7.2. Deuda Técnica Detectada
1. **Archivos que Incumplen el Límite de 180 Líneas:**
   - **RELIC:**
     - `tools/benchmarks/benchmark_semantic_interpreters.py` (718 líneas) [MEDIDO]
     - `tools/training/train_and_bench_semantic_compiler.py` (580 líneas) [MEDIDO]
     - `tools/data/generate_semantic_dataset.py` (567 líneas) [MEDIDO]
     - `tools/training/train_ablation_h2_toroidal.py` (489 líneas) [MEDIDO]
     - `tools/benchmarks/benchmark_llm_comparison.py` (475 líneas) [MEDIDO]
     - `crates/relic_core/tests/rir_conformance_tests.rs` (283 líneas) [MEDIDO]
   - **Simple_Domain:**
     - `backend/src/simple_domain/experiments/run_exp01.py` (454 líneas) [MEDIDO]
     - `backend/src/simple_domain/experiments/relational.py` (419 líneas) [MEDIDO]
     - `frontend/src/components/ChatMessageBubble.tsx` (218 líneas) [MEDIDO]
2. **Duplicación entre CLI y Servidor:**
   - `crates/relic_server` delega directamente en `relic_cli` para la normalización e intención. Sin embargo, en `Simple_Domain`, `relic_client.py` tiene un parser redundante que scrapea texto stdout de `relic_cli` mediante split de cadenas en vez de consumir una estructura de datos compartida o un endpoint unificado.
3. **Pendientes de Publicación:**
   - No existen GitHub Releases con binarios precompilados ni pesos safetensors descargables desde CDN.
   - El banco de pruebas humano externo (`external_human_bank`) figura como `PENDIENTE_DE_DATOS` en `reports/fase_a_closure_report.json:9`.

---

## 8. LO QUE NO SABEMOS (Incertidumbres No Verificadas)

1. **Hiperparámetros y Épocas Reales del Entrenamiento en GPU:** [NO VERIFICADO: El cuaderno `relic_colab_train_fase_a.ipynb` carece de celdas con salidas y no hay logs de W&B o TensorBoard en el repositorio]. No es posible verificar si el modelo divergió por learning rate excesivo, colapso de gradiente en SwiGLU o sub-ajuste severo.
2. **Origen Físico del Hash `2f8485...`:** [NO VERIFICADO: No existe archivo local con este hash]. Se desconoce si este checkpoint residió en un bucket temporal de GCS o si fue sobreescrito durante las pruebas.
3. **Rendimiento de Memoria bajo Carga Concurrente Real en Rust:** [NO VERIFICADO: El servidor se probó secuencialmente]. Aunque `relic_server` instancia 4 hilos de `tiny_http`, no se ha ejecutado una prueba de estrés de 1.000 peticiones simultáneas con Apache Bench o wrk.

---

## 9. TOP 10 ACCIONES MÁS RENTABLES (Priorizadas y Ordenadas)

*(Acciones estrictamente analíticas y de recomendación; NINGUNA ha sido implementada durante esta auditoría)*

1. **[CRÍTICO] Corregir la Carga de Datos en `SandboxRunner` (`load_cases`):** Filtrar estrictamente por `split == 'holdout_template'` (o permitir selector en UI) y utilizar el parámetro `seed` para un muestreo aleatorio determinista (`random.Random(seed).sample(...)`). *Esfuerzo estimado: 2 horas.*
2. **[CRÍTICO] Eliminar la Falsa Atribución de Procedencia en `relic_client.py`:** Si el usuario elige evaluar `model` o `slot_filler`, no recurrir silenciosamente al fallback de `relic_cli`. En su lugar, retornar un error explícito `503 UNAVAILABLE` indicando que el modelo neuronal no está en ejecución. *Esfuerzo estimado: 3 horas.*
3. **[ALTO] Optimizar la Persistencia de Eventos en Disco ($O(N^2) \rightarrow O(N)$):** Reemplazar la reescritura completa de `events.json` por un archivo append-only JSONL (`events.jsonl`) en `SandboxRepository.append_event()`. *Esfuerzo estimado: 2 horas.*
4. **[ALTO] Mitigar Path Traversal y Creación de Carpetas en `_run_dir`:** Sanitizar el parámetro `run_id` mediante expresión regular estricta (`r"^sb_[a-f0-9]{8}$"`) y no invocar `mkdir()` en operaciones de sólo lectura (`get_run`). *Esfuerzo estimado: 1 hora.*
5. **[ALTO] Unificar y Robustecer las Definiciones Métricas de Sandbox:** Incorporar el contador `syntax_rejection_count` en `SandboxRunner`, contabilizar las respuestas erróneas en casos resolubles como falsas aceptaciones (`fa_count += 1`), y evaluar conformidad contra `gold_rir`. *Esfuerzo estimado: 4 horas.*
6. **[MEDIO] Saneamiento de Mensajes de Rechazo en `relic_cli`:** Actualizar el evaluador semántico de grafos para que, cuando la entidad de origen o destino no exista en el grafo (ej. "Bogotá" o "Lima"), el mensaje indique claramente `Entidad 'Bogotá' desconocida en el catálogo` en lugar de `falta especificar origen o destino`. *Esfuerzo estimado: 3 horas.*
7. **[MEDIO] Agregar Limpieza y Resiliencia a `EventSource` en Frontend:** Implementar el cierre de conexión SSE en la función de limpieza de `useEffect` en `SandboxLab.tsx` y desplegar un banner de estado cuando la conexión se interrumpa. *Esfuerzo estimado: 2 horas.*
8. **[MEDIO] Virtualización de Lista y Loteo de Renders en `SandboxLab`:** Agrupar las actualizaciones de estado de episodios mediante buffers en lugar de ejecutar un re-render por cada evento individual, evitando 2.000 re-renders en corridas extensas. *Esfuerzo estimado: 4 horas.*
9. **[BAJO] Incorporar Glosario / Tooltips de Terminología Técnica en UI:** Añadir explicaciones emergentes sobre las siglas `EM`, `FA`, `Wilson 95%`, `λ` y `RIR` en los encabezados de la tabla y controles del formulario. *Esfuerzo estimado: 2 horas.*
10. **[BAJO] Sincronizar y Publicar Checkpoints Oficiales en GitHub Releases:** Subir el binario de pesos Safetensors definitivo a Releases y alinear el hash de `tools/download_weights.py` con el archivo verificado en disco. *Esfuerzo estimado: 1 hora.*

---
*Fin del Documento de Auditoría General.*
