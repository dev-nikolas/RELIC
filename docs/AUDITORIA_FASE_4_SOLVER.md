# Auditoría Fase 4 — Segundo orden, extracción exacta y solver

**Fecha:** 2026-10-30
**Commit de partida:** `ebbfd2c`
**Toolchain:** `rustc 1.99.1` / `cargo 1.99.1`, perfil `dev`
**Documentos previos:** `AUDITORIA_FASES_0_1_BASELINE.md`, `AUDITORIA_FASES_2_3_MOTOR_DUAL.md`

Fase 4 cierra el primer cuello de botella: el motor ya no solo evalúa y deriva,
también **resuelve** y **certifica**, con abstención formal cuando no puede
hacerlo. No se declara ninguna latencia: solo cotas $O(\cdot)$, bytes exactos y
valores que los tests imprimen o fijan.

---

## 1. Alcance

| Pieza | Archivo | Líneas | Rol |
|---|---|---:|---|
| Segundo orden | `arithmetic/dual2.rs` | 129 | `HyperDual` 24 B y reglas |
| Funciones hyperduales | `arithmetic/dual2_transcendental.rs` | 81 | `sin/cos/tan/exp/ln/sqrt/abs` |
| Evaluador de 2.º orden | `arithmetic/rpn_eval2.rs` | 116 | `eval2`, `HyperStack` 776 B |
| Coeficientes exactos | `arithmetic/poly2.rs` | 180 | `Poly2` 24 B, `extract` |
| Newton | `arithmetic/newton.rs` | 78 | `MAX_ITER = 16`, `MAX_POLISH = 4` |
| Certificación | `arithmetic/certify.rs` | 140 | `Absolute` / `Relative` |
| Despacho | `arithmetic/solver.rs` | 159 | `solve`, `SolveOutcome` |
| Matriz | `arithmetic/solver_tests.rs` | 176 | 22 tests M1–M22 |
| Reglas de 2.º orden | `arithmetic/dual2_tests.rs` | 88 | 9 tests |
| Gate zero-alloc | `tests/solver_zero_alloc.rs` | 69 | 1 test, 20 000 iteraciones |

Máximo por archivo: **180** (`poly2.rs`, exacto). Todos los `.rs` de
`arithmetic/` cumplen la regla.

---

## 2. `HyperDual`: 24 bytes

```rust
#[repr(C)]
pub struct HyperDual { pub real: f64, pub d1: f64, pub d2: f64 }
const _: () = assert!(size_of::<HyperDual>() == 24);
const _: () = assert!(align_of::<HyperDual>() == 8);
```

Representa `(f, f', f'')`. Reglas implementadas:

| Operación | `d1` | `d2` |
|---|---|---|
| `u ± v` | `u' ± v'` | `u'' ± v''` |
| `u · v` | `u'v + uv'` | `u''v + 2u'v' + uv''` |
| `u / v` | `u − qv` | `a''/b − 2uv + 2qv² − qb''/b` |
| `u^b` (b constante) | `b·a^{b−1}·u'` | `b(b−1)a^{b−2}u'² + b a^{b−1}u''` |
| `sin u` | `cos(a)·u'` | `−sin(a)u'² + cos(a)u''` |
| `cos u` | `−sin(a)·u'` | `−cos(a)u'² − sin(a)u''` |
| `exp u` | `e^a·u'` | `e^a(u'² + u'')` |
| `ln u` | `u'/a` | `u''/a − u'²/a²` |
| `sqrt u` | `u'/(2√a)` | `u''/(2√a) − u'²/(4a√a)` |
| `tan u` | `(1+t²)u'` | `2t(1+t²)u'² + (1+t²)u''` |
| `abs u` | `sign(a)·u'` | `sign(a)·u''` |

donde `q = a/b`, `u = a'/b`, `v = b'/b`. El cociente **no forma `b²`**, igual
que en Fase 2. Un exponente variable (`b' ≠ 0` o `b'' ≠ 0`) se rechaza con
`DomainFault::Pow`: la regla general exige `ln a` y no se improvisa.

`HyperStack` = 32 × 24 + 4 + 4 = **776 B**, alineado a 8.

---

## 3. `Poly2`: coeficientes exactos, no muestreo

`Poly2 { c0, c1, c2 }` = **24 B**. `extract(program)` recorre el RPN con una
pila de `Poly2` y propaga:

- `PushX → (0, 1, 0)`,
- `Mul` con grado resultante > 2 → `DegreeExceeded` (nunca truncamiento),
- `Div` con divisor no constante → `NonConstantDivisor`,
- `Pow` con exponente no entero o grado > 2 → fallo explícito,
- trascendentes → `NonPolynomial`.

Esto solo se invoca cuando `classify` ya demostró grado ≤ 2 sin trascendentes
ni variable en denominador, así que cualquier fallo es una inconsistencia
detectada, no un caso normal.

`extract` llama a `validate()` y devuelve `PolyFault::Malformed` si el programa
fue manipulado. Se añadió porque `RpnProgram` tiene campos públicos y un `len`
corrupto hacía panic en `validate`/`has_variable`; ahora devuelve
`ProgramFault::Full` y `false` respectivamente.

---

## 4. Solver: despacho sin heurística

| Entrada | Ruta | Semilla |
|---|---|---|
| sin variable | `InfinitelyMany` si `|f| ≤ 1e-12`, si no `NoRealRoots` | — |
| grado ≤ 1 demostrado | fórmula cerrada | ignorada |
| grado 2 demostrado | fórmula cerrada | ignorada |
| resto | Newton | **obligatoria** |

Sin semilla, un programa no polinómico devuelve
`Abstain(NeedsInitialGuess)`. No se prueba con 0, 1 o −1 "por si acaso":
eso sería una heurística no declarada.

### 4.1 Cuadrática estable

Con `S = max(|a|,|b|,|c|)` y `a' = a/S`, etc., se calcula
`disc = b'² − 4a'c'`. Como `|b'| ≤ 1`, **`disc` nunca desborda**: no se forma
`b²` en escala original. Las raíces usan la forma de Citardauq:

$$q=-\tfrac12\left(b'+\operatorname{sign}(b')\sqrt{\text{disc}}\right),\qquad
r_1=\frac{q}{a'},\qquad r_2=\frac{c'}{q}$$

que evita la cancelación de `(−b ± √disc)/(2a)` cuando `b² ≫ 4ac`.

### 4.2 Newton

```
repetir hasta MAX_ITER:
    (f, f', f'') = eval2(programa, x)
    si |f| ≤ 1e-12        → certificada
    si iter ≥ MAX_ITER    → NoConvergence
    si |f'| < 1e-12       → ZeroDerivative
    x ← x − f/f'
```

La parada es el **residuo**, no `|Δx|`. Un paso minúsculo con `f'` minúscula
no garantiza estar cerca de una raíz. `MAX_ITER = 16` y `MAX_POLISH = 4` son
topes duros: no hay iteración hasta convergencia.

---

## 5. Certificación: dos criterios, declarados

`Certified` = **24 B**:

```rust
pub struct Certified {
    pub root: f64,
    pub residual: f64,      // métrica ya normalizada; ≤ 1e-12 siempre
    pub criterion: Certification, // Absolute | Relative
    pub multiplicity: u8,   // 1 simple, 2 doble, 0 mayor/indeterminada
    pub iters: u8,
}
```

| Criterio | Definición | Cuándo se usa |
|---|---|---|
| `Absolute` | `|f(x)| ≤ 1e-12` con coeficientes originales | primera opción |
| `Relative` | `|f(x)| ≤ 1e-12 · Σ|aᵢ||x|ⁱ` | cuando `f(x)` desborda o la escala es enorme |

`Relative` es el error backward estándar. Es imprescindible: para
`x² − 10²⁰⁰x + 1`, evaluar en `x = 10²⁰⁰` forma `x² = 10⁴⁰⁰ = inf`, así que
`Absolute` es **imposible** aunque la raíz sea correcta a 1e-400 relativo. El
informe dice cuál se aplicó; no se disfraza uno de otro.

En la rama Newton, `Relative` equivale a `|f/f'| ≤ 1e-12·|x|`: el paso
correctivo es despreciable frente a `x`.

Multiplicidad: `|f'| > 1e-12 ⇒ 1`; `|f'| ≤ 1e-12` y `|f''| > 1e-12 ⇒ 2`;
resto `0`. Es información estructural, no una promesa de convergencia.

---

## 6. Matriz M1–M22 (medida)

Todas las filas están fijadas por un `#[test]` independiente.

| ID | Expresión / semilla | Resultado medido |
|---|---|---|
| M01 | `2*x + 1` | `Linear(-0.5)`, simple, `Absolute` |
| M02 | `x/2 - 3` | `Linear(6.0)` |
| M03 | `x` | `Linear(0.0)` |
| M04 | `5` | `NoRealRoots` |
| M05 | `0` | `InfinitelyMany` |
| M06 | `x^2 - 1` | `Quadratic(-1.0, 1.0)`, ambas simples |
| M07 | `x^2 - 2*x + 1` | `DoubleRoot(1.0)`, multiplicidad 2 |
| M08 | `x^2 + 1` | `NoRealRoots` |
| M09 | `x^2 - 1e8·x + 1` | `1.00000000000000019e-8` (`Absolute`) y `9.99999999999999851e7` (`Relative`, residuo `2.45e-17`) |
| M10 | `x^2 - 1e200·x + 1` | `9.99999999999999982e-201` (`Absolute`) y `9.99999999999999970e199` (`Relative`) |
| M11 | `sin(x)`, x0 = 3 | `3.14159265358979312`, residuo `1.225e-16`, 3 iteraciones |
| M12 | `exp(x) - 2`, x0 = 0 | `6.93147180560025444e-1`, residuo `1.603e-13`, 5 iteraciones |
| M13 | `ln(x) - 1`, x0 = 2 | `2.71828182845893762`, residuo `3.963e-14`, 4 iteraciones |
| M14 | `sqrt(x) - 3`, x0 = 10 | `9.0`, residuo `0.0`, 4 iteraciones |
| M15 | `1/x - 2`, x0 = 0.3 | `4.99999999999907740e-1`, residuo `3.690e-13`, 5 iteraciones |
| M16 | `abs(x) - 1`, x0 = −0.5 | `-1.0`, residuo `0.0`, 1 iteración |
| M17 | `sin(x)` sin semilla | `Abstain(NeedsInitialGuess)` |
| M18 | `ln(x) - 1`, x0 = −1 | `Abstain(Newton(Eval(Domain(Ln))))` |
| M19 | `x^3`, x0 = 0 | `Newton(0.0)`, multiplicidad 0, 0 iteraciones |
| M20 | `exp(x) + 1`, x0 = 0 | `Abstain(Newton(ZeroDerivative))` |
| M21 | `x^2 + 2*x + 1` | `DoubleRoot(-1.0)` |
| M22 | `-3*x + 6` | `Linear(2.0)` |

Los residuos de M11–M15 son los que el test comparó contra `1e-12`; no son
cotas teóricas. M20 no es un fallo del solver: `exp(x)+1` no tiene raíz real,
Newton diverge hacia `−∞` hasta que `exp` subdesborda a `0` y `f' = 0`; la
abstención es la respuesta correcta.

### 6.1 Lo que cada bloque protege

- **M01–M05**: linealidad, constantes y los dos casos degenerados.
- **M06–M08**: discriminante positivo, cero y negativo.
- **M09**: cancelación (`b² ≫ 4ac`); sin la forma de Citardauq la raíz pequeña
  se pierde.
- **M10**: `b²` inalcanzable; obliga a usar `Relative` y demuestra que el
  solver no depende de evaluar `x²`.
- **M11–M16**: las seis rutas de Newton (trig, exp, log, sqrt, racional, abs).
- **M17–M18**: abstención por falta de semilla y por dominio.
- **M19–M20**: raíz múltiple y divergencia; ambas devuelven estado formal.
- **M21–M22**: simetría (doble raíz negativa) y pendiente negativa.

---

## 7. Gate de cero asignaciones

`tests/solver_zero_alloc.rs` replica el `ZeroAllocTracker` de
`kernel_end_to_end_stress_audit.rs:26-43` en su propio binario (un
`#[global_allocator]` solo puede declararse una vez por binario; los `static`
del otro test no son importables).

Ventana medida: **20 000 iteraciones** sobre tres programas (lineal,
cuadrática con cancelación y `sin(x)` con 64 semillas distintas), tras un
calentamiento de 8 rondas.

Resultado: `ALLOC_COUNT == 0`, `ALLOC_BYTES == 0`, checksum finito. La ruta
`solve → validate → class → extract|certify → eval2` no toca el heap.

---

## 8. Gates ejecutados

| Gate | Comando | Resultado |
|---|---|---|
| Compilación total | `cargo check --workspace --all-targets` | exit 0 |
| Unitarios `relic_core` | `cargo test -p relic_core --lib` | **62/62** |
| Zero-alloc | `cargo test -p relic_core --test solver_zero_alloc` | **1/1**, 0 asignaciones |
| Kernel preexistente | `cargo test -p relic_core --test kernel_end_to_end_stress_audit` | 1/1 (el assert `< 64 KB` sigue intacto) |
| Regresión `relic_core` | `cargo test -p relic_core --no-fail-fast` | 149 verdes, solo los 2 rojos de Fase 0 |
| `relic_blackwall` | `cargo test -p relic_blackwall` | exit 0, 10 targets |
| `relic_cli` | `cargo test -p relic_cli` | exit 0, 11 targets |
| `relic_server` | `cargo test -p relic_server` | exit 0, 9 targets |
| `relic_lexicon` | `cargo test -p relic_lexicon` | exit 0, 5 targets |
| `relic_mcp` | `cargo test -p relic_mcp` | exit 0, 4 targets |

`relic_core --lib` pasó de 30 a **62** tests: +9 de `dual2`, +22 de la matriz,
+1 del `len` corrupto. `relic_core` pasó de 23 a **24 targets** por el nuevo
binario de zero-alloc. Los 2 rojos de latencia siguen siendo los de la línea
base, con los mismos nombres y sin tocar umbrales.

`relic_benchmarks` se compiló (`--all-targets`) pero no se re-ejecutó.

---

## 9. Deuda registrada

1. **`poly2.rs` está exactamente en 180 líneas.** Cualquier adición exige
   dividirlo (`poly2` + `poly2_extract`). No se hizo antes para no introducir
   un archivo más sin necesidad.
2. **Newton no tiene región de confianza ni bracketing.** Si la semilla está
   en una cuenca equivocada, devuelve la raíz de esa cuenca o se abstiene; no
   busca todas las raíces. Documentado, no oculto.
3. **Solo raíces reales.** `NoRealRoots` es terminal; no hay raíces complejas.
4. **Un programa con `Pow` de exponente variable se abstiene en `HyperDual`.**
   `x^x` no se puede certificar con esta implementación.
5. **`Certified::residual` en la rama `Relative` puede ser `0.0`** cuando el
   cociente real cae por debajo de `f64::MIN_POSITIVE`; significa
   "despreciable", no "exactamente cero". El `criterion` lo distingue.
6. **La superficie pública creció otra vez** (Fase 4 re-exporta 17 nombres más:
   `HyperDual`, `Poly2`, `Certified`, `SolveOutcome`, etc.). La revisión en bloque
   llegó en Fase 5: ver `docs/API_ARITMETICA.md`.

---

## 10. Estado para la Fase 5

El solver ya devuelve un estado formal para cada entrada: raíz certificada con
criterio declarado, `NoRealRoots`, `InfinitelyMany` o un `SolveFault` concreto.
La Fase 5 puede exponerlo sin inventar semántica: no hay `Option<f64>` ambiguo
ni `NaN` que interpretar. Lo que falta es decidir la superficie pública final
y si `SolveOutcome` debe implementar conversiones hacia el contrato histórico
de `ArithmeticPlan` sin tocar `ArithmeticOp`.

---

> **Actualización (Fase 5):** la superficie quedó congelada. Los módulos de
> implementación pasaron a privados, se re-exportan 39 nombres (4 históricos +
> 35 nuevos), se añadió `extract` y el gate `tests/arithmetic_public_api.rs`.
> El contrato completo vive en `docs/API_ARITMETICA.md`. El punto 6 de la deuda
> (revisión en bloque) queda cerrado; `SolveOutcome` **no** se convierte a
> `ArithmeticPlan`, por decisión explícita.
