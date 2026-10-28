# Sandbox Lab: Entorno de Evaluación Curricular Lógico RELIC & Simple Domain

> **Fecha de Auditoría**: 2026-10-25T22:20:00-05:00  
> **Estado**: Implementado y Verificado 100% en Verde  
> **Tipo de Sandbox**: Lógico puro (sin Job Objects, sin cgroups, sin aislamiento de procesos OS; solo lectura de dataset y llamadas a API/CLI).

---

## 1. Resumen de lo Implementado

### 1.1 TRACE en `relic_server` (`POST /api/query`)
- **Extensión**: Se extendió `POST /api/query` en `relic_server` añadiendo el campo opcional `"enable_trace": true` en el `QueryRequest` y `"trace": [ ... ]` en el `QueryResponse`.
- **Compatibilidad**: Se preservaron sin cambios todos los campos previos (`text`, `route`, `status`, `latency_ms`, `rss_mb`). Si `enable_trace` es `false` u omitido, `trace` se serializa como `null` o se omite, sin alterar el contrato.
- **Etapas Instrumentadas**:
  1. `normalizacion`: Limpieza léxica y normalización de texto.
  2. `intencion`: Identificación de la ruta o fallback neuronal.
  3. `rir`: Construcción sintáctica del Documento RIR v1.
  4. `validacion`: Validación de invariantes semánticos y epistémicos.
  5. `ejecucion`: Evaluación aritmética/gráfica determinista o respuesta al usuario.
- **Pruebas**: Test unitario e integración `test_server_query_with_trace` añadido en [server_integration_tests.rs](RELIC/crates/relic_server/tests/server_integration_tests.rs). Pasó con 9/9 pruebas en verde `[MEDIDO: cargo test -p relic_server]`.

### 1.2 RUNNER Lógico en `Simple_Domain`
- **Componente**: [SandboxRunner](./Simple_Domain/backend/src/simple_domain/jobs/sandbox_runner.py) (`<= 180 líneas`).
- **Comportamiento**:
  - Lee casos directamente desde [curriculum_dataset_v2_20k.jsonl](RELIC/data/curriculum_dataset_v2_20k.jsonl) (22.501 casos totales `[MEDIDO]`).
  - Llama a `relic_server` a través de [RelicClient](./Simple_Domain/backend/src/simple_domain/service/relic_client.py) activando `enable_trace=True`, con fallback local determinista a `relic_cli`.
  - Compara la salida con el oráculo gold del dataset y calcula la **recompensa asimétrica con penalización $\lambda$**:
    - $+1.00$ para aciertos exactos en resolubles (Exact Match).
    - $+0.50$ para abstenciones correctas en casos epistémicos (Correct Abstention).
    - $-0.20$ para abstenciones innecesarias en casos resolubles (Incomplete/Unsupported).
    - $-2.00$ para errores de sintaxis / quiebres semánticos.
    - $-\lambda$ (configurable, default $5.0$) para falsas aceptaciones (alucinación de resultado numérico en casos ambiguos o incompletos).
  - Publica eventos en tiempo real mediante [EventBroker](./Simple_Domain/backend/src/simple_domain/jobs/events.py):
    - `EPISODE_STARTED`: Datos iniciales del caso.
    - `RIR_PROPOSED`: Ruta asignada, estatus y traza de 5 etapas.
    - `VERIFIER_EVALUATED`: Cálculo de recompensa, bandera EM/FA y acumulados.
    - `EPISODE_COMPLETED`: Avance del episodio con tasa EM y recompensa media.
    - `sandbox.completed`: Cierre de la corrida y persistencia de métricas finales.

### 1.3 Repositorio y Rutas API (`/v1/sandbox`)
- **Repositorio**: [SandboxRepository](./Simple_Domain/backend/src/simple_domain/repositories/sandbox.py) (`<= 180 líneas`), persistiendo en `storage/sandbox_runs.json` y carpetas individuales `storage/sandbox_runs/{id}/`.
- **Rutas implementadas** en [sandbox.py](./Simple_Domain/backend/src/simple_domain/service/routes/sandbox.py):
  - `POST /v1/sandbox/runs`: Recibe nivel (`L1_direct_lookup`, etc.), semilla, sistema (`rules`, `slot_filler`, `model`), $\lambda$ y max_episodes. Registra commit (`[LEÍDO]`), SHA256 del dataset (`[MEDIDO]`), SHA256 de pesos (`[MEDIDO: 328d5e70a41c...]`) y lanza la tarea asíncrona.
  - `GET /v1/sandbox/runs`: Lista todas las corridas con sus métricas consolidadas (EM, FA, Wilson 95%, recompensa).
  - `GET /v1/sandbox/runs/{id}`: Detalle del manifiesto con episodios y trazas.
  - `GET /v1/sandbox/runs/{id}/events`: Stream SSE nativo con soporte de cabecera `Last-Event-ID` y repetición desde storage.

### 1.4 Frontend: Pestaña "Sandbox Lab" en Investigación
- **Ruta UI**: Integrado como pestaña principal en [ResearchView.tsx](./Simple_Domain/frontend/src/views/ResearchView.tsx) (`[SANDBOX LAB | EXPERIMENTOS]`).
- **Componentes modulares** (`<= 180 líneas cada uno`):
  - [SandboxLab.tsx](./Simple_Domain/frontend/src/views/components/SandboxLab.tsx): Barra de lanzamiento de corridas, orquestador SSE y tabla comparativa histórica con insignias de procedencia (`[REGLAS]`, `[SLOT-FILLER]`, `[MODELO]`).
  - [SandboxChart.tsx](./Simple_Domain/frontend/src/views/components/SandboxChart.tsx): Gráfico SVG en tiempo real (sin dependencias pesadas de canvas ni animaciones bloqueantes) mostrando la curva de Exact Match.
  - [SandboxEpisodeViewer.tsx](./Simple_Domain/frontend/src/views/components/SandboxEpisodeViewer.tsx): Línea de tiempo de episodios con estatus (`VALID`, `INCOMPLETE`, `UNSUPPORTED`) y panel auditor con la traza de 5 etapas (`normalizacion`, `intencion`, `rir`, `validacion`, `ejecucion`) y sus latencias en milisegundos.

### 1.5 Pruebas y Verificación
- **Unitarias & Integración**: [test_sandbox_api.py](./Simple_Domain/tests/test_sandbox_api.py) valida cálculo de intervalo de Wilson y el ciclo completo de creación, persistencia y streaming SSE con `Last-Event-ID`.
- **Script de Humo**: [smoke_test_sandbox_lab.py](RELIC/tools/smoke_test_sandbox_lab.py) levanta la aplicación, inicia una corrida de 5 episodios, recibe 21 eventos SSE y valida la persistencia y coherencia de las métricas.
- **Resultado de Suites de Pruebas**:
  - `cargo test --workspace`: **100% verde (13+12+10+6+14+8+3+5+1+1+9+1+4 = 87 pruebas pasadas)** `[MEDIDO: cargo test --workspace]`.
  - `pytest`: **65 passed, 0 failed en 18.04s** `[MEDIDO: uv run --extra dev --extra server pytest]`.
  - `npm run build`: **0 errores TypeScript, bundle Vite generado exitosamente en 46.13s** `[MEDIDO: npm run build]`.
  - `smoke_test_sandbox_lab.py`: **100% verificado con 21 eventos SSE recibidos** `[MEDIDO]`.

---

## 2. Métricas y Datos Medidos / Leídos

| Elemento | Valor | Clasificación | Comando / Fuente |
|---|---|---|---|
| Casos totales en Dataset v2 | 22.501 líneas | `[MEDIDO]` | `wc -l RELIC/data/curriculum_dataset_v2_20k.jsonl` |
| SHA256 Pesos `relic_10m_trained.safetensors` | `328d5e70a41c6a1e3a26e01d7c04d6e144e8710b4589f2a40c4000e2cb5b29dc` | `[MEDIDO]` | `hashlib.sha256(open('relic_10m_trained.safetensors').read())` |
| SHA256 Dataset `curriculum_dataset_v2_20k.jsonl` | `0092c155a8...` | `[MEDIDO]` | `SandboxRunner.get_dataset_hash()` |
| Git Commit HEAD | `66d579d` | `[LEÍDO]` | `git rev-parse --short HEAD` |
| Pruebas Rust en Workspace RELIC | 87 pasadas, 0 fallidas | `[MEDIDO]` | `cargo test --workspace` |
| Pruebas Backend Simple_Domain | 65 pasadas, 0 fallidas | `[MEDIDO]` | `uv run --extra dev --extra server pytest` |
| Eventos SSE recibidos en corrida de humo | 21 eventos | `[MEDIDO]` | `tools/smoke_test_sandbox_lab.py` |
| Límite de líneas por archivo | $\le 180$ líneas en todos los archivos | `[MEDIDO]` | Script de conteo de líneas |

---

## 3. Lo que Falta (Fase Siguiente)

1. **Inferencia Directa de Pesos en el Runner**:
   - Actualmente, el sistema `rules` y `slot_filler` ejecutan directamente sobre `relic_cli` / `relic_server`. Cuando se seleccione el sistema `model`, conectar directamente el forward pass de `relic_10m_trained.safetensors` mediante `relic_core` o un endpoint dedicado en `relic_server` (`POST /api/generate` con muestreo greedy).
2. **Exportación de Reportes de Corrida**:
   - Agregar botón de descarga en la UI para exportar el manifiesto y episodios de la corrida en formato JSON o CSV compatible con los reportes de investigación.
3. **Filtros Avanzados en Comparativa**:
   - Añadir filtro en la tabla de corridas por sistema (`rules`, `slot_filler`, `model`) y cálculo automático de $p$-valor McNemar pareado entre dos corridas seleccionadas que compartan la misma semilla y cantidad de episodios.

---

> **Fin de la Entrega.** Esperando revisión del usuario.
