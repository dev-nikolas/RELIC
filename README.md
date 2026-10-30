# RELIC

Motor de razonamiento determinista y seguro en Rust, pensado para funcionar en local (sin internet y sin depender de APIs de terceros).

A diferencia de los modelos de lenguaje tradicionales (LLMs) que predicen texto por aproximación estadística y pueden alucinar, **RELIC separa entender de calcular**:
- Si le pides un cálculo o una ruta en un grafo, **lo calcula de forma matemática exacta**.
- Si no está seguro de una respuesta, **se abstiene en lugar de inventar**.
- Si aprende un nuevo dato, **comprueba que no rompa lo que ya sabía** antes de guardarlo.

---

## ¿Para qué sirve?

1. **Cálculos y Aritmética Cotidiana con Explicación Paso a Paso:**
   - Resuelve porcentajes, descuentos, proporciones y operaciones combinadas mostrando la traza exacta de cómo llegó al resultado (sin errores de redondeo ni alucinaciones numéricas).
2. **Consultas de Rutas y Relaciones en Grafos:**
   - Responde si es posible conectar dos puntos cumpliendo restricciones (ej: *"¿puedo llegar de A a B pasando por C pero evitando D?"*), devolviendo el camino exacto o demostrando si es imposible.
3. **Gestión de Memoria y Conocimiento sin Olvido Catastrófico:**
   - Guarda hechos y relaciones en un archivo seguro (`engram.jsonl`). Si un dato cambia (ej: *"el precio subió"* o *"el servidor cambió de IP"*), actualiza la información sin dejar datos obsoletos huérfanos.
4. **Vigilancia de Sensores y Telemetría en Tiempo Real (Seis Ojos):**
   - Monitorea lecturas numéricas continuas (como métricas de servidores o sensores IoT). Aprende el comportamiento normal y detecta fallas o anomalías al instante, aislando nodos defectuosos en memoria antes de que afecten al resto del sistema.
5. **Comprobación de Seguridad antes de Guardar (Blackwall):**
   - Antes de confirmar cualquier cambio, comprueba que no cree contradicciones, ciclos infinitos ni rompa pruebas anteriores. Si algo falla, revierte todo exactamente al estado anterior.

---

## Estructura del Proyecto

El sistema está dividido en componentes modulares e independientes:

- **`relic_core`**: El motor central. Se encarga de los cálculos exactos, la búsqueda de caminos en grafos y el detector de anomalías Seis Ojos.
- **`relic_lexicon`**: El almacén de conocimiento. Guarda el historial de hechos de forma segura y permite viajar en el tiempo entre versiones.
- **`relic_blackwall`**: La compuerta de seguridad. Valida cada cambio antes de guardarlo y revierte automáticamente si detecta regresiones o datos sospechosos.
- **`relic_cli`**: Interfaz de línea de comandos para hacerle consultas directas a RELIC desde la terminal.
- **`relic_server`**: Servidor local ligero (HTTP) para conectar interfaces web u otras aplicaciones.
- **`relic_mcp`**: Conector estándar para Model Context Protocol (MCP), permitiendo que asistentes como Claude o Cursor usen RELIC como herramienta determinista.
- **`relic_tokenizer`**: Tokenizador de texto propio sin dependencias externas.

---

## Cómo Probarlo

### Requisitos
Necesitas tener instalado Rust (estable reciente). Si no lo tienes, instálalo desde [rustup.rs](https://rustup.rs).

### 1. Ejecutar las pruebas
Para verificar que todo el motor funciona correctamente:
```bash
cargo test --workspace
```

### 2. Consultas desde la terminal (`relic_cli`)

```bash
# 1. Aritmética cotidiana con explicación
cargo run -p relic_cli -- query "¿cuánto es el 15% de 240?"
# -> 15 / 100 = 0.15; 0.15 * 240 = 36. El resultado es 36.

cargo run -p relic_cli -- query "calcula el doble de (25 + 15)"
# -> 25 + 15 = 40; 40 * 2 = 80. El resultado es 80.

# 2. Rutas en grafos con restricciones
cargo run -p relic_cli -- query "¿Puede Marta llegar a la biblioteca pasando por la estación?"
# -> Sí, es posible alcanzar biblioteca desde Marta pasando por estación (Ruta: Marta -> estación -> biblioteca, 2 saltos).
```

### 3. Usar como Servidor MCP o HTTP
- Para conectarlo como herramienta a un agente o editor (MCP sobre stdio):
  ```bash
  cargo run -p relic_mcp
  ```
- Para levantar el servidor HTTP local:
  ```bash
  cargo run -p relic_server
  ```

---

## Lo que hace y lo que no hace

- **Lo que sí hace:** Si una consulta coincide con los patrones que conoce (aritmética, grafos, relaciones, telemetría), el resultado es **100% exacto, calculado y verificable**.
- **Lo que no hace:** No es un chatbot de conversación general libre. Si le haces una pregunta abierta fuera de su conocimiento o sin estructura, el sistema **prefiere responder que no sabe antes de inventar una respuesta**.

---

## Licencia

Distribuido bajo la licencia **GNU Affero General Public License v3.0 (AGPL-3.0)**. Consulta el archivo [`LICENSE`](LICENSE) para más detalles.
