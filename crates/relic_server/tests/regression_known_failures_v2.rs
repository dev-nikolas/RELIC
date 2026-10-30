//! Pruebas de regresión con criterio corregido (v2): el banco known_failures_v2.json
//! re-define el oro del caso Bogotá→Tokio como abstención epistémica correcta.
//! A diferencia de v1 (ledger suave), esta suite exige que TODOS los casos pasen:
//! si el pipeline retrocede hacia una respuesta inválida o un falso acepte, muere aquí.
use std::fs;
use std::path::Path;
use serde::Deserialize;
use relic_server::engine::execute_query;
use relic_server::state::ServerState;

#[derive(Debug, Deserialize)]
struct KnownFailureItem {
    input: String,
    expected_output: String,
    expected_route: String,
    #[allow(dead_code)]
    date: String,
    #[allow(dead_code)]
    description: String,
}

#[test]
fn test_known_failures_v2_gold_corregido_debe_pasar_completo() {
    let manifest_path = Path::new("../../tests/known_failures_v2.json");
    let content = match fs::read_to_string(manifest_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("[SKIP] No se encontró tests/known_failures_v2.json desde {}", manifest_path.display());
            return;
        }
    };

    let items: Vec<KnownFailureItem> =
        serde_json::from_str(&content).expect("Debe deserializar known_failures_v2.json");
    assert!(!items.is_empty(), "El banco v2 no puede estar vacío");
    let state = ServerState::new();

    let mut fallos: Vec<String> = Vec::new();
    for item in &items {
        let res = execute_query(&item.input, &state);
        let output_ok = res.text.to_lowercase().contains(&item.expected_output.to_lowercase());
        let route_ok = res.route == item.expected_route;
        if !(output_ok && route_ok) {
            fallos.push(format!(
                "input='{}' | ruta={} (esperada={}) | texto='{}' (aguja='{}')",
                item.input, res.route, item.expected_route, res.text, item.expected_output
            ));
        }
    }

    assert!(
        fallos.is_empty(),
        "El oro corregido v2 exige 100%; fallos:\n{}",
        fallos.join("\n")
    );
}
