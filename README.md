# RELIC
[![M8ven Score](https://m8ven.ai/badge/mcp/dev-nikolas-relic-bziaik?v=dbd37b1e39cc51eb203b176f16d0b8c2)](https://m8ven.ai/mcp/dev-nikolas-relic-bziaik?s=readme)
Motor de consultas determinista con validación transaccional, escrito en Rust. Funciona completamente en local sin depender de internet ni APIs externas.

RELIC no predice texto ni genera respuestas. Lo que hace es calcular de forma exacta y verificar antes de guardar cambios:
- Los cálculos aritméticos son matemáticamente exactos (sin aproximaciones).
- Las rutas en grafos se calculan con certificados de validez.
- Cada cambio se valida antes de confirmarse; si detecta inconsistencias, revierte automáticamente.
- Si no puede responder con certeza, se abstiene en lugar de inventar.

---

## ¿Para qué sirve?

1. **Cálculos Aritméticos con Traza Verificable:**
   - Resuelve porcentajes, descuentos y operaciones combinadas mostrando cada paso.
   - Evalúa expresiones con derivadas (árbol aritmético dual) para cálculos numéricos exactos.

2. **Consultas de Grafos con Restricciones:**
   - Responde si existe un camino entre dos puntos cumpliendo condiciones (ej: "¿puedo llegar de A a B evitando C?").
   - Devuelve la ruta exacta o demuestra formalmente que es imposible.

3. **Detección de Anomalías en Tiempo Real (RELIC:Seis Ojos):**
   - Monitorea métricas numéricas continuas (servidores, sensores IoT).
   - Detecta patrones anómalos basándose en análisis espectral y pruebas estadísticas.

4. **Almacén de Conocimiento con Validación:**
   - Guarda hechos y relaciones en un archivo seguro (`engram.jsonl`).
   - Antes de guardar un cambio, verifica que no rompa conocimientos previos (Blackwall).
   - Si algo falla, revierte al estado anterior exacto.

---

## Validación en Benchmarks

### Detección de Anomalías (NAB Benchmark)
Evaluado en el benchmark oficial **NAB (Numenta Anomaly Benchmark)** sobre 58 conjuntos de datos reales:

| Algoritmo / Detector | Standard Score | Low FP Score |
| :--- | :---: | :---: |
| **Detector Perfecto Teórico** | 100.0 | 100.0 |
| **Numenta HTM (Red Cortical)** | 70.1 | 63.1 |
| **knncad (K-Vecinos)** | 58.0 | 43.4 |
| **RELIC:Seis Ojos (Rust)** | **57.7** | **53.2** |
| **Relative Entropy (HP Labs)** | 54.6 | 47.6 |
| **Random Cut Forest (Amazon AWS)** | 51.7 | 38.4 |
| **Twitter ADVec (Twitter)** | 47.1 | 33.6 |
| **Windowed Gaussian (Media/Varianza)**| 39.6 | 20.9 |

RELIC:Seis Ojos alcanza un puntaje comparable a algoritmos de la industria (57.7), con latencia de 45.94 µs/punto y huella de memoria de 4.99 MB.

---

## Estructura del Proyecto

- **`relic_core`**: Motor central. Cálculos exactos, búsqueda en grafos, detector de anomalías.
- **`relic_lexicon`**: Almacén de conocimiento con historial versionado.
- **`relic_blackwall`**: Compuerta de seguridad. Valida cambios y revierte si hay errores.
- **`relic_cli`**: Interfaz de línea de comandos.
- **`relic_server`**: Servidor HTTP local.
- **`relic_mcp`**: Conector para Model Context Protocol (MCP).
- **`relic_tokenizer`**: Tokenizador de texto sin dependencias externas.

---

## Cómo Probarlo

### Requisitos
Rust estable (instalar desde [rustup.rs](https://rustup.rs)).

### Ejecutar pruebas
```bash
cargo test --workspace
```

### Consultas desde la terminal
```bash
# Aritmética
cargo run -p relic_cli -- query "¿cuánto es el 15% de 240?"
# -> 15 / 100 = 0.15; 0.15 * 240 = 36. El resultado es 36.

# Rutas en grafos
cargo run -p relic_cli -- query "¿Puede Marta llegar a la biblioteca pasando por la estación?"
# -> Sí, es posible alcanzar biblioteca desde Marta pasando por estación (Ruta: Marta -> estación -> biblioteca, 2 saltos).
```

### Servidor MCP o HTTP
```bash
# MCP (para usar como herramienta en Claude/Cursor)
cargo run -p relic_mcp

# Servidor HTTP
cargo run -p relic_server
```

---

## Alcance y Limitaciones

**Lo que hace:**
- Cálculos aritméticos exactos con trazas verificables.
- Consultas de grafos con certificados de validez.
- Detección de anomalías en series temporales numéricas.
- Gestión de conocimiento con validación transaccional.

**Lo que no hace:**
- No es un chatbot de conversación general.
- No genera texto ni tiene creatividad lingüística.
- Si la consulta no coincide con sus patrones conocidos, responde que no sabe.

---

## Licencia

GNU Affero General Public License v3.0 (AGPL-3.0). Ver [`LICENSE`](LICENSE) y [`NOTICE`](NOTICE).
