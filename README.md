# RELIC — Hoja de ruta y programa de investigación

> **Una nota de honestidad antes de empezar.** Este documento mezcla tres cosas, y las etiqueta para no confundirlas:
> **[Hecho]** existe en el código y se puede ejecutar.
> **[Plan]** está diseñado y es construible con los recursos actuales.
> **[Hipótesis]** es una idea a probar; puede resultar falsa.

---

## 1. Visión

RELIC no busca competir con los LLM grandes en escala. Busca explorar otra pregunta:

> ¿Puede un sistema compacto volverse más capaz **acumulando experiencia** (memoria, decisiones, correcciones) en lugar de crecer en parámetros?

Un LLM aprende a predecir la siguiente palabra y luego congela sus pesos. Eso le da fluidez, pero no memoria propia, no criterio estable y no adaptación posterior. RELIC parte de otra separación:

- **El lenguaje es una interfaz**, no el sustrato del pensamiento.
- **El conocimiento vive en una memoria**, y recordar es un proceso activo que ejecuta mientras avanza.
- **El comportamiento tiene una orientación estable** (personalidad), formada por las primeras experiencias y acotada por un núcleo que no cambia.
- **Los errores no se esconden ni se olvidan:** se registran, se corrigen y se vuelven pruebas permanentes.

### Lo que RELIC no pretende

- Igualar a un LLM de frontera en conocimiento general.
- Demostrar "conciencia" o "inteligencia general". Las analogías biológicas de este documento son **guías de diseño**, no afirmaciones sobre lo que el sistema es.
- Ser un servidor de modelos como Ollama. Su nicho es ser un motor pequeño, local y verificable.

---

## 2. Arquitectura conceptual: mente, cuerpo y núcleo

```text
┌─────────────────────────────────────────────────────────────┐
│ NÚCLEO INMUTABLE  (no se auto-modifica)                     │
│  pruebas de regresión · reglas base · evaluador · límites   │
│  de la personalidad                                         │
├─────────────────────────────────────────────────────────────┤
│ MENTE   decide, razona, detecta, recuerda                   │
│  RIR · evaluación VALID/INCOMPLETE/INVALID · memoria        │
│  asociativa · función de puntaje (personalidad)             │
├─────────────────────────────────────────────────────────────┤
│ CUERPO  ejecuta y se repara solo                            │
│  módulos, pesos, runtime · reparación y adaptación          │
│  La mente solo recibe una señal resumida ("zona débil",     │
│  "herida cerrada"), no los detalles de la reparación.       │
└─────────────────────────────────────────────────────────────┘
```

Reparto de responsabilidades:

| Capa | Qué hace | Ya existe en el código |
| :--- | :--- | :--- |
| Núcleo inmutable | Garantiza que el sistema no se degrade sin notarlo | Parcial: tests y conformidad RIR |
| Mente | Interpreta, recuerda, decide, razona | RIR, `reasoning_engine`, `episodic_memory`, evaluador |
| Cuerpo | Ejecuta y repara/adapta módulos | Runtime Rust y modelo; la auto-reparación es [Plan] |

**Por qué un núcleo inmutable:** un sistema que puede reescribir todo, incluido su propio evaluador, puede degradarse sin darse cuenta. La parte fija es la que permite juzgar si un cambio mejoró o empeoró.

**Metáfora guía:** la cicatrización, no el reinicio. La herida se cierra y el sistema queda más fuerte ahí; lo aprendido no se descarta.

---

## 3. Estado actual

Resumen (el detalle está en el [README](README.md)):

- **[Hecho]** Transformer 10M en Rust, tokenizador BPE, carga zero-copy de Safetensors.
- **[Hecho]** Motor aritmético determinista y ruta directa desde el CLI.
- **[Hecho]** RIR, motor de razonamiento (BFS, reglas), memoria episódica con hashes.
- **[Hecho]** Prueba de concepto de transferencia entre dominios (10 consultas).
- **[Hecho]** Front y back de demostración, aún sin conectar.
- **[Experimental]** Núcleo recurrente H1, con resultados mixtos.

---

## 4. Hoja de ruta por fases

### Fase 0 — Cerrar la base

Antes de añadir ideas nuevas, que lo que existe sea sólido y verificable.

- [ ] Conectar el front con el back en un solo flujo de punta a punta (por ejemplo: "¿A llega a C evitando B?" → plan verificado).
- [ ] **Router determinista:** toda consulta exacta (aritmética, grafos, reglas) va al motor; el Transformer solo atiende lenguaje.
- [ ] **Archivo de fallos conocidos:** cada error observado (empezando por el caso de "1 + 1") se vuelve un test automático. Es la semilla de la Fase 2.
- [ ] Publicar pesos y reportes (Releases u otro hosting) para que los benchmarks sean reproducibles.
- [ ] Unificar la documentación (cifras, licencia) y alinear el código con `CONTRIBUTING.md`.

**Criterio de éxito:** un demo reproducible de punta a punta y una suite de fallos conocidos en verde.

### Fase 1 — Memoria asociativa activa

**Idea [Hipótesis]:** el conocimiento no se "consulta", se **recorre**. Una pista activa nodos conectados, la activación se propaga por las aristas más fuertes, y cada eslabón que se visita puede ser un hecho (se lee) o un procedimiento (se ejecuta). No hay un paso de "primero busco todo, luego actúo".

Diseño propuesto [Plan]:

- **Grafo persistente en disco** (mapeado en memoria). En RAM solo viven el índice y el subgrafo activo de la tarea, de modo que guardar sea barato y el costo dependa de cuánto se recupera, no de cuánto se almacenó.
- **Aristas ponderadas** que se refuerzan con el uso.
- **Puntaje por salto:**
  `puntaje = peso de la arista × encaje con el camino hasta aquí + utilidad pasada + novedad`
  El contexto acumulado poda las ramas: de miles de salidas posibles solo unas pocas son compatibles con el camino recorrido.
- **Beam acotado:** se conservan las k mejores ramas y las demás quedan como alternativas por si hay que retroceder.
- **Normalización por número de salidas** para que los nodos muy conectados (hubs) no diluyan la activación (el llamado *fan effect*).
- **Ejecución perezosa:** el `ReasoningEngine` como iterador que recorre la cadena y ejecuta mientras avanza.
- **Protecciones al actuar con información incompleta:** modo ensayo (acciones simuladas o reversibles hasta confirmar el camino) y verificación en cada eslabón, con retroceso si algo no cuadra.
- **Olvido y consolidación:** peso decreciente de lo no usado y compresión de recuerdos redundantes; la memoria perfecta no es el objetivo, sino recuperar lo relevante.

**Experimento:** un grafo con hechos y 2-3 procedimientos; una consulta que recorra la cadena ejecutando mientras avanza.
**Criterio de éxito:** respuestas correctas con latencia y RAM que no crezcan al aumentar el tamaño total de la memoria (solo con el tamaño del subgrafo activo).

### Fase 2 — Cicatrización (rueda de Mahoraga)

Ciclo: **falla → registra → corrige → verifica → inmuniza.**

Niveles de adaptación, de más seguro a más arriesgado:

1. **Memoria:** añadir hechos o experiencias sin tocar pesos (cicatrices blandas).
2. **Reglas y router:** nuevas reglas en el motor determinista o en el enrutador.
3. **Pesos:** ajuste fino del modelo, solo con la suite de regresión como guardián.

Mecanismos:

- **Banco de pruebas permanente:** cada fallo corregido queda como test que ningún cambio futuro puede romper (cicatrices duras).
- **Rollback automático:** si un candidato degrada tareas previas o introduce alucinaciones, se revierte al último estado certificado.
- **Pesos protegidos** (técnicas tipo EWC): los pesos importantes para lo ya aprendido se vuelven difíciles de cambiar.
- **Crecimiento en lugar de sobrescritura:** módulos nuevos en vez de reescribir los existentes.
- **Señal de "herida" hacia la mente:** el cuerpo repara y la mente solo recibe el estado resumido.
- **Si el mismo fallo se repite:** se convierte en una regla o módulo nuevo; si es inevitable, se incorpora como parte normal de la operación.

**Problemas abiertos:** cómo juzgar que algo "sanó bien" (un evaluador débil permite que el sistema se desvíe sin notarlo), y el dilema estabilidad-plasticidad (demasiadas cicatrices duras vuelven rígido al sistema; hará falta consolidar y comprimir de vez en cuando).

**Criterio de éxito:** a lo largo de N ciclos de corrección, la tasa de regresiones sobre los tests anteriores se mantiene en cero.

### Fase 3 — Personalidad y orientación

**Idea [Hipótesis]:** la personalidad es el **sesgo permanente con el que se puntúa qué camino seguir**. Dos sistemas con la misma memoria y las mismas capacidades recorren cadenas distintas si pesan distinto. Tener capacidades no determina qué se hace con ellas.

Diseño propuesto [Plan]:

- **Núcleo de orientación inmutable:** valores base, qué cuenta como éxito o daño.
- **Pesos de personalidad en la función de puntaje:** términos como afinidad, novedad, cautela y utilidad pasada. Cambiar sus pesos cambia el carácter sin tocar la memoria.
- **Margen de evolución:** la experiencia puede desplazar los pesos poco a poco, siempre dentro de los límites fijados por el núcleo.
- **Formación temprana (imprinting):** los primeros conocimientos siembran los pesos iniciales. El orden y el contenido de esa "infancia" son una decisión de diseño explícita y deben documentarse.

Riesgos a vigilar:

- **Deriva:** si la experiencia mueve demasiado la personalidad, el sistema termina siendo otro. El margen del núcleo lo limita.
- **Sesgos que se refuerzan solos:** preferir cierto tipo de camino lleva a vivir solo esas experiencias. Se mitiga con exploración forzada.
- **Una infancia sesgada se arrastra:** conviene que los primeros datos sean variados y que el margen permita corregir.

**Experimentos:**
1. *Misma memoria, dos personalidades* (cautelosa vs. curiosa) ante una tarea con varias soluciones válidas.
2. *Mismas capacidades, dos "infancias"* (experiencias de cuidado vs. de exploración).

**Criterio de éxito:** cada variante elige de forma consistente caminos distintos entre sí, y ambas llegan a respuestas correctas. Si no hay diferencia sistemática, la hipótesis se descarta.

### Fase 4 — Mundo controlado y currículo

**Idea [Hipótesis]:** el lenguaje se aprende mejor **anclado a acciones y consecuencias** que como texto suelto.

- Un mundo mínimo (una cuadrícula con 5-10 palabras y 3 acciones) donde el agente asocia palabras con objetos y acciones por experiencia.
- **Currículo por etapas**, del nivel de un niño pequeño hacia arriba.
- **Variación y selección sobre piezas pequeñas** (reglas, parámetros del router, módulos concretos), conservando solo lo que mejora sin romper los tests. Evolucionar la red completa queda fuera de alcance con el cómputo disponible.

**Punto crítico:** la función de evaluación. Evolucionar solo sirve si "mejoró" se mide con fiabilidad; con una medida débil, el sistema aprende a engañar la prueba.

**Criterio de éxito:** el agente resuelve tareas del nivel N+1 reutilizando lo aprendido en el nivel N, con menos experiencia que partiendo desde cero.

### Fase 5 — Integración y despliegue

- **Sidecar de LLM:** un LLM mayor emite o solicita tareas en RIR; RELIC resuelve y devuelve un resultado verificado.
- **Librería** con API sencilla, además del CLI.
- **Portabilidad:** soporte de CPU sin AVX2 (detección en tiempo de ejecución con ruta de respaldo), ARM, y compilación a WASM.
- **Cuantización int8** para reducir pesos (~19 MB → ~10 MB).
- **Compilador lenguaje libre → RIR** aprendido, para reemplazar gradualmente el parser por reglas.

---

## 5. Hipótesis y cómo falsarlas

| # | Hipótesis | Experimento | Se descarta si |
| :-: | :--- | :--- | :--- |
| H-A | Recorrer y ejecutar la memoria a la vez mantiene latencia y RAM acotadas al crecer el conocimiento | Fase 1 con memorias de 10², 10⁴ y 10⁶ nodos | La latencia o la RAM crecen con el total almacenado |
| H-B | El ciclo de cicatrización evita regresiones | Fase 2 con N ciclos y banco de pruebas | Aparecen regresiones que el banco no detecta |
| H-C | Un sesgo estable produce comportamiento distinto y consistente | Fase 3, misma memoria con dos personalidades | No hay diferencias sistemáticas entre variantes |
| H-D | La infancia determina la orientación | Fase 3, dos currículos tempranos distintos | El orden inicial no altera el resultado final |
| H-E | El lenguaje anclado transfiere mejor entre niveles | Fase 4 | Entrenar desde cero iguala o supera al currículo |
| H-F | Modularidad: ¿compensan los especialistas pequeños frente a un modelo único? | Comparación A/B/C/D (H1 único, cabezas separadas, especialistas independientes, pesos compartidos con activación selectiva) | Un modelo único iguala la exactitud con menos RAM |

**Métricas comunes:** exactitud por tarea, generalización fuera de distribución, RSS máximo, latencia p50/p95, parámetros totales frente a activados, y tasa de interferencia catastrófica.

---

## 6. Problemas abiertos

Se enuncian a propósito; resolverlos (o descubrir que no se pueden resolver a esta escala) es parte del proyecto.

1. **Saber qué recordar:** elegir las pistas correctas y qué cadenas vale la pena seguir. Es lo que separa pensamiento de simple búsqueda.
2. **Evaluar la propia mejora:** un evaluador débil permite que el sistema "sane" mal.
3. **Estabilidad frente a plasticidad:** proteger lo aprendido sin volverse rígido.
4. **Aprender el puntaje de los saltos:** al inicio se define a mano; luego, a partir de qué caminos terminaron en éxito (quizá con una red pequeña como "juez").
5. **El puente lenguaje libre → RIR:** el cuello de botella actual de todo el sistema.
6. **Escala:** casi todo esto se plantea a tamaño chico. Que funcione ahí no garantiza que escale.

---

## 7. Marco conceptual y trabajo relacionado

Ideas con literatura previa en las que se apoya o con las que dialoga este diseño:

- **Activación por propagación y arquitecturas cognitivas** (ACT-R, Soar): memoria declarativa y procedimental, *fan effect*.
- **Aprendizaje continuo:** olvido catastrófico, dilema estabilidad-plasticidad, EWC, redes progresivas, pérdida de plasticidad.
- **Currículo y lenguaje anclado:** BabyAI, comunicación emergente, reto BabyLM.
- **Auto-mejora acotada por evaluación:** AlphaEvolve y la Darwin Gödel Machine (archivo de versiones y descarte de lo que no pasa las pruebas).
- **Neuro-simbólico:** modelos compactos que delegan el razonamiento exacto a módulos verificables.
- **Tolerancia a fallos:** supervisión y reinicio localizado (modelo de Erlang).

---

## 8. Los tres pilares

1. **RELIC Core:** runtime local en Rust, RIR, memoria y operadores exactos.
2. **RELIC Cognitive:** modelos compactos y recurrentes, memoria asociativa y puntaje de decisión.
3. **RELIC Adaptive:** cicatrización, personalidad y currículo; lo que permite que el sistema cambie sin perder lo que ya sabe.