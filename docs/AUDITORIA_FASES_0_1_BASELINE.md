# Auditoría Fase 0 + Fase 1 — Línea base y extracción de `arithmetic`

**Fecha:** 2026-10-30
**Commit de partida:** `ebbfd2c` (`refactor: implement core neuro-symbolic engine with anomaly detection and tactical dispatch`)
**Toolchain:** `rustc 1.99.1` / `cargo 1.99.1`, perfil `dev` (debug), `resolver = "2"`

Este documento registra dos cosas distintas y no intercambiables:

1. **Fase 0** — el estado *medido* de la suite antes de tocar código. Sirve como línea base para detectar regresiones.
2. **Fase 1** — el renombrado puro `arithmetic.rs` → `arithmetic/{mod.rs, plan.rs}`, con 0 líneas de lógica modificadas.

> **Nota metodológica:** no se declara ninguna mejora de rendimiento. Los únicos números de este documento son conteos de tests y los valores de latencia que los propios tests imprimen cuando fallan, reproducidos tal cual para dejar constancia de que son **preexistentes**.

---

## 1. Línea base (Fase 0)

### 1.1 Resultado por crate

| Crate | Targets verdes | Targets fallidos | Tests verdes | Tests fallidos | Exit |
|---|---|---|---|---|---|
| `relic_core` | 21 | **2** | 96 | **2** | 1 |
| `relic_blackwall` | 10 | 0 | 34 | 0 | 0 |
| `relic_cli` | 10 | 0 | 58 | 0 | 0 |
| `relic_server` | 9 | 0 | 21 | 0 | 0 |
| `relic_lexicon` | 5 | 0 | 8 | 0 | 0 |
| `relic_mcp` | 4 | 0 | 9 | 0 | 0 |
| `relic_benchmarks` | 20 | 0 | 4 | 0 | 0 |

**Total: 230 tests verdes, 2 fallidos**, ambos en `relic_core` (los fallidos son los dos targets de latencia de 1.2, que suman 4 verdes + 2 rojos).

### 1.2 Fallos preexistentes (NO relacionados con esta refactorización)

Ambos son aserciones de **latencia** ejecutadas en perfil `debug`. Se registran con los valores que imprimieron en la corrida de línea base:

| Test | Archivo | Umbral | Valor medido en debug |
|---|---|---|---|
| `test_syndrome_latency_sub_50ns` | `crates/relic_core/tests/causal_syndrome_falsification.rs:84` | < 50 ns | 646.88 ns / 865.13 ns (dos corridas) |
| `test_lodf_simd_latency_sub_50ns` | `crates/relic_core/tests/kirchhoff_lodf_sensitivity_falsification.rs:131` | < 50 ns | 1091.29 ns |

Consecuencias operativas:

- La suite de `relic_core` **no está verde** en `master` antes de este trabajo. Cualquier comparación de regresión debe hacerse **por nombre de test**, no por código de salida global.
- Estos dos tests no ejercitan `arithmetic` ni `contingency_bfs`. No se tocaron ni se relajaron sus umbrales: hacerlo sería maquillar la línea base.
- No se afirma que pasen en `release`: no se ejecutó `--release` (haría falta para sostenerlo).

### 1.3 Inventario de dependencias de `ArithmeticOp` / `ArithmeticPlan`

Corrección al diseño previo: no son "12 puntos", son **~20 archivos de código fuente y ~12 archivos de test** los que consumen estos tipos. Clasificación por naturaleza del acoplamiento:

| Naturaleza | Archivos | Riesgo si cambia el tipo |
|---|---|---|
| Re-export público | `relic_core/src/lib.rs:44` | Ruptura de API de todo el workspace |
| `match` exhaustivo sin comodín | `relic_server/src/engine.rs:33-38`, `relic_cli/src/normalize.rs:121-126`, `relic_cli/src/answer.rs:46-49`, `relic_cli/src/inventory.rs:145-148`, `relic_lexicon/src/calibrator.rs:47-50` | **Añadir una variante rompe la compilación** en 5 puntos |
| Clave de `HashMap` / `BTreeMap` | `relic_lexicon/src/polysemy.rs:44,58`, `relic_lexicon/src/view.rs:26`, `relic_lexicon/src/log.rs` | Requiere `Hash + Eq + Copy`, ya derivados |
| Serialización (`Serialize`/`Deserialize`) | `relic_core/src/arithmetic/plan.rs:9` + payloads de `relic_lexicon` | Cambiar la representación rompe logs persistidos |
| Acceso a campos públicos (`steps`, `reductions`, `unit`) | `relic_server/src/engine.rs:32-41`, `relic_cli/src/dispatch.rs:55-61`, `relic_cli/src/inverted.rs:100`, `relic_server/tests/inverted_extractor_frozen_suite.rs:59` | Renombrar o reordenar campos rompe tests congelados |
| Cadenas exactas de `format_formula` | `relic_blackwall/tests/core_and_associative_tests.rs:13,23,30,37` | Cambiar el formato rompe el contrato lingüístico |
| Uso directo de `ArithmeticOp` como valor | `benchmarks/src/{ablation,epistemic,continual}.rs`, `relic_cli/examples/{eval_dump,bridge_dump}.rs`, `relic_mcp/src/tools_blackwall.rs:112`, `relic_server/src/types.rs:144` | Solo lectura; seguro |

**Conclusión vinculante para las fases siguientes:** `ArithmeticOp` **no se amplía**. Los operadores nuevos (`Pow`, `Sin`, `Ln`, …) viven en un enum separado (`DiffOp`) dentro de los nuevos submódulos.

---

## 2. Fase 1 — Renombrado puro

### 2.1 Cambios aplicados

```
R  crates/relic_core/src/arithmetic.rs -> crates/relic_core/src/arithmetic/plan.rs   (100%)
A  crates/relic_core/src/arithmetic/mod.rs                                          (+13)
```

- `plan.rs`: **0 líneas añadidas, 0 eliminadas**. `git diff -M --summary` reporta `rename ... (100%)`.
- `mod.rs` (nuevo, 13 líneas): declara `mod plan;` (privado) y re-exporta exactamente los 4 tipos históricos. No se añadió superficie pública nueva.
- `crates/relic_core/src/lib.rs:44` **sin tocar**: `pub use arithmetic::{ArithmeticError, ArithmeticOp, ArithmeticPlan, ArithmeticReduction};` sigue resolviendo, porque un módulo directorio expone los mismos nombres que el archivo anterior.
- Se preservaron los finales de línea CRLF del repositorio (`core.autocrlf = true`, sin `.gitattributes`).

### 2.2 Gates de aceptación

| Gate | Comando | Resultado |
|---|---|---|
| Renombrado 100 %, sin cambios de lógica | `git diff --cached -M --summary` | `rename ... (100%)` ✓ |
| Compila todo el workspace, incluidos tests, ejemplos y benches | `cargo check --workspace --all-targets` | exit 0 ✓ |
| Tests unitarios de `relic_core` (incluye `arithmetic::plan::tests`) | `cargo test -p relic_core --lib` | 9/9 ✓ |
| Regresión de `relic_core` | `cargo test -p relic_core --no-fail-fast` | **mismos 2 fallos, mismos nombres**, 96 verdes / 21 targets verdes ✓ |
| Consumidores del contrato | `cargo test -p relic_blackwall -p relic_cli -p relic_server -p relic_lexicon -p relic_mcp` | todos exit 0 ✓ |
| Límite de 180 líneas | `plan.rs` = 168, `mod.rs` = 13 | ✓ |

### 2.3 Deuda documental registrada (deliberadamente no corregida)

Los informes históricos citan el path antiguo y sus números de línea:

- `docs/AUDITORIA_FUNCIONAL.md` (líneas 12, 16, 19, 29, 47, 161, 174, 185, 353)
- `docs/DISENO_SANDBOX_MCP.md:49`
- `tools/audit_funcional/RESULTS.md`, `tools/audit_funcional/run_experiments.py:91`
- `reports/*.json` (artefactos generados, gitignored)

Los números de línea siguen siendo correctos porque el archivo se movió **verbatim**; solo cambió el path (`arithmetic.rs` → `arithmetic/plan.rs`). Reescribir un informe de auditoría histórica para "actualizar" una cita es falsificar el registro, así que no se hizo. Si se desea, se corrige en un commit aparte y explícito.

### 2.4 Estado de partida para la Fase 2

`crates/relic_core/src/arithmetic/` contiene únicamente `mod.rs` y `plan.rs`. El siguiente paso es `dual.rs` + `dual_transcendental.rs` con los `const _: () = assert!(size_of::<DualNumber>() == 16)` y la matriz de casos M7–M14 (dominio, polos, singularidades), sin tocar `plan.rs`.
