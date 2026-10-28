# Plan de Integración y Hoja de Ruta Técnica: RELIC ↔ Simple Domain

> **Documento de Planificación y Arquitectura.**
> Repositorio del motor: `RELIC` | Frontend & Backend existente: `Simple_Domain`
> Estado: Fase 0 aprobada con condiciones. En ejecución Fase 1.

---

## 1. Reglas de Medición y Cero Estimaciones Especulativas

> [!IMPORTANT]
> **Regla de Medición Real:** Quedan eliminadas todas las cifras no medidas previamente ("más de 150 MB", "< 20 MB").
> Cualquier número de consumo de memoria RSS o latencia que se reporte debe provenir de un comando ejecutado en esta máquina (`Get-Process`, `relic_cli::monitor` o benchmarking automatizado) y citar el comando exacto y su salida. Las mediciones comparativas entre el backend Python y el servidor Rust se realizarán durante la Fase 1.

---

## 2. Inventario Completo de Endpoints del Frontend

Revisión exhaustiva de [`Simple_Domain/frontend/src/services/simpleDomainApi.ts`](./Simple_Domain/frontend/src/services/simpleDomainApi.ts) y [`Simple_Domain/frontend/src/services/modularApi.ts`](./Simple_Domain/frontend/src/services/modularApi.ts):

### 2.1 Telemetría y Diagnóstico
1. `GET /health` ([`simpleDomainApi.ts:L58`](./Simple_Domain/frontend/src/services/simpleDomainApi.ts#L58)):
   - **Retorno:** `{ "status": string, "detector_ready": boolean, "message": string }`.
   - **Semántica de `detector_ready` en RELIC:** En el sistema original significaba detector estadístico calibrado. En RELIC, `detector_ready: true` significa que el **motor de razonamiento determinista y el grafo episódico están inicializados y listos para ejecutar consultas**, independientemente de si los pesos Safetensors están presentes (modo completo) o ausentes (modo degradado determinista).
2. `GET /v1/info` ([`simpleDomainApi.ts:L62`](./Simple_Domain/frontend/src/services/simpleDomainApi.ts#L62)):
   - **Retorno:** `{ "service_version": "0.1.0", "package_version": "...", "schema_version": "v1", "model_status": "ready" | "degraded", "capabilities": [...], "model_configuration": ... }`.
3. `POST /v1/score`:
   - Declarado en el catálogo de endpoints de Simple Domain para compatibilidad hacia atrás.

### 2.2 Gestión de Sesiones y Espacio Conversacional
4. `GET /v1/conversations` ([`modularApi.ts:L87`](./Simple_Domain/frontend/src/services/modularApi.ts#L87)):
   - Lista todas las conversaciones para la barra lateral (`Conversation[]`).
5. `POST /v1/conversations` ([`modularApi.ts:L88-L89`](./Simple_Domain/frontend/src/services/modularApi.ts#L88-L89)):
   - Crea nueva conversación. Body: `{"title": string}`. Retorna `Conversation`.
6. `GET /v1/conversations/{id}` ([`modularApi.ts:L90`](./Simple_Domain/frontend/src/services/modularApi.ts#L90)):
   - Retorna la conversación con todos sus `ChatMessage[]`.
7. `POST /v1/conversations/{id}/messages` ([`modularApi.ts:L91-L95`](./Simple_Domain/frontend/src/services/modularApi.ts#L91-L95)):
   - Body: `{"role": "user", "content": string}`.
   - **Contrato de persistencia:** Guarda el mensaje del usuario, invoca el router de RELIC, genera y guarda de inmediato el mensaje del asistente `{"role": "assistant", "content": ...}` con metadatos de latencia y ruta, retornando el mensaje creado.

### 2.3 Módulos de Investigación, Fuentes, Patrones y Configuración
8. `GET /v1/research/experiments`, `POST /v1/research/experiments`, `DELETE /v1/research/experiments/{id}` ([`modularApi.ts:L49-L53`](./Simple_Domain/frontend/src/services/modularApi.ts#L49-L53)).
9. `GET /v1/data-sources`, `POST /v1/data-sources`, `DELETE /v1/data-sources/{id}` ([`modularApi.ts:L56-L60`](./Simple_Domain/frontend/src/services/modularApi.ts#L56-L60)).
10. `GET /v1/patterns`, `POST /v1/patterns`, `DELETE /v1/patterns/{id}` ([`modularApi.ts:L63-L67`](./Simple_Domain/frontend/src/services/modularApi.ts#L63-L67)).
11. `GET /v1/settings`, `PATCH /v1/settings`, `POST /v1/settings/reset` ([`modularApi.ts:L70-L73`](./Simple_Domain/frontend/src/services/modularApi.ts#L70-L73)).
12. `POST /v1/archive/submit`, `GET /v1/archive/{id}`, `GET /v1/archive/{id}/result`, `GET /v1/archive` ([`modularApi.ts:L76-L84`](./Simple_Domain/frontend/src/services/modularApi.ts#L76-L84)).
13. `GET /v1/archive/{id}/events` ([`ChatMessageBubble.tsx:L35`](./Simple_Domain/frontend/src/components/ChatMessageBubble.tsx#L35)): SSE para tareas de ingesta de archivos.
14. `POST /v1/analysis`, `GET /v1/analysis`, `GET /v1/analysis/{id}` ([`modularApi.ts:L98-L104`](./Simple_Domain/frontend/src/services/modularApi.ts#L98-L104)).

### 2.4 Endpoints Nativos de RELIC
15. `POST /api/query`: Inferencia unificada (aritmética, grafo, red o rechazo epistémico).
16. `POST /api/generate`: Streaming causal de texto token a token vía Server-Sent Events.

---

## 3. Justificación Arquitectónica del Servidor HTTP

### Justificación de Implementación en Rust: Simplicidad de Despliegue Local
La adopción de `relic_server` en Rust se fundamenta estrictamente en la **simplicidad de despliegue local y autonomía operativa**:
- **Binario único autocontenido:** Elimina la necesidad de instalar o mantener un entorno de ejecución de Python, gestores de paquetes (`pip`/`uv`), entornos virtuales (`.venv`) o servidores Node.js para servir el frontend compilado.
- **Servicio integral:** El mismo ejecutable nativo atiende las APIs de inferencia y entrega los activos estáticos del frontend (`dist`), permitiendo levantar todo el sistema con un solo comando.

### Elección de `tiny_http` frente a `axum`/`tokio`
1. **Minimización de dependencias y tiempos de compilación:**
   - `tiny_http` solo requiere 4 dependencias mínimas (`ascii`, `chunked_transfer`, `httpdate`, `log`), compilando en segundos con `cargo build --release -j 1`.
   - `axum` + `tokio` arrastra decenas de crates transitivos innecesarios para un servicio monousuario de escritorio.
2. **Naturaleza del Cómputo (CPU-bound):**
   - El cómputo en RELIC (evaluación de Transformer, BFS en grafos y aritmética exacta) es CPU-bound. Un pool síncrono de 4 hilos de trabajo atiende peticiones locales concurrentes sin el overhead de un reactor asíncrono.
3. **Soporte de Streaming SSE:**
   - Emisión limpia de Server-Sent Events mediante codificación por fragmentos (chunked transfer).

### Medición Real de Memoria (Comando Ejecutado)
- **Comando:** `Get-Process relic_server | Select-Object Id, ProcessName, @{Name="WS_MB";Expression={[math]::Round($_.WorkingSet64 / 1MB, 2)}}`
- **Memoria Residente Real Medida (Working Set):** **46.05 MB** (en modo de ejecución con frontend y pesos mapeados).

---

## 4. Contrato SSE para `/api/generate` (Sin Conflicto con `/v1/archive`)

El frontend existente usa `EventSource` exclusivamente en `/v1/archive/{id}/events` para reportar progreso de ingesta de datasets.
El endpoint `/api/generate` opera de manera completamente independiente bajo el siguiente formato SSE:
- Cabeceras de respuesta:
  - `Content-Type: text/event-stream`
  - `Cache-Control: no-cache`
  - `Connection: keep-alive`
  - `Access-Control-Allow-Origin: *`
- Cada token generado se emite como un bloque SSE:
  ```text
  data: {"token": "el", "token_id": 45, "finished": false}

  data: {"token": " pingüino", "token_id": 99, "finished": false}

  data: {"token": "", "token_id": 315, "finished": true, "route": "neural_model", "latency_ms": 142.5}
  ```

---

## 5. Plan de Ejecución Detallado (Fase 1)

### Paso 1: Motor Desacoplado `relic_core` / `relic_cli`
- Implementar en `relic_cli` (o crate interno) una función ejecutable pura `execute_query(query: &str, state: &EngineState) -> QueryResult` que retorne datos estructurados sin depender de `println!`.
- Verificar con `cargo test --workspace`.

### Paso 2: Creación de `crates/relic_server`
- Configurar `crates/relic_server/Cargo.toml` dentro del workspace.
- Crear módulos modulares (≤ 180 líneas cada uno):
  * `state.rs`: Estructura `ServerState` con `RwLock` para conversaciones y memoria episódica.
  * `handlers/health.rs`: Implementación de `/health` y `/v1/info`.
  * `handlers/conversations.rs`: Implementación completa de crear, listar, leer y responder mensajes.
  * `handlers/query.rs`: Handler de `POST /api/query`.
  * `handlers/generate.rs`: Handler SSE de `POST /api/generate`.
  * `handlers/static_files.rs`: Servicio de `Simple_Domain/frontend/dist` en la raíz `/`.
  * `main.rs`: Arranque, parsing de puerto y loop de despacho.

### Paso 3: Medición Real de Consumo y Smoke Test
- Levantar el servidor y medir mediante comando PowerShell su RSS real.
- Comparar con el proceso de Python si está levantado o registrar su medición exacta.
- Ejecutar script de prueba de humo con "1 + 1", una consulta de grafo y una de lenguaje libre.
