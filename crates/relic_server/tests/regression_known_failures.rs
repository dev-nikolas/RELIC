//! Pruebas de regresión automatizadas sobre el banco tests/known_failures.json.
//! Menos de 180 líneas.

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
    date: String,
    description: String,
}

#[test]
fn test_regression_known_failures_dataset() {
    let manifest_path = Path::new("../../tests/known_failures.json");
    if !manifest_path.exists() {
        eprintln!("[SKIP] No se encontró tests/known_failures.json en {}", manifest_path.display());
        return;
    }

    let content = fs::read_to_string(manifest_path).expect("Debe leer known_failures.json");
    let items: Vec<KnownFailureItem> = serde_json::from_str(&content).expect("Debe deserializar JSON");
    let state = ServerState::new();

    let mut passed = 0;
    let mut total = 0;

    println!("\n============================================================");
    println!("  RELIC — Evaluación de Casos Históricos y Known Failures");
    println!("============================================================");

    for item in &items {
        total += 1;
        let res = execute_query(&item.input, &state);
        let output_matches = res.text.to_lowercase().contains(&item.expected_output.to_lowercase());
        let route_matches = res.route == item.expected_route;

        let ok = output_matches && route_matches;
        if ok { passed += 1; }

        println!("[{}] Input: \"{}\"", if ok { "PASSED" } else { "KNOWN_FAIL" }, item.input);
        println!("       Route: {} (Esperada: {})", res.route, item.expected_route);
        println!("       Text:  {} (Esperaba contenga: \"{}\")", res.text, item.expected_output);
        println!("       Ref:   {} | {}\n", item.date, item.description);
    }

    println!("Total evaluados: {} | Pasaron: {} | Fallos conocidos pendientes de generalización: {}",
        total, passed, total - passed);
    println!("============================================================\n");
}
