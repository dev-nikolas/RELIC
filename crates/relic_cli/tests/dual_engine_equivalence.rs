//! Gate de Fase 3: el motor dual debe reproducir **bit a bit** el resultado
//! del Shunting-Yard existente en `relic_cli::math` para todo el corpus que
//! ese parser acepta hoy.

use relic_cli::math::{evaluate_math_tokens_with_reductions, tokenize_math};
use relic_core::arithmetic::{compile, eval, DualNumber};

const CORPUS: &[&str] = &[
    "1 + 2",
    "2 + 3 * 4",
    "(2 + 3) * 4",
    "10 - 4 / 2",
    "100 / 8",
    "2.5 * 4",
    ".5 + .25",
    "7 / 3",
    "1 + 2 + 3 + 4",
    "((1 + 2) * (3 + 4)) / 5",
    "-5 + 3",
    "2 - -3",
    "3 * (-2)",
    "8 / 2 / 2",
    "2 - 3 - 4",
    "1.5 * 2.5 + 0.25",
    "1000000 * 1000000",
    "0.1 + 0.2",
    "1 / 7",
    "(1 + 2) * 3 - 4 / 2",
    "9 - 3 * 2 + 1",
    "(9 - 3) * (2 + 1)",
    "5 * 5 * 5",
    "12.345 + 67.89",
    "1 - 0.9",
    "0.3 * 3",
    "7 - (2 + 3)",
    "(7 - 2) + 3",
    "0 - 0",
    "1 + 0",
];

fn cli_value(expr: &str) -> f64 {
    let tokens = tokenize_math(expr).unwrap_or_else(|| panic!("CLI no tokeniza {expr}"));
    evaluate_math_tokens_with_reductions(&tokens)
        .unwrap_or_else(|| panic!("CLI no evalúa {expr}"))
        .unwrap_or_else(|op| panic!("CLI error {op:?} en {expr}"))
        .0
}

fn dual_value(expr: &str) -> f64 {
    let program = compile(expr).unwrap_or_else(|e| panic!("lexer nuevo rechaza {expr}: {e:?}"));
    eval(&program, DualNumber::constant(0.0))
        .unwrap_or_else(|e| panic!("evaluador nuevo rechaza {expr}: {e:?}"))
        .real
}

#[test]
fn dual_engine_is_bit_identical_to_shunting_yard() {
    for expr in CORPUS {
        let cli = cli_value(expr);
        let new = dual_value(expr);
        assert_eq!(
            cli.to_bits(),
            new.to_bits(),
            "divergencia de bits en `{expr}`: CLI={cli:?} nuevo={new:?}"
        );
    }
}

#[test]
fn corpus_is_not_trivial() {
    // El corpus debe ejercitar valores no enteros y distintos órdenes de magnitud.
    assert!(CORPUS.iter().any(|e| cli_value(e).fract() != 0.0));
    assert!(CORPUS.iter().any(|e| cli_value(e) > 1e6));
}

#[test]
fn documented_divergence_variable_vs_multiplication() {
    // El lexer del CLI interpreta `x` como multiplicación (legado lingüístico).
    // El motor diferencial la interpreta como la incógnita: no hay ambigüedad
    // silenciosa, el resultado es un rechazo explícito.
    let tokens = tokenize_math("2 x 3").expect("el CLI acepta `x` como producto");
    let cli = evaluate_math_tokens_with_reductions(&tokens).unwrap().unwrap().0;
    assert_eq!(cli, 6.0);
    assert!(compile("2 x 3").is_err());
    assert!(compile("2*x").is_ok());
}

#[test]
fn documented_divergence_division_by_zero() {
    // El CLI devuelve un error de operador; el motor dual una abstención formal.
    let tokens = tokenize_math("1 / 0").unwrap();
    let cli = evaluate_math_tokens_with_reductions(&tokens).unwrap();
    assert!(cli.is_err());

    let program = compile("1 / 0").unwrap();
    let new = eval(&program, DualNumber::constant(0.0));
    assert!(new.is_err());
}
