//! Suite congelada de etapa 4: extractor de cláusulas invertidas.
//! Lea tests/f6_inverted_oro_v1.json (congelado ANTES de la implementación).
//! ROJO antes del extractor, VERDE después SIN retroceso del resto.
use relic_cli::dispatch::parse_inventory_problem;
use relic_cli::normalize::normalize_query;
use serde_json::Value;

fn oro() -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/f6_inverted_oro_v1.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("leer oro etapa 4")).expect("json oro")
}

fn verifica(caso: &Value, tag: &str) {
    let texto = caso["text"].as_str().unwrap();
    let norm = normalize_query(texto);
    let plan = parse_inventory_problem(&norm).or_else(|| parse_inventory_problem(texto));
    match caso["expect"].as_str().unwrap() {
        "VALID" => {
            let p = plan.unwrap_or_else(|| panic!("[{}] sin plan: {}", tag, texto));
            assert!((p.initial - caso["initial"].as_f64().unwrap()).abs() < 1e-9,
                "[{}] initial {}: got {}", tag, texto, p.initial);
            let unit = p.unit.clone().unwrap_or_default();
            assert_eq!(unit, caso["entity"].as_str().unwrap(), "[{}] unit {}", tag, texto);
            let oro_ops = caso["ops"].as_array().unwrap();
            assert_eq!(p.steps.len(), oro_ops.len(), "[{}] #ops {}", tag, texto);
            for (s, g) in p.steps.iter().zip(oro_ops) {
                let signo = matches!(s.0, relic_core::ArithmeticOp::Add);
                let g_add = g["operator"] == "add";
                assert_eq!(signo, g_add, "[{}] signo {}", tag, texto);
                assert!((s.1 - g["operand"].as_f64().unwrap()).abs() < 1e-9,
                    "[{}] operando {}", tag, texto);
            }
        }
        "ABSTAIN" => assert!(plan.is_none(), "[{}] debe abstener: {}", tag, texto),
        o => panic!("expect desconocido {}: {}", tag, o),
    }
}

#[test]
fn etapa4_f6_frozen_25() {
    for c in oro()["f6_frozen"].as_array().unwrap() { verifica(c, "F6"); }
}

#[test]
fn etapa4_heldout_invertidas_genericas() {
    for c in oro()["heldout_invertidas"].as_array().unwrap() { verifica(c, "heldout"); }
}

#[test]
fn etapa4_casos_canonicos_estables() {
    // casos canonico preexistente deben seguir leyéndose igual (sin regresion)
    let p = parse_inventory_problem("Tengo 8 lápices y presté 2.");
    assert!(p.is_some(), "regresión: caso canonico dejo de parsear");
    let p = p.unwrap();
    assert_eq!(p.initial, 8.0);
    assert_eq!(p.unit.as_deref(), Some("lápices"));
    assert_eq!(p.steps.len(), 1);
    assert!(!matches!(p.steps[0].0, relic_core::ArithmeticOp::Add), "deberia ser resta");
}
