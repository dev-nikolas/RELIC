# RELIC — Arquitectura Cognitiva de Software y Programa de Investigación

> **Definición de un Relic Digital:** Unidad de software cognitiva portable, local, autónoma y persistente. Diseñada para procesar lenguaje humano, mantener representaciones estructuradas de problemas, razonar con procedimientos verificables y seleccionar acciones bajo incertidumbre, con una huella de memoria física mínima (< 50 MB en inferencia nativa en Rust).

---

## 1. Fundamento Filosófico y Cognitivo

La mayoría de los sistemas contemporáneos confunden **predecir la siguiente palabra** con **pensar**. Asumen que una red neuronal de miles de millones de parámetros que emite texto plausible debe ser el único sustrato de toda la inteligencia.

RELIC parte de un principio alternativo derivado de la ciencia cognitiva y la neurobiología:

1. **El lenguaje no es el sustrato único del pensamiento:**  
   El lenguaje humano funciona como una interfaz de altísima expresividad para organizar, refinar y comunicar conceptos a otras mentes. Mucho procesamiento cognitivo esencial (espacial, motor, relacional y deductivo) opera sobre representaciones no verbales, geometrías y dinámicas de estados.
2. **Separación de Niveles de Representación:**
   - **Nivel 1 — RELIC Intermediate Representation (RIR):**  
     Una representación formal, explícita, fuertemente tipada y verificable. Permite serializar problemas como grafos de estado, objetivos, restricciones y planes, garantizando ejecución matemática exacta, cero alucinaciones y transferencia de procedimientos entre dominios dispares.
   - **Nivel 2 — Representación Cognitiva Aprendida (Línea Experimental H1):**  
     Un espacio latente continuo y recurrente de estado compacto $O(1)$ para descubrir regularidades, absorber variaciones de entrada, mantener la memoria de trabajo secuencial y generar hipótesis ante problemas que no encajen en plantillas conocidas.

---

## 2. Arquitectura de Cuatro Subsistemas Auditados

La arquitectura de RELIC desacopla deliberadamente las responsabilidades del sistema para que cada fallo sea localizable y no se atribuya a una misteriosa "caja negra":

```mermaid
graph TD
    User([Entrada Humana en Español / Código]) --> SubA[A. Sistema Lingüístico<br/>Parser Sintáctico / H1]
    SubA -->|Compila a RIR canónico| SubD[D. Sistema de Evaluación y Selección<br/>a* = argmin J(s, a)]
    SubD -->|Consulta entidades y hechos| SubB[B. Sistema de Representación y Memoria<br/>EpisodicMemory indexada]
    SubB -->|Hechos conocidos / Estado de certidumbre| SubD
    SubD -->|Despacha tarea validada| SubC[C. Sistema de Razonamiento<br/>ReasoningEngine BFS / Reglas]
    SubC -->|Plan y solución certificada| SubD
    SubD -->|Resultado verificado| SubA
    SubA --> Output([Respuesta Fiel en Lenguaje Natural])
```

### A. Sistema Lingüístico
- **Entrada:** Lenguaje natural libre en español, preguntas modales, órdenes imperativas o código.
- **Función:** Identificar la intención, extraer argumentos, detectar polaridad (afirmación / negación simple o doble) y emitir la especificación formal RIR.
- **Estado Actual:** Parser sintáctico explícito validado con 100% de exactitud estricta en casos adversariales y latencia sub-milisegundo.

### B. Sistema de Representación y Memoria Episódica
- **Estructura:** Memoria indexada en tablas hash y conjuntos (`HashMap`, `HashSet`) en Rust puro.
- **Función:** Registrar hechos relacionales explícitos, verificar la existencia de entidades y mantener el estado del mundo.
- **Escalabilidad:** 14.5 KB para 10 nodos, 1.7 MB para 1,000 nodos ($O(V + E)$ estricto).

### C. Sistema de Razonamiento Determinista
- **Algoritmos:** Búsqueda en anchura (BFS) sobre grafos dirigidos, catálogo de operadores discretos de reglas y composición de procedimientos.
- **Garantía:** Exactitud matemática del 100% en grafos y reglas cuando el problema está formalizado, detección de ciclos y prevención de desbordamiento de profundidad.
- **Latencia:** 26.27 µs por consulta completa en Rust nativo.

### D. Sistema de Evaluación, Incertidumbre y Selección de Acción
- **Formulación:**
  $$a^* = \arg\min_{a \in A(s)} J(s, a)$$
- **Clasificación Tripartita del Estado:**
  1. `VALID`: Entidades conocidas, campos presentes y restricciones solubles. Se ejecuta el procedimiento exacto.
  2. `INCOMPLETE`: Información insuficiente. Se solicita aclaración específica sin inventar datos.
  3. `INVALID`: Entidades inexistentes en la memoria episódica o contradicciones lógicas. Se emite un rechazo epistémico verificable.

---

## 3. Especificación Formal de RIR (RELIC Intermediate Representation)

El DSL canónico para representar problemas y planes de manera desacoplada del idioma:

```yaml
DOMAIN: SpatialGraph | SoftwareDependency | AlgebraicRule
GOAL:
    reach(target_state)
KNOWN:
    initial_state = A
    edges = [(A, B), (B, C)]
CONSTRAINTS:
    via(X)          # Restricción de paso obligatorio
    avoid(Y)        # Restricción de exclusión estricta
    max_cost(K)     # Límite de saltos o recursos
PLAN:
    A -> B -> C     # Cadena de transiciones propuesta
VERIFY:
    edges_valid = true
    constraints_satisfied = true
    cost_optimal = true
```

---

## 4. Resultados del Hito de Transferencia Isomórfica

Evaluación experimental completada sobre 5 pares isomórficos (10 consultas cross-domain) entre navegación espacial y dependencias de compilación de software en Rust (`reports/cross_domain_transfer_report.json`):

| Métrica Evaluada | Resultado Obtenido | Significado Científico |
| :--- | :---: | :--- |
| **Exactitud de Especificación RIR** | **100.0%** | Compilación exacta de la intención y slots en ambos dominios. |
| **Calibración de Incertidumbre** | **100.0%** | Rechazo epistémico (`REJECT_UNKNOWN`) y solicitud de datos (`ASK_CLARIFICATION`) sin alucinaciones. |
| **Tasa de Isomorfismo Procedural** | **100.0%** | La estructura abstracta del plan se conserva idéntica en grafos de ciudades y crates de Rust. |
| **Latencia Promedio por Par** | **1.54 ms** | Validación y ejecución integral de extremo a extremo. |

---

## 5. RELIC como una Familia de Especialistas Compactos

RELIC no se concibe como un modelo monolítico gigante ni como un enjambre multiagente tradicional que intercambia párrafos de texto libre con contextos inflados. Se concibe como una **familia de especialistas compactos** que comparten un estado cognitivo tipado:

```text
Entrada Lingüística (Español)
          ↓
[Compilador a RIR + Estado Cognitivo Compartido]
          ↓
┌─────────────────────────────────────────────────────────────┐
│              ESPECIALISTAS MODULARES COMPACTOS              │
│                                                             │
│  • Especialista Secuencial (H1: Memoria de trabajo y pila)  │
│  • Especialista Relacional (Grafos, dependencias y caminos) │
│  • Especialista Matemático (Patrones y transformaciones)    │
│  • Especialista Lingüístico (Interpretación y expresión)    │
└─────────────────────────────────────────────────────────────┘
          ↓
[Evaluador y Verificador RIR: Certifica validez y devuelve resultado]
```

**Diferencia clave frente a multiagentes tradicionales:**
- No intercambian lenguaje natural redundante.
- Operan sobre estructuras comunes fuertemente tipadas (RIR).
- Comparten representaciones y vocabularios, eliminando duplicación de memoria.

---

## 6. Decisiones de Diseño para un Consumo Mínimo de Memoria

1. **Compartir lo que no necesita duplicarse:**  
   Mismo tokenizador Unicode, mismas estructuras de memoria episódica y embeddings compartidos.
2. **Activación selectiva perezosa (*Lazy Activation*):**  
   Una consulta relacional no activa los especialistas matemáticos ni lingüísticos pesados. Un enrutador ultraligero activa solo el componente necesario, manteniendo el resto descargado o inactivo.
3. **Separación de algoritmos exactos vs aprendizaje:**  
   Un algoritmo BFS en Rust resuelve grafos con 100% de fiabilidad en 26 microsegundos. No gastamos millones de parámetros neuronales en reaprender lo que un algoritmo exacto resuelve con coste cero. El presupuesto de parámetros se reserva para lo que verdaderamente requiere aprendizaje.
4. **Distinción entre memoria persistente y memoria de trabajo:**  
   La memoria episódica crece con los hechos almacenados ($O(V+E)$, lineal y predecible); la memoria de trabajo de inferencia se mantiene acotada ($O(1)$ con H1).

---

## 7. Especialización y Auto-modificación Controlada

Para investigar la capacidad de especialización sin caer en degradación catastrófica ni desestabilizar el sistema:

1. **Adaptación de Conocimiento:** Añadir hechos, entidades y experiencias a la `EpisodicMemory` sin tocar los pesos neuronales.
2. **Especialización Controlada:** Entrenar o ajustar un especialista para una tarea concreta (por ejemplo, dependencias de software), conservando siempre congelada la versión anterior.
3. **Evolución Arquitectónica con Reversión Automática:**  
   Cualquier cambio de pesos o módulos candidatos debe someterse a una **suite de pruebas de regresión**. Si la especialización degrada el rendimiento en tareas previas o introduce alucinaciones, el sistema revierte automáticamente al estado de referencia certificado.

---

## 8. RELIC como Coprocesador Cognitivo de LLMs Mayores

Oportunidad de despliegue industrial como *sidecar* o coprocesador cognitivo local:

```text
Usuario / Aplicación
       ↓
[LLM Mayor: GPT-4 / Claude / Gemini]  ← Tareas lingüísticas difusas y creativas
       ↓  (Emite o solicita tarea formal RIR)
[RELIC Local: Core + EpisodicMemory + ReasoningEngine]  ← < 50 MB RAM, Privado, 26 µs
       ↓  (Resuelve transitividad, verifica restricciones y valida hechos sin alucinar)
Resultado Certificado y Verificado devuelto al LLM o al Usuario
```

---

## 9. Próximo Experimento: Comparación Cuádruple de Modularidad

Antes de construir especialistas múltiples, someteremos la hipótesis de modularidad a falsación empírica comparando cuatro variantes bajo el mismo presupuesto:

| Variante | Arquitectura Evaluada | Pregunta Experimental a Falsar |
| :---: | :--- | :--- |
| **A** | **H1 Único Compartido** | ¿Cuál es la capacidad y el coste de referencia de un modelo único? |
| **B** | **H1 con Cabezas Especializadas** | ¿Separar las cabezas de salida mejora el rendimiento sin duplicar el tronco? |
| **C** | **Especialistas Pequeños Independientes** | ¿La especialización estricta compensa el coste de tener redes separadas? |
| **D** | **Especialistas con Pesos Compartidos y Activación Selectiva** | ¿Es posible mantener capacidad especializada con activación perezosa de memoria? |

**Métricas a medir:** Exactitud por tarea, generalización OOD, RAM máxima residente (RSS), latencia p50/p95, parámetros totales vs activados, y tasa de interferencia catastrófica.

---

## 10. Los Tres Pilares Estratégicos de RELIC

1. **RELIC Core:** El runtime local de alto rendimiento en Rust puro, el DSL de especificación RIR, la memoria episódica indexada y los operadores deterministas exactos.
2. **RELIC Cognitive:** Los modelos y núcleos recurrentes compactos (H1) entrenados para mantener estados de trabajo $O(1)$, generalizar secuencias y aprender representaciones.
3. **RELIC Specialists:** Los componentes modulares especializados que se activan, ajustan o sustituyen de manera auditable y reversible.
