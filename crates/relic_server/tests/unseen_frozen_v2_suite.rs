//! Suite congelada v2 "no vista" (tests/unseen_frozen_v2.json): banco de 16 casos jamás
//! usados para iterar el código, ejecutado AUTOMÁTICAMENTE por el pipeline completo
//! (despacho -> RIR -> motor determinista) con guarda de no-regresión sobre la línea
//! base registrada. Menos de 180 líneas.

use std::fs;
use std::path::Path;
use serde::Deserialize;
use relic_server::engine::execute_query;
use relic_server::state::ServerState;

#[derive(Debug, Deserialize)]
struct FrozenCase {
    input: String,
    expected_output: String,
    expected_route: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct FrozenManifest {
    frozen: String,
    note: String,
    baseline_accuracy_pct: f64,
    cases: Vec<FrozenCase>,
}

#[test]
fn test_unseen_frozen_v2_suite() {
    let manifest_path = Path::new("../../tests/unseen_frozen_v2.json");
    if !manifest_path.exists() {
        panic!("[FAIL] Suite congelada ausente: {}", manifest_path.display());
    }
    let content = fs::read_to_string(manifest_path).expect("Debe leer unseen_frozen_v2.json");
    let manifest: FrozenManifest = serde_json::from_str(&content).expect("JSON de suite congelada inválido");
    assert!(!manifest.cases.is_empty(), "El banco congelado no puede estar vacío");

    let state = ServerState::new();
    let mut passed = 0;
    let mut rows: Vec<String> = Vec::new();

    for case in &manifest.cases {
        let res = execute_query(&case.input, &state);
        let ok = res.route == case.expected_route
            && res.text.to_lowercase().contains(&case.expected_output.to_lowercase());
        if ok { passed += 1; }
        rows.push(format!(
            "[{}] Input: \"{}\" | Route: {}/{} | Texto: \"{}\" (esperaba \"{}\")\n     Motivo: {}",
            if ok { "PASSED" } else { "FAILED" }, case.input, res.route, case.expected_route, res.text, case.expected_output, case.description
        ));
    }

    let accuracy = 100.0 * passed as f64 / manifest.cases.len() as f64;
    println!("\n============================================================");
    println!("  RELIC — Suite congelada NO VISTA v2 (congelada: {})", manifest.frozen);
    println!("============================================================");
    for row in &rows { println!("{}", row); }
    println!("Casos pasados: {}/{} | Precisión: {:.1} % | Baseline congelada: {:.1} %",
        passed, manifest.cases.len(), accuracy, manifest.baseline_accuracy_pct);
    println!("Protocolo: {}", manifest.note);
    println!("============================================================\n");

    // Guarda de no-regresión: precisión SIEMPRE >= baseline congelada (nunca hacia atrás).
    assert!(
        accuracy >= manifest.baseline_accuracy_pct,
        "REGRESIÓN en suite congelada v2: {:.1} % < baseline {:.1} % (ver output arriba). Si es una generalización intencional, actualizar la baseline y el número de casos.",
        accuracy, manifest.baseline_accuracy_pct
    );
    assert_eq!(passed, manifest.cases.len(),
        "Todo el banco congelado debe pasar: revisar casos [FAILED] arriba antes de tocar la baseline.");
}
