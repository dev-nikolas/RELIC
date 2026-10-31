# Auditoría Fase 2 + Fase 3 — Motor dual y compilador RPN

**Fecha:** 2026-10-30
**Commit de partida:** `ebbfd2c`
**Toolchain:** `rustc 1.99.1` / `cargo 1.99.1`, perfil `dev` (debug)
**Documento previo:** `docs/AUDITORIA_FASES_0_1_BASELINE.md`

Este documento cubre el motor algebraico dual (Fase 2) y el compilador/evaluador RPN
(Fase 3). No declara ninguna mejora de latencia: los únicos números son conteos de
líneas, conteos de tests, bytes exactos de `size_of`/`align_of` y cotas $O(\cdot)$.

---

## 0. Corrección respecto al diseño previo

El documento de diseño afirmaba que la derivada del cociente era

$$\left(\frac{u}{v}\right)'=\frac{\beta-q\alpha}{b},\qquad q=\frac{a}{b}$$

**Eso tiene los signos invertidos.** La identidad correcta es

$$\left(\frac{u}{v}\right)'=\frac{\alpha b-a\beta}{b^{2}}
=\frac{\alpha-(a/b)\beta}{b}
=\frac{\alpha-q\beta}{b},\qquad q=\frac{a}{b}$$

con $u=a+\alpha\varepsilon$, $v=b+\beta\varepsilon$. La implementación
(`crates/relic_core/src/arithmetic/dual.rs:86-92`) usa la forma corregida:

```rust
pub fn checked_div(self, rhs: Self) -> DualResult {
    if rhs.real.abs() < EPS_SINGULAR {
        return Err(DomainFault::SingularDenominator);
    }
    let q = self.real / rhs.real;
    let dual = (self.dual - q * rhs.dual) / rhs.real;
    Self::new(q, dual).checked()
}
```

### 0.1 Por qué esta forma es la blindada

1. **Identidad algebraica, no aproximación.** $(\alpha-q\beta)/b=(\alpha b-a\beta)/b^{2}$
   para todo $b\neq 0$. El cambio es de *rango*, no de precisión.
2. **Rango.** La forma clásica forma $b^{2}$, que desborda a `inf` para
   $|b|\gtrsim 1.34\times10^{154}$ aunque $b$ sea representable. Con $b=10^{200}$,
   $b^{2}\to\infty$ y el resultado colapsa. La forma con $q$ nunca materializa $b^{2}$.
3. **Determinismo.** Una sola división final reutiliza $q$; no hay FMA ni reasociación,
   así que el bit de resultado es reproducible entre targets.

### 0.2 Lo que la Fase 1 **no** podía blindar

La Fase 1 fue un renombrado puro (`git diff -cached -M --numstat` = `0 0` en `plan.rs`).
No contenía una sola línea de derivada, así que **no podía** blindar el signo: no había
nada que corregir allí. El blindaje efectivo es doble y vive en Fase 2:

- la fórmula implementada, y
- el test `quotient_rule_has_correct_sign`, que falla si alguien vuelve a intercambiar
  $\alpha$ y $\beta$.

Lo único que garantizó la Fase 1 es **atribuibilidad**: al no tocar `plan.rs`, cualquier
cambio numérico posterior es necesariamente atribuible a un archivo nuevo, nunca a una
edición encubierta del contrato histórico.

---

## 1. Alcance y no-objetivos

### 1.1 Entregado

| Pieza | Archivo | Contenido |
|---|---|---|
| Número dual + reglas | `arithmetic/dual.rs` | `DualNumber`, `DomainFault`, `checked_{add,sub,mul,div,pow}` |
| Funciones elementales | `arithmetic/dual_transcendental.rs` | `sin`, `cos`, `tan`, `exp`, `ln`, `sqrt`, `abs` |
| Programa RPN | `arithmetic/program.rs` | `DiffOp` (15 variantes), `RpnProgram`, `validate` |
| Clasificación | `arithmetic/classify.rs` | `ProgramClass`, `is_linear`, `is_quadratic` |
| Compilador | `arithmetic/lexer/{mod,scan}.rs` | Shunting-Yard con pila fija |
| Evaluador | `arithmetic/rpn_eval.rs` | `eval`, `DualStack`, `EvalFault` |
| Tests del motor | `arithmetic/dual_tests.rs` | 13 tests |
| Tests del compilador | `arithmetic/rpn_tests.rs` | 8 tests |
| Gate cross-crate | `relic_cli/tests/dual_engine_equivalence.rs` | 4 tests, bit a bit contra el CLI |

### 1.2 No-objetivos explícitos

- **No** se amplía `ArithmeticOp` (rompería 5 `match` exhaustivos cross-crate; ver
  documento de Fase 0/1, §1.3). Los operadores nuevos viven en `DiffOp`.
- **No** se modifica `plan.rs` ni su contrato (`ArithmeticPlan`, `ArithmeticError`,
  `ArithmeticReduction`).
- **No** hay solver de ecuaciones todavía: `classify.rs` solo expone la inferencia que
  la Fase 4 consumirá.
- **No** hay traza RPN (`RpnTrace`): diferida.
- **No** se integra nada en `TacticalKernelContext`.

---

## 2. Inventario y límite de 180 líneas

| Archivo | Líneas | ≤ 180 |
|---|---:|---|
| `arithmetic/mod.rs` | 30 | ✓ |
| `arithmetic/plan.rs` (heredado, verbatim) | 168 | ✓ |
| `arithmetic/dual.rs` | 174 | ✓ |
| `arithmetic/dual_transcendental.rs` | 75 | ✓ |
| `arithmetic/program.rs` | 142 | ✓ |
| `arithmetic/classify.rs` | 105 | ✓ |
| `arithmetic/lexer/mod.rs` | 171 | ✓ |
| `arithmetic/lexer/scan.rs` | 132 | ✓ |
| `arithmetic/rpn_eval.rs` | 112 | ✓ |
| `arithmetic/dual_tests.rs` | 161 | ✓ |
| `arithmetic/rpn_tests.rs` | 125 | ✓ |
| `relic_cli/tests/dual_engine_equivalence.rs` | 98 | ✓ |

Producción: 1109 líneas. Tests: 384 líneas. Máximo por archivo: **174**.
`lexer.rs` se escribió primero como archivo único (~250 líneas) y se dividió en
`lexer/mod.rs` + `lexer/scan.rs` antes de continuar; no quedó ningún archivo por encima
del límite.

---

## 3. Representación en memoria (bytes exactos)

Todos los tamaños están fijados con `const _: () = assert!(...)` en tiempo de
compilación, no con un test en tiempo de ejecución.

| Tipo | `size_of` | `align_of` | Composición |
|---|---:|---:|---|
| `DualNumber` | **16** | **16** | `real: f64` + `dual: f64`, `#[repr(C, align(16))]` |
| `DiffOp` | **1** | 1 | `#[repr(u8)]`, 15 variantes |
| `RpnProgram` | **584** | 8 | `[DiffOp; 64]` + `[f64; 64]` + `len: u8` + `_pad: [u8; 7]` |
| `DualStack` | **528** | 16 | `[DualNumber; 32]` + `sp: u32` + `_pad: [u8; 12]` |

Capacidades:

- `MAX_INSTRUCTIONS = 64` ⇒ `RpnProgram` acotado a 64 instrucciones.
- `MAX_DEPTH = 32` ⇒ profundidad máxima de pila de datos.
- `MAX_OP_STACK = 32` ⇒ anidamiento máximo de paréntesis/funciones durante la compilación.
- `MAX_STACK = 32` ⇒ valores simultáneos en `eval`.

Complejidad: `compile` es $O(n)$ sobre los bytes de la expresión con espacio $O(1)$;
`eval` y `RpnProgram::class` son $O(\text{len}) \le O(64)$ con espacio $O(1)$.

---

## 4. Abstención epistémica

`DomainFault` es el estado formal de abstención. Ninguna operación devuelve `NaN` o
`inf` como si fuese un resultado: `checked()` rechaza cualquier componente no finita.

| Variante | Se emite cuando |
|---|---|
| `Ln` | argumento $\le 0$ |
| `Sqrt` | radicando $< 0$ |
| `Pow` | base $\le 0$ con exponente no entero (salvo el caso $0^{b}$, $0<b<1$) |
| `TanPole` | $\lvert\cos x\rvert < 10^{-12}$ |
| `NonDifferentiable` | `abs` en $x=0$ con derivada $\neq 0$ |
| `Indeterminate00` | $0^{0}$ |
| `SingularDenominator` | $\lvert b\rvert < 10^{-12}$, o $0^{b}$ con $b<0$ entero |
| `SingularDerivative` | `sqrt(0)` con derivada $\neq 0$; $0^{b}$ con $0<b<1$ o $b<0$ no entero |
| `Overflow` | resultado real no finito (`exp` grande, `pow` grande) |
| `NonFinite` | entrada o resultado `NaN`/`inf` |

`EvalFault` envuelve `DomainFault` (`EvalFault::Domain`) y añade los fallos
estructurales: `StackUnderflow`, `StackOverflow`, `LeftoverStack`, `MalformedProgram`.

### 4.1 Tabla de potencias

`checked_pow` distingue explícitamente:

1. exponente con derivada nula y entero ⇒ regla $b\,a^{b-1}\alpha$, válida para base negativa;
2. $a=0$ con $b\ge 0$ ⇒ casos $b=0$, $b=1$, $b>1$ (derivada nula salvo $b=1$);
3. base positiva ⇒ $a^{b}\left(\beta\ln a+\dfrac{b\alpha}{a}\right)$;
4. $0<b<1$ con $a=0$ ⇒ `SingularDerivative` (derivada lateral infinita);
5. resto ⇒ `Pow`.

El caso 4 se añadió durante la fase: sin él, `0^0.5` caía en `Pow`, que describe el
dominio del valor pero no la singularidad de la derivada.

---

## 5. La prueba de $b=10^{200}$

`dual_tests.rs:38-51`:

```rust
let u = d(1.0, 3.0);       // u = 1 + 3ε
let v = d(1e200, 0.0);     // v = 1e200
assert!((v.real * v.real).is_infinite());   // b² = 1e400 = inf

let robust = u.checked_div(v).unwrap();
assert_eq!(robust.real, 1.0 / 1e200);       // 1e-200, exacto
assert_eq!(robust.dual, 3.0 / 1e200);       // 3e-200, exacto
assert!(robust.dual > 0.0);

let classic = (u.dual * v.real - u.real * v.dual) / (v.real * v.real);
assert_eq!(classic, 0.0);                   // pérdida total
```

Resultados exactos de la corrida:

| Forma | `.real` | `.dual` |
|---|---|---|
| $(α-qβ)/b$ (implementada) | `1e-200` | `3e-200` |
| $(αb-aβ)/b^{2}$ (clásica) | `1e-200` | **`0.0`** |

Los dos `assert_eq!` son bit a bit porque ambos lados ejecutan la misma división
(`1.0 / 1e200` y `3.0 / 1e200`), no una comparación con tolerancia.

`dual_tests.rs:54-63` cubre el otro modo de fallo de la forma clásica: si el numerador
también desborda, el resultado es `inf / inf = NaN`. Con $u=10^{200}+10^{200}\varepsilon$
y $v=10^{200}$:

| Forma | Resultado |
|---|---|
| implementada | `(1.0, 1.0)` |
| clásica | `NaN` |

---

## 6. Gate de equivalencia con el motor existente

`crates/relic_cli/tests/dual_engine_equivalence.rs` compara el motor nuevo contra
`relic_cli::math` — el Shunting-Yard que ya usaba el producto — con **30 expresiones**
y `assert_eq!(cli.to_bits(), new.to_bits())`.

Usar `to_bits()` en vez de `==` evita que pasen desapercibidos `-0.0` vs `0.0` y
payloads de `NaN`. El corpus cubre:

- precedencia y asociatividad (`2 + 3 * 4`, `8 / 2 / 2`, `2 - 3 - 4`);
- signos unarios (`-5 + 3`, `2 - -3`, `3 * (-2)`);
- paréntesis anidados (`((1 + 2) * (3 + 4)) / 5`);
- decimales sin cero inicial (`.5 + .25`) y con él (`12.345 + 67.89`);
- magnitudes de $10^{-1}$ a $10^{12}$ (`0.1 + 0.2`, `1000000 * 1000000`);
- casos límite (`0 - 0`, `1 + 0`, `1 - 0.9`, `0.3 * 3`).

Resultado: 4/4 tests verdes. `corpus_is_not_trivial` verifica que el corpus contiene al
menos un valor no entero y al menos uno $>10^{6}$, para que el gate no sea trivialmente
satisfacible.

### 6.1 Divergencias deliberadas y documentadas

| Caso | CLI (`relic_cli::math`) | Motor nuevo | Razón |
|---|---|---|---|
| `2 x 3` | `6.0` (`x` = multiplicación) | `LexFault::InvalidChar` | `x` es la incógnita; no hay ambigüedad silenciosa |
| `1 / 0` | error con `ArithmeticOp::Divide` | `EvalFault::Domain(SingularDenominator)` | abstención formal, no error de operador |
| `^`, `sin`, `cos`, `tan`, `exp`, `ln`, `sqrt`, `abs`, variable `x` | no soportado | soportado | superconjunto del lenguaje del CLI |

Las dos primeras están fijadas por tests (`documented_divergence_variable_vs_multiplication`,
`documented_divergence_division_by_zero`) para que un cambio accidental rompa la suite.

---

## 7. Bugs reales encontrados durante la fase

Se registran porque ninguno se detectó por inspección: los tres los encontró la suite.

1. **Off-by-one en `close_paren`.** Tras `self.sp -= 1`, se indexaba `kinds[self.sp]`
   (el elemento *anterior* al paréntesis) en lugar de `kinds[self.sp - 1]`. Resultado:
   `sin(x)`, `-(2+3)` y `(2+3)*4` devolvían `UnbalancedParen`. Corregido en
   `lexer/mod.rs:106-108`; lo detectaron `arithmetic_and_derivatives` y
   `domain_faults_cross_the_evaluator`.
2. **`assert!(expr).is_infinite()`** en vez de `assert!((expr).is_infinite())` en el test
   de $10^{200}$: el `!` se aplicaba al `f64`. Detectado por el compilador.
3. **`assert_eq!(compile(...), Err(...))`** exige `PartialEq` en `RpnProgram`, que no lo
   deriva. Se cambió a `compile(...).err()`; comparar solo el error es además más
   informativo que comparar programas de 584 bytes.

---

## 8. Estado de asignación dinámica

Búsqueda de `Vec`, `Box`, `String`, `HashMap`, `format!` y `to_string` sobre los módulos
de producción:

```
plan.rs: 39, 40, 41, 42, 46, 47, 94, 97, 99, 101, 106, 108,
         116, 117, 118, 127, 128, 132, 137, 141, 143, 147, 159
lexer/mod.rs: 4   (solo el comentario «No hay `Vec` en ninguna etapa»)
```

**Todas las ocurrencias ejecutables están en `plan.rs`**, el archivo heredado que no se
puede tocar sin romper el contrato de `format_formula` y `ArithmeticPlan`. La única
mención en un módulo nuevo es un doc-comment que afirma lo contrario de lo que la
búsqueda detecta. En `dual.rs`, `dual_transcendental.rs`, `program.rs`, `classify.rs`,
`lexer/scan.rs` y `rpn_eval.rs` no hay ni una ocurrencia.

Los tests sí asignan (`"(".repeat(33)`, `vec!["1"; 40].join("+")`) porque construyen
entradas adversariales; eso no afecta a la ruta de evaluación.

Consecuencia estructural: `eval` no necesita heap. Su marco contiene un `DualStack` de
**528 B** y un `DualNumber` de **16 B** por argumento; el `RpnProgram` de **584 B** lo
posee el llamador. No se declara latencia, solo ausencia de asignación dinámica.

---

## 9. Gates ejecutados

| Gate | Comando | Resultado |
|---|---|---|
| Compilación total | `cargo check --workspace --all-targets` | exit 0 |
| Unitarios del motor | `cargo test -p relic_core --lib` | **30/30** (9 `plan` + 13 `dual` + 8 `rpn`) |
| Regresión `relic_core` | `cargo test -p relic_core --no-fail-fast` | solo los 2 rojos preexistentes de Fase 0, mismos nombres |
| Equivalencia bit a bit | `cargo test -p relic_cli --test dual_engine_equivalence` | **4/4** |
| `relic_blackwall` | `cargo test -p relic_blackwall --no-fail-fast` | exit 0, 10 targets |
| `relic_cli` | `cargo test -p relic_cli --no-fail-fast` | exit 0, **11 targets** (antes 10) |
| `relic_server` | `cargo test -p relic_server --no-fail-fast` | exit 0, 9 targets |
| `relic_lexicon` | `cargo test -p relic_lexicon --no-fail-fast` | exit 0, 5 targets |
| `relic_mcp` | `cargo test -p relic_mcp --no-fail-fast` | exit 0, 4 targets |

`relic_benchmarks` no se re-ejecutó en esta fase; sí se compiló dentro de
`--workspace --all-targets`. No se afirma nada sobre sus 4 tests.

Deltas medidos: `relic_core --lib` 9 → **30** tests; `relic_cli` 58 → **62** tests y
10 → **11** targets. Los 2 rojos de latencia siguen siendo exactamente los de la línea
base; no se relajó ningún umbral.

---

## 10. Deuda registrada (no corregida a propósito)

1. **Superficie pública adelantada a Fase 3.** `mod.rs` expone `dual`, `program`,
   `classify`, `lexer`, `rpn_eval` como `pub`, más **17** nombres nuevos re-exportados
   (4 históricos intactos). El diseño
   reservaba esto para la Fase 5. Se adelantó porque el gate de equivalencia necesita
   `compile`/`eval` desde `relic_cli`. Es una desviación deliberada, no un accidente; la
   Fase 5 deberá revisar esta superficie en bloque en lugar de crearla.
2. **`ProgramClass::has_transcendental` está mal nombrado.** Marca también exponentes
   literales no enteros (`x^2.5`), que son algebraicos pero no polinómicos. El nombre
   correcto sería `not_polynomial`. No se renombra para no romper el contrato recién
   expuesto; queda como deuda de nomenclatura.
3. **`RpnProgram` no deriva `PartialEq` ni `Debug`.** Los tests comparan `.err()`. Añadir
   `PartialEq` a 584 B es barato pero no se necesitó; se difiere hasta que exista un
   consumidor real.
4. **Sin notación científica en el lexer.** `1e10` se tokeniza como `1` seguido de `e10`
   y produce `InvalidChar`. Es una limitación explícita, no un bug silencioso.
5. **`RpnTrace` no existe.** No hay registro de qué instrucción produjo qué abstención;
   el `EvalFault` solo identifica la causa. Necesario antes de exponer el motor al
   usuario final.
6. **`arithmetic/mod.rs` ya no es el `mod.rs` de 13 líneas de Fase 1.** Creció a 30 por
   los re-exports. Sigue por debajo de 180.

---

## 11. Estado de partida para la Fase 4

`classify.rs` entrega exactamente lo que el solver necesita: `is_linear()` e
`is_quadratic()` sobre una cota **sintáctica** de grado, con `UNKNOWN_DEGREE = u16::MAX`
significando "no demostrable", nunca "grado medido". La Fase 4 puede por tanto:

- resolver lineal/cuadrático por fórmula cerrada cuando `is_linear`/`is_quadratic`;
- usar Newton solo cuando la clasificación no concluya;
- abstenerse cuando `has_transcendental` o `var_in_denominator` impidan garantizar
  convergencia.

Nada de eso está implementado todavía. Este documento no anticipa métricas de la Fase 4.

---

> **Actualización (Fase 4):** implementado en `docs/AUDITORIA_FASE_4_SOLVER.md`.
> Este documento se conserva como registro del estado al cerrar la Fase 3.
