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
4. **Vigilancia de Sensores y Telemetría en Tiempo Real (RELIC:Seis Ojos):**
   - Monitorea lecturas numéricas continuas (como métricas de servidores o sensores IoT). Aprende el comportamiento normal y detecta fallas o anomalías al instante, aislando nodos defectuosos en memoria antes de que afecten al resto del sistema.
5. **Comprobación de Seguridad antes de Guardar (Blackwall):**
   - Antes de confirmar cualquier cambio, comprueba que no cree contradicciones, ciclos infinitos ni rompa pruebas anteriores. Si algo falla, revierte todo exactamente al estado anterior.

---

## Comparativa con Métodos Estándar

¿Cómo sabemos si el sistema es bueno? Contrastado directamente contra los estándares de la industria:

### 1. Detección de Anomalías (RELIC:Seis Ojos vs. Tablero Oficial NAB)
En el benchmark oficial **NAB (Numenta Anomaly Benchmark)** sobre 58 conjuntos de datos reales de servidores, tráfico y métricas en tiempo real:

| Algoritmo / Detector | Standard Score | Low FP Score (Castigo a Falsas Alarmas) | Latencia y Memoria |
| :--- | :---: | :---: | :--- |
| **Detector Perfecto Teórico** | 100.0 | 100.0 | Límite matemático superior |
| **Numenta HTM (Red Cortical)** | 70.1 | 63.1 | Pesado en CPU/RAM |
| **knncad (K-Vecinos)** | 58.0 | 43.4 | Requiere almacenar buffer masivo |
| **RELIC:Seis Ojos (Rust)** | **57.7** | **53.2** | **25 µs/punto, 278 KB RAM, 0 allocs** |
| **Relative Entropy (HP Labs)** | 54.6 | 47.6 | Métricas de entropía |
| **Random Cut Forest (Amazon AWS)** | 51.7 | 38.4 | Árboles de aislamiento |
| **Twitter ADVec (Twitter)** | 47.1 | 33.6 | Descomposición clásica |
| **Windowed Gaussian (Media/Varianza)**| 39.6 | 20.9 | Baseline simple |
| **Detector Cero / Nulo** | 0.0 | 0.0 | No emite alertas |

*RELIC:Seis Ojos supera los detectores oficiales de AWS (51.7) y Twitter (47.1), manteniendo una huella mínima de solo 278 KB por canal.*

### 2. Razonamiento y Grafos (RELIC vs. LLMs Tradicionales)

| Dimensión | LLMs Típicos (7B/13B o APIs en la nube) | RELIC (Núcleo Determinista) |
| :--- | :--- | :--- |
| **Aritmética y Números** | Propenso a errores en cifras largas y redondeos. | **100% exacto** (árbol aritmético evaluado en Rust). |
| **Rutas en Grafos** | Puede alucinar atajos o conexiones inexistentes. | **Certificado de corte**: devuelve la ruta o demuestra imposibilidad. |
| **Actualización de Memoria** | Olvido catastrófico al reentrenar o actualizar. | **Consistencia causal del 100%** mediante grafo de reemplazo. |
| **Latencia por Consulta** | 300 ms – 2,500 ms (requiere GPU). | **< 35 microsegundos** en CPU local (> 1,000x más rápido). |
| **Consumo de Memoria** | 4 GB – 16 GB VRAM. | **4.15 MB de RAM base**. |

---

## Estructura del Proyecto

El sistema está dividido en componentes modulares e independientes:

- **`relic_core`**: El motor central. Se encarga de los cálculos exactos, la búsqueda de caminos en grafos y el detector de anomalías RELIC:Seis Ojos.
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

## Documentación Técnica

Para profundizar en la arquitectura interna y auditorías del proyecto, consulta la carpeta [`docs/`](docs/README.md):
- [Arquitectura Técnica de RELIC](docs/ARCHITECTURE.md)
- [Evaluación Empírica de RELIC:Seis Ojos](INFORME_INTEGRACION_ESTRICTA_SEIS_OJOS_RELIC.md)
- [Guía Rápida de RELIC:Seis Ojos](GUIA_SIMPLE_SEIS_OJOS_RELIC.md)
- [Especificación del Servidor MCP](docs/DISENO_SANDBOX_MCP.md)

---

## Lo que hace y lo que no hace

- **Lo que sí hace:** Si una consulta coincide con los patrones que conoce (aritmética, grafos, relaciones, telemetría), el resultado es **100% exacto, calculado y verificable**.
- **Lo que no hace:** No es un chatbot de conversación general libre. Si le haces una pregunta abierta fuera de su conocimiento o sin estructura, el sistema **prefiere responder que no sabe antes de inventar una respuesta**.

---

## Licencia

Distribuido bajo la licencia **GNU Affero General Public License v3.0 (AGPL-3.0)** pura. Consulta el archivo [`LICENSE`](LICENSE) para el texto completo de la licencia y el archivo [`NOTICE`](NOTICE) para la atribución de autoría.
