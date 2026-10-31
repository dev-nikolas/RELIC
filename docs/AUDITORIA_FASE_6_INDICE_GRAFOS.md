# Auditoría Fase 6 — Índice jerárquico de alcanzabilidad sin heap

**Fecha:** 2026-10-30
**Commit de partida:** `5c569e6` (checkpoint de Fases 0–5)
**Toolchain:** `rustc 1.99.1` / `cargo 1.99.1`, perfil `dev`
**Documentos previos:** `docs/API_ARITMETICA.md` y las auditorías de Fases 0–4

Esta fase ataca el segundo cuello de botella: la consulta de alcanzabilidad de
`reasoning_engine.rs`, que asigna `Vec` por frontera, clona los buffers de
`search_buffer.rs` y materializa `String` por cada nodo del camino.

No se declara ninguna mejora de latencia. Los únicos números son bytes exactos,
conteos de tests, cotas $O(\cdot)$ y los resultados que los gates imprimen.

---

## 0. Punto de partida real (medido por inspección)

| Hecho | Ubicación |
|---|---|
| `Vec` por frontera (`frontier_fwd`, `nxt`, `meets`) | `reasoning_engine.rs:61,87,107` |
| `clone()` del conjunto cerrado | `reasoning_engine.rs:128` |
| `to_string()` por nodo del camino y del conjunto cerrado | `reasoning_engine.rs:131,152` |
| `BitAdjacencyGraph` existente: 64 nodos, 512 B | `contingency_bfs/graph.rs:6-15` |
| No existe especificación de Fase 6 en el repositorio | `git log -S "Fase 6"` vacío |

`reasoning_engine.rs` no se modifica en esta fase. El índice es aditivo.

### 0.1 Por qué no un índice por bloques y frontera

La alternativa natural era partir los ids en bloques de 64 y usar los nodos
incidentes a aristas entre bloques como frontera. Se descartó por una razón
estructural, no por una medición: los ids se asignan por orden de inserción, así
que el corte entre bloques es esencialmente arbitrario. En un grafo con grado
medio $d$, la probabilidad de que una arista cruce un corte con $B$ bloques es
$(B-1)/B$; con 16 bloques, el 93.75 % de las aristas son de corte y casi todo
nodo no aislado acaba siendo frontera. El índice se abstendría en los grafos
reales, que es justo lo contrario de lo que se busca.

El índice implementado no depende de la densidad ni del orden de inserción.

---

## 1. Contrato

```rust
pub const MAX_INDEX_NODES: usize = 1024;
pub const MAX_COMPONENTS: usize = 512;

pub enum IndexFault {
    TooManyNodes { nodes: usize, max: usize },
    TooManyComponents { max: usize },
}

pub enum ReachVerdict { Reachable, Unreachable, OutOfRange }

pub struct ReachHierarchy { /* campos privados */ }

impl ReachHierarchy {
    pub fn build(mem: &EpisodicMemory) -> Result<Self, IndexFault>;
    pub fn reaches(&self, u: u32, v: u32) -> ReachVerdict;
    pub fn node_count(&self) -> usize;
    pub fn component_count(&self) -> usize;
    pub fn component_of(&self, node: u32) -> Option<u16>;
}
```

`ReachVerdict` tiene tres estados y ninguno significa "no lo sé":
`Unreachable` es una respuesta **demostrada**. La abstención ocurre solo en
`build`, antes de existir el índice, y es un `IndexFault` explícito.

---

## 2. Algoritmo: dos niveles

### Nivel 0 — nodo → componente

Kosaraju iterativo con arrays fijos:

1. DFS sobre sucesores para obtener el orden de finalización.
2. DFS sobre predecesores en orden inverso para asignar componentes.

No hay recursión: la profundidad no depende de la pila del hilo. Los vecinos se
recorren en el orden en que `EpisodicMemory` los guarda (ordenado por nombre), así
que los ids de componente son deterministas para el mismo grafo.

### Nivel 1 — componente → cierre

Sobre la condensación (un DAG) se calcula el cierre transitivo con bitsets:

- una fila por componente, `[u64; 8]` (512 bits) ⇒ 64 B por fila;
- diagonal marcada antes del cierre;
- Floyd–Warshall con `row_k` copiada, sin préstamos mutables solapados.

Consulta:

```
si u == v            → Reachable
si comp(u) == comp(v) → Reachable          (misma SCC: mutuamente alcanzables)
si closure[comp(u)] tiene comp(v) → Reachable
en otro caso          → Unreachable
```

### 2.1 Exactitud

**Teorema.** `reaches(u, v)` devuelve `Reachable` si y solo si existe un camino
dirigido de `u` a `v`.

*Demostración.* Si `comp(u) = comp(v)`, hay camino en ambos sentidos por
definición de SCC. Si son componentes distintas, todo camino de `u` a `v` induce
un camino en la condensación, y el cierre contiene exactamente los pares
alcanzables del DAG. El recíproco es inmediato: cada bit del cierre proviene de
una arista real y la composición de caminos es un camino. ∎

No hay sobre-aproximación como en un grafo cociente de bloques: la condensación
preserva la alcanzabilidad **exactamente**, porque dentro de una SCC todos los
pares son alcanzables.

---

## 3. Memoria (bytes exactos)

`#[repr(C)]` y `const _: () = assert!(size_of::<ReachHierarchy>() == 34_824)`.

| Campo | Tipo | Bytes |
|---|---|---:|
| `closure` | `[[u64; 8]; 512]` | 32 768 |
| `component` | `[u16; 1024]` | 2 048 |
| `nodes` | `u16` | 2 |
| `components` | `u16` | 2 |
| `_pad` | `[u8; 4]` | 4 |
| **Total** | | **34 824** |

El índice completo (34 824 B) es un valor propiedad del llamador, igual que
`RpnProgram` (584 B). No hay `Box`, `Vec` ni `String` en ninguna parte del
módulo, ni siquiera durante la construcción.

El *scratch* de construcción vive en la pila: orden de finalización (4 KB),
pila de DFS (4 KB), cursor por nodo (4 KB) y bitset de visitados (128 B).

### 3.1 Capacidad y abstención

| Límite | Valor | Consecuencia al excederlo |
|---|---:|---|
| Nodos | 1024 | `TooManyNodes { nodes, max }` |
| Componentes (SCC) | 512 | `TooManyComponents { max }` |

Un DAG de 600 nodos tiene 600 SCC triviales y **se abstiene**; una cadena de 500
nodos se indexa (500 componentes). Un grafo de 1024 nodos aislados también se
abstiene, porque cada nodo aislado es su propia SCC. Es una limitación declarada,
no un fallo silencioso: el llamador conserva `ReasoningEngine` como respaldo.

---

## 4. Complejidad

Con $n$ nodos, $m$ aristas, $C$ componentes y $W = \lceil C/64 \rceil$ palabras:

| Operación | Coste |
|---|---|
| Kosaraju | $O(n + m)$ |
| Cierre | $O(C^{2} \cdot W)$ |
| Consulta | $O(1)$ |
| Memoria | $O(C \cdot W + n)$ |

El cierre usa `count`, no `MAX_COMPONENTS`: un grafo de 9 nodos no paga 512².
No se afirma nada sobre tiempos de pared.

---

## 5. Lo que el índice **no** hace

1. **No devuelve el camino.** Solo el veredicto. `Reachable` sin testigo no es un
   certificado verificable; para eso sigue estando `ReasoningEngine`.
2. **No reemplaza a `ReasoningEngine`.** No se ha tocado ni integrado. La
   integración (fast path con respaldo) es trabajo posterior.
3. **No es incremental.** Cualquier cambio en el grafo exige reconstruir.
4. **No indexa grafos fuera de capacidad.** 1024 nodos / 512 SCC como máximo.
5. **No usa nombres.** Trabaja sobre ids del interner; el mapeo nombre → id sigue
   siendo responsabilidad del llamador.
6. **No es serializable.** No hay formato en disco.
7. **No promete latencia.** Solo ausencia de asignación dinámica y $O(1)$ en la
   consulta.

---

## 6. Gates ejecutados

| Gate | Comando | Resultado |
|---|---|---|
| Compilación total | `cargo check --workspace --all-targets` | exit 0, **0 warnings** |
| Unitarios del índice | `cargo test -p relic_core --lib graph_index` | **13/13** |
| Unitarios `relic_core` | `cargo test -p relic_core --lib` | **75/75** |
| Propiedad vs BFS | `cargo test -p relic_core --test graph_index_property` | **3/3** |
| Cero asignaciones | `cargo test -p relic_core --test graph_index_zero_alloc` | **1/1** |
| Regresión `relic_core` | `cargo test -p relic_core --no-fail-fast` | **173 verdes**, 27 targets, solo los 2 rojos de Fase 0 |
| Límite de 180 líneas | recuento por archivo | máximo **149** |

### 6.1 Qué prueba cada gate

- **13 unitarios**: layout 34 824 B, grafo vacío, nodo único, cadena dirigida,
  ciclo (todos contra todos), dos ciclos unidos en un sentido, nodos aislados,
  diamante, ciclo con salida, fuera de rango, `TooManyNodes` (1025),
  `TooManyComponents` (601), determinismo de ids.
- **Propiedad**: 120 grafos aleatorios dirigidos de 2 a 23 nodos, con
  autobucles y nodos aislados, comparando **todas las parejas** contra un BFS
  exhaustivo independiente (48 000+ comparaciones). Además, un grafo fijo de 9
  nodos contrastado contra `ReasoningEngine` en las 81 parejas, y una cadena de
  500 nodos justo por debajo del límite de componentes.
- **Cero asignaciones**: construcción completa y **200 000 consultas** medidas con
  un `#[global_allocator]` propio; `ALLOC_COUNT == 0` en ambas ventanas.

### 6.2 Valores medidos de capacidad

| Grafo | Nodos | SCC | Resultado |
|---|---:|---:|---|
| Cadena de 500 | 500 | 500 | índice construido, exacto |
| Cadena de 601 | 601 | 601 | `Abstain(TooManyComponents)` |
| Cadena de 1025 | 1025 | — | `Abstain(TooManyNodes)` |

---

## 7. Deuda registrada

1. **Sin camino ni certificado positivo.** `Reachable` no viene con testigo. Un
   verificador externo no puede comprobarlo sin volver a buscar.
2. **Sin actualización incremental.** Un `insert_edge` invalida el índice entero.
3. **Cota de 512 SCCs.** Un DAG de más de 512 nodos se abstiene aunque tenga
   menos de 1024 nodos. Subirla a 1024 cuesta 128 KB de cierre; es un cambio de
   constante con su `assert` de tamaño, no de algoritmo.
4. **Construcción $O(C^2 W)$.** Con $C = 512$ son ~2 M operaciones de palabra;
   aceptable una vez, no para reconstruir por consulta.
5. **No integrado.** `ReasoningEngine` sigue siendo la única ruta de producción.
   La Fase 7 tendría que decidir el despacho: índice para grafos dentro de
   capacidad, respaldo al motor actual para el resto.
6. **Sin métrica de latencia comparativa.** No se ha medido el índice contra el
   motor; esta fase solo fija exactitud, memoria y ausencia de asignación.

---

## 8. Estado para la Fase 7

El índice ya responde con exactitud y sin heap para grafos dentro de capacidad.
Lo que falta es la política de convivencia con `ReasoningEngine`:

- ¿el índice es un fast path opcional que devuelve `Unreachable` de forma
  inmediata y delega los `Reachable` al motor para obtener camino?
- ¿o sustituye la consulta booleana y el motor solo se usa cuando se pide ruta?

Esa decisión cambia `relic_server`/`relic_cli` y no se toma aquí.
