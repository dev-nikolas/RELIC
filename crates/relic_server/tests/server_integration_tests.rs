//! Pruebas de integración de motor y endpoints de relic_server sin estado.
//! Menos de 180 líneas.

use relic_server::engine::execute_query;
use relic_server::generate::handle_generate_sse;
use relic_server::state::ServerState;

#[test]
fn test_execute_query_simple_arithmetic() {
    let state = ServerState::new();
    let res = execute_query("1 + 1", &state);

    assert_eq!(res.status, "VALID");
    assert_eq!(res.route, "deterministic_arithmetic");
    assert!(res.text.contains("2"), "La respuesta debe contener 2, recibido: {}", res.text);
    assert!(res.latency_ms >= 0.0);
    assert!(res.rss_mb > 0.0);
}

#[test]
fn test_execute_query_graph_reachability() {
    let state = ServerState::new();
    let res = execute_query("¿Depende el crate relic_cli del paquete memmap2?", &state);

    assert_eq!(res.status, "VALID");
    assert_eq!(res.route, "deterministic_graph");
    assert!(res.text.contains("memmap2") || res.text.contains("relic_cli"));
}

#[test]
fn test_execute_query_incomplete_epistemic_rejection() {
    let state = ServerState::new();
    let res = execute_query("tengo algunas manzanas y luego compro 5", &state);

    assert_eq!(res.status, "INCOMPLETE");
    assert_eq!(res.route, "epistemic_rejection");
    assert!(res.text.contains("desconoce la cantidad inicial") || res.text.contains("incompleta"));
}

#[test]
fn test_execute_query_empty_rejection() {
    let state = ServerState::new();
    let res = execute_query("   ", &state);

    assert_eq!(res.status, "INVALID");
    assert_eq!(res.route, "validation_error");
}

#[test]
fn test_execute_query_free_language_degraded_or_neural() {
    let state = ServerState::new();
    let res = execute_query("Explica brevemente la teoría del universo", &state);

    assert_eq!(res.status, "VALID");
    assert!(res.route == "neural_model" || res.route == "degraded_mode");
}

#[test]
fn test_handle_generate_sse_output() {
    let state = ServerState::new();
    let sse_bytes = handle_generate_sse("1 + 1", &state);
    let sse_str = String::from_utf8_lossy(&sse_bytes);

    assert!(sse_str.contains("data: "));
    assert!(sse_str.contains("\"finished\":true"));
}
