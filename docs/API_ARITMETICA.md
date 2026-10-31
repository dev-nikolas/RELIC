# API pública de `relic_core::arithmetic`

**Fecha:** 2026-10-30
**Commit de partida:** `ebbfd2c`
**Toolchain:** `rustc 1.99.1` / `cargo 1.99.1`
**Gate ejecutable:** `crates/relic_core/tests/arithmetic_public_api.rs` (6 tests)
**Documentos de fase:** `docs/AUDITORIA_FASES_0_1_BASELINE.md`,
`docs/AUDITORIA_FASES_2_3_MOTOR_DUAL.md`, `docs/AUDITORIA_FASE_4_SOLVER.md`

Este documento es el contrato, no la historia. Si algo cambia aquí, cambia la API.
Si cambia la implementación sin cambiar este documento, es un bug.

---

## 1. Qué es estable

La superficie estable es **exactamente lo que re-exporta `arithmetic/mod.rs`**, más
los métodos inherentes `pub` de esos tipos. Nada más.

| Estable | No estable |
|---|---|
| `relic_core::arithmetic::{...}` (39 nombres) | Rutas de módulo: `arithmetic::dual`, `arithmetic::solver`, … son privadas |
| Campos `pub` de los tipos re-exportados | Orden interno de los archivos, nombres de helpers privados |
| Firmas y variantes de los `enum` | `newton::run`, `newton::refine`, `certify::poly`, `certify::probe`, `certify::multiplicity` (libres, no re-exportadas) |
| `relic_core::{ArithmeticOp, ArithmeticPlan, ArithmeticError, ArithmeticReduction}` (`lib.rs:44`) | Todo lo demás del crate |

Dos matices que se suelen pasar por alto:

1. **Un módulo privado no oculta los métodos inherentes.** `HyperDual::sin` vive en
   `dual2_transcendental.rs`, que es privado, pero como `HyperDual` es re-exportado,
   `sin` es alcanzable. Por eso el contrato incluye los métodos, no solo los nombres
   del `pub use`.
2. **Los campos son `pub`.** `DualNumber { real, dual }`, `HyperDual { real, d1, d2 }`,
   `Poly2 { c0, c1, c2 }`, `ProgramClass { degree_bound, has_transcendental,
   var_in_denominator }` y `RpnProgram { opcodes, imms, len, _pad }` se pueden construir
   con literal de struct. Renombrar un campo, cambiar su tipo o su orden rompe
   consumidores y los `const _: () = assert!(size_of…)`.

---

## 2. Inventario congelado

39 nombres re-exportados: **4 históricos + 35 nuevos** (17 de Fase 3, 17 de Fase 4,
1 de Fase 5 — `extract`).

### 2.1 Contrato histórico (inmutable desde `ebbfd2c`)

| Nombre | Forma |
|---|---|
| `ArithmeticOp` | 4 variantes: `Add`, `Subtract`, `Multiply`, `Divide` |
| `ArithmeticError` | 3 variantes: `DivisionByZero`, `Overflow`, `InvalidNumber` |
| `ArithmeticReduction` | 5 campos: `step_index`, `left`, `op`, `right`, `result` |
| `ArithmeticPlan` | campos `initial`, `steps`, `unit`, `reductions`, `exact_division_note`; métodos `new`, `add_step`, `add_reduction`, `execute`, `format_result`, `format_formula` |

`plan.rs` es el archivo de `ebbfd2c` movido con `git mv` y `0 0` líneas de diff. Usa
`Vec`/`String` y **no está en la ruta de evaluación** del motor nuevo.

### 2.2 Motor de primer orden (Fase 2 + Fase 3)

| Nombre | Forma |
|---|---|
| `DualNumber` | `{ real, dual }`, 16 B; `new`, `constant`, `variable`, `checked_{add,sub,mul,div,pow}`; `Add`, `Sub`, `Mul`, `Neg`; `sin`, `cos`, `tan`, `exp`, `ln`, `sqrt`, `abs` |
| `DomainFault` | 10 variantes: `Ln`, `Sqrt`, `Pow`, `TanPole`, `NonDifferentiable`, `Indeterminate00`, `SingularDenominator`, `SingularDerivative`, `Overflow`, `NonFinite` |
| `DualResult` | `Result<DualNumber, DomainFault>` |
| `EPS_SINGULAR` | `1e-12` |
| `RpnProgram` | `{ opcodes: [DiffOp; 64], imms: [f64; 64], len: u8, _pad: [u8; 7] }`, 584 B; `new`, `push_const`, `push_op`, `len`, `is_empty`, `has_variable`, `validate`, `class` |
| `DiffOp` | 15 variantes, `#[repr(u8)]`: `PushConst=0` … `Abs=14`; `arity`, `is_transcendental` |
| `ProgramFault` | 4 variantes: `Full`, `StackUnderflow`, `StackOverflow`, `LeftoverStack` |
| `MAX_INSTRUCTIONS` / `MAX_DEPTH` / `UNKNOWN_DEGREE` | `64` / `32` / `u16::MAX` |
| `compile` | `fn(&str) -> Result<RpnProgram, LexFault>` |
| `LexFault` | 10 variantes: `Empty`, `InvalidChar`, `MalformedNumber`, `UnbalancedParen`, `TrailingOperator`, `TooDeep`, `ProgramFull`, `StackUnderflow`, `StackOverflow`, `LeftoverStack` |
| `eval` | `fn(&RpnProgram, DualNumber) -> Result<DualNumber, EvalFault>` |
| `DualStack` | 528 B; `new` |
| `EvalFault` | 5 variantes: `StackUnderflow`, `StackOverflow`, `LeftoverStack`, `MalformedProgram`, `Domain(DomainFault)` |
| `MAX_STACK` | `32` |
| `ProgramClass` | `{ degree_bound, has_transcendental, var_in_denominator }`; `is_linear`, `is_quadratic` |

### 2.3 Segundo orden y solver (Fase 4 + Fase 5)

| Nombre | Forma |
|---|---|
| `HyperDual` | `{ real, d1, d2 }`, 24 B; `new`, `constant`, `variable`, `checked_{add,sub,mul,div,pow}`, `Neg`; `sin`, `cos`, `tan`, `exp`, `ln`, `sqrt`, `abs` |
| `HyperResult` | `Result<HyperDual, DomainFault>` |
| `eval2` | `fn(&RpnProgram, HyperDual) -> Result<HyperDual, EvalFault>` |
| `HyperStack` | 776 B; `new` |
| `MAX_HYPER_STACK` | `32` |
| `Poly2` | `{ c0, c1, c2 }`, 24 B; `ZERO`, `ONE`, `X`, `constant`, `degree`, `scale`, `eval`, `add`, `sub`, `neg`, `mul`, `div`, `pow` |
| `PolyFault` | 8 variantes: `StackUnderflow`, `StackOverflow`, `LeftoverStack`, `DegreeExceeded`, `NonConstantDivisor`, `NonPolynomial`, `Malformed`, `Domain(DomainFault)` |
| `extract` | `fn(&RpnProgram) -> Result<Poly2, PolyFault>` (Fase 5) |
| `Certified` | `{ root, residual, criterion, multiplicity, iters }`, 24 B; `is_simple` |
| `Certification` | `Absolute`, `Relative` |
| `NewtonReport` | `{ root, residual, slope, curvature, iters }` |
| `NewtonFault` | 4 variantes: `ZeroDerivative`, `NoConvergence`, `NonFinite`, `Eval(EvalFault)` |
| `CERT_TOL` / `MAX_ITER` / `MAX_POLISH` | `1e-12` / `16` / `4` |
| `SolveOutcome` | 7 variantes: `Linear`, `Quadratic`, `DoubleRoot`, `Newton`, `NoRealRoots`, `InfinitelyMany`, `Abstain(SolveFault)` |
| `SolveFault` | 6 variantes: `Program`, `NeedsInitialGuess`, `Poly`, `Eval`, `Newton`, `Uncertified` |
| `solve` | `fn(&RpnProgram, Option<f64>) -> SolveOutcome` |

---

## 3. Invariantes que la API promete

### 3.1 Cero heap en la ruta de evaluación

`compile`, `eval`, `eval2`, `extract`, `solve` y `Newton` no asignan memoria dinámica.
No hay `Vec`, `Box`, `String`, `HashMap`, `format!` ni `to_string` en ningún módulo de
producción nuevo; la única excepción es `plan.rs`, que es el contrato histórico y no
participa en la ruta.

Verificación ejecutable: `tests/solver_zero_alloc.rs`, **20 000 iteraciones** sobre
tres programas (lineal, cuadrática con cancelación y `sin(x)` con 64 semillas), tras
calentamiento. Resultado medido: `ALLOC_COUNT == 0`, `ALLOC_BYTES == 0`.

Los tipos devueltos son `Copy`. El `RpnProgram` (584 B) y las pilas (528 / 776 B) los
posee el llamador o la pila del hilo.

### 3.2 Abstención determinista, nunca aproximación basura

Ninguna operación devuelve `NaN` o `inf` como si fuese un resultado. Cada fallo tiene
una variante concreta y el llamador puede distinguirlo:

| Estado | Tipo | Significado |
|---|---|---|
| `DomainFault` | 10 variantes | la operación no está definida en ese punto |
| `EvalFault` | 5 variantes | fallo de dominio o estructura del programa |
| `ProgramFault` | 4 variantes | el programa RPN no es evaluable |
| `LexFault` | 10 variantes | la expresión no compila |
| `PolyFault` | 8 variantes | no es un polinomio de grado ≤ 2 |
| `NewtonFault` | 4 variantes | no converge o no puede iterar |
| `SolveFault` | 6 variantes | el solver se abstiene por una causa declarada |

Reglas asociadas:

- `solve` sin semilla y sin clasificación polinómica ⇒ `Abstain(NeedsInitialGuess)`.
  No se prueban semillas "por si acaso".
- `NoRealRoots` solo se emite cuando el discriminante es negativo o la constante no es
  cero. No significa "no lo encontré".
- `InfinitelyMany` solo cuando el programa es idénticamente cero dentro de `CERT_TOL`.
- `multiplicity = 0` significa "mayor que 2 o indeterminada", no "triple".
- En `Certification::Relative`, `residual = 0.0` significa **despreciable frente a la
  escala**, no exactamente cero. El `criterion` distingue los dos casos.
- `RpnProgram::validate` rechaza un `len` corrupto con `ProgramFault::Full` en vez de
  hacer panic; `has_variable` devuelve `false` y `extract` devuelve
  `PolyFault::Malformed`.

### 3.3 `ArithmeticOp` está congelado

Exactamente 4 variantes. Hay **5 `match` exhaustivos** fuera del módulo que se rompen
si se añade una:

- `relic_server/src/engine.rs:33-38`
- `relic_cli/src/normalize.rs:121-126`
- `relic_cli/src/answer.rs:46-49`
- `relic_cli/src/inventory.rs:145-148`
- `relic_lexicon/src/calibrator.rs:47-50`

El gate `historical_variants_are_frozen` hace un `match` sin comodín sobre las 4
variantes, así que una variante nueva rompe también el test.

Los operadores nuevos (trig, `Pow`, `Sqrt`, …) viven en `DiffOp`, no en `ArithmeticOp`.

### 3.4 Tamaños exactos

Fijados con `const _: () = assert!(…)` en tiempo de compilación y repetidos en el gate
`layouts_are_frozen`.

| Tipo | `size_of` | `align_of` |
|---|---:|---:|
| `DualNumber` | 16 | 16 |
| `HyperDual` | 24 | 8 |
| `Poly2` | 24 | 8 |
| `Certified` | 24 | — |
| `DiffOp` | 1 | 1 |
| `RpnProgram` | 584 | 8 |
| `DualStack` | 528 | 16 |
| `HyperStack` | 776 | 8 |

### 3.5 Límites duros

| Constante | Valor | Qué acota |
|---|---:|---|
| `MAX_INSTRUCTIONS` | 64 | instrucciones por programa |
| `MAX_DEPTH` | 32 | pila de extracción de coeficientes |
| `MAX_STACK` / `MAX_HYPER_STACK` | 32 | valores simultáneos en `eval` / `eval2` |
| `MAX_OP_STACK` (privado) | 32 | anidamiento durante la compilación |
| `MAX_ITER` | 16 | iteraciones de Newton |
| `MAX_POLISH` | 4 | repaso sobre una raíz de fórmula cerrada |
| `EPS_SINGULAR` / `CERT_TOL` | `1e-12` | singularidad / certificación |

Ninguno es configurable. Cambiarlos cambia el contrato y los tests de tamaño o de
resultado.

### 3.6 Límite de 180 líneas

Ningún `.rs` de `arithmetic/` supera 180 líneas. El máximo actual es `poly2.rs` con
**180 exactas**; la siguiente adición obliga a dividirlo.

---

## 4. Lo que la API **no** promete

1. **Latencia.** No hay ninguna promesa de tiempo. Los dos tests de latencia de la
   línea base (`test_syndrome_latency_sub_50ns`, `test_lodf_simd_latency_sub_50ns`)
   siguen rojos, con los mismos umbrales y sin relación con este módulo.
2. **Número de iteraciones ni convergencia de Newton.** `MAX_ITER = 16` es un tope, no
   una garantía. Un programa puede abstenerse por `NoConvergence` o `ZeroDerivative`.
3. **Todas las raíces.** Se devuelve la raíz de la cuenca de la semilla. No hay
   bracketing, región de confianza ni barrido de semillas.
4. **Raíces complejas.** `NoRealRoots` es terminal.
5. **`Pow` con exponente variable.** `x^x` se abstiene en `HyperDual` con
   `DomainFault::Pow`; la regla general exige `ln a`.
6. **Notación científica en el lexer.** `1e10` se tokeniza como `1` y `e10`, y falla con
   `InvalidChar`. Limitación declarada, no bug silencioso.
7. **`RpnProgram: PartialEq` / `Debug`.** No se derivan. Los tests comparan `.err()`.
8. **Traza RPN.** `EvalFault` dice la causa, no la instrucción que la produjo.
9. **El nombre `ProgramClass::has_transcendental`.** Marca también exponentes literales
   no enteros (`x^2.5`). El nombre correcto sería `not_polynomial`; se conserva por
   compatibilidad.
10. **Conversión a `ArithmeticPlan`.** `SolveOutcome` no se traduce al contrato
    histórico. Son capas separadas a propósito.

---

## 5. Proceso de evolución

### 5.1 Añadir algo nuevo

1. Crear un módulo **privado** (`mod nombre;`), nunca `pub mod`.
2. Re-exportar el nombre mínimo en `mod.rs`, dentro del bloque de su capa.
3. Añadir el nombre al `use` de `tests/arithmetic_public_api.rs` y, si aplica, un test
   que lo ejercite.
4. Si el tipo tiene tamaño fijado, añadir `const _: () = assert!(size_of…)` y la fila
   en `layouts_are_frozen`.
5. Si toca la ruta de evaluación, volver a ejecutar `solver_zero_alloc`.
6. Documentar el cambio en el documento de fase correspondiente.

### 5.2 Cambiar algo existente

| Cambio | Veredicto |
|---|---|
| Añadir variante a `ArithmeticOp` | **Prohibido.** Rompe 5 `match` cross-crate y el gate |
| Renombrar/eliminar un nombre re-exportado | **Ruptura.** Requiere fase propia y actualizar este documento |
| Cambiar un campo `pub` o su orden | **Ruptura.** Rompe literales de struct y `size_of` |
| Añadir una variante al final de `DomainFault`/`SolveFault`/`EvalFault` | Ruptura de `match` exhaustivos externos; tratar como cambio mayor |
| Cambiar una fórmula numérica | Permitido **solo** con un test que fije el nuevo comportamiento y una nota de fase |
| Añadir una función privada o un helper | Libre, si el archivo sigue ≤ 180 líneas |
| Cambiar `EPS_SINGULAR` o `CERT_TOL` | Ruptura de comportamiento; exige revisar toda la matriz M1–M22 |

Regla general: **si el contrato no encaja, se añade un tipo o un módulo nuevo; no se
muta el existente.** Así se hizo con `DiffOp` frente a `ArithmeticOp` y con `Poly2`
frente a `ArithmeticPlan`.

### 5.3 Cómo se detecta una ruptura

| Gate | Detecta |
|---|---|
| `cargo check --workspace --all-targets` | nombres eliminados o firmas cambiadas |
| `tests/arithmetic_public_api.rs` | API congelada: 39 nombres, variantes, tamaños, abstención |
| `const _: () = assert!(size_of…)` | cambios de layout en tiempo de compilación |
| `tests/solver_zero_alloc.rs` | cualquier asignación nueva en la ruta `solve` |
| `solver_tests.rs` (M1–M22) | cambios de comportamiento numérico o de abstención |
| `relic_cli/tests/dual_engine_equivalence.rs` | divergencia bit a bit con el motor del CLI (30 expresiones) |

---

## 6. Estado medido de los gates

| Gate | Comando | Resultado |
|---|---|---|
| Compilación total | `cargo check --workspace --all-targets` | exit 0, **0 warnings** |
| Unitarios `relic_core` | `cargo test -p relic_core --lib` | **62/62** |
| API congelada | `cargo test -p relic_core --test arithmetic_public_api` | **6/6** |
| Cero asignaciones | `cargo test -p relic_core --test solver_zero_alloc` | **1/1**, 0 B / 0 alloc |
| Equivalencia CLI | `cargo test -p relic_cli --test dual_engine_equivalence` | **4/4** |
| Regresión `relic_core` | `cargo test -p relic_core --no-fail-fast` | **156 verdes**, 25 targets; solo los 2 rojos de latencia de Fase 0 |
| Límite de 180 líneas | recuento por archivo | máximo **180** (`poly2.rs`) |
| `lib.rs:44` | `pub use arithmetic::{…}` | intacto: las 4 rutas históricas siguen válidas |

Los 2 rojos son exactamente `test_syndrome_latency_sub_50ns` y
`test_lodf_simd_latency_sub_50ns`, con los mismos nombres y umbrales que en la línea
base. No hay ningún rojo nuevo.
