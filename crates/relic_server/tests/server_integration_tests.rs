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
    let res_unsupported = execute_query("Explica brevemente la teoría del universo", &state);
    assert_eq!(res_unsupported.status, "UNSUPPORTED");
    assert_eq!(res_unsupported.route, "unsupported");

    let res_gen = execute_query("genera un cuento sobre estrellas", &state);
    assert!(res_gen.status == "GENERATED" || res_gen.status == "UNVERIFIED");
    assert!(res_gen.route == "neural_model" || res_gen.route == "degraded_mode");
}

#[test]
fn test_handle_generate_sse_output() {
    let state = ServerState::new();
    let sse_bytes = handle_generate_sse("1 + 1", &state);
    let sse_str = String::from_utf8_lossy(&sse_bytes);

    assert!(sse_str.contains("data: "));
    assert!(sse_str.contains("\"finished\":true"));
}

#[test]
fn test_server_giant_input_rejected() {
    let state = ServerState::new();
    let giant_query = "x".repeat(20_000); // 20 KB > 16 KB
    let payload = format!(r#"{{"query":"{}"}}"#, giant_query);
    let (code, val) = relic_server::process_query_in_process(&payload, &state, 1000);

    assert_eq!(code, 413);
    assert_eq!(val["status"], "PAYLOAD_TOO_LARGE");
    assert!(val["detail"].as_str().unwrap().contains("16 KB"));
}

#[test]
fn test_server_query_timeout_rejected() {
    let state = ServerState::new();
    let payload = r#"{"query":"1 + 1"}"#;
    // Timeout estricto de 0 ms
    let (code, val) = relic_server::process_query_in_process(payload, &state, 0);

    assert_eq!(code, 504);
    assert_eq!(val["status"], "TIMEOUT");
    assert!(val["detail"].as_str().unwrap().contains("timeout"));
}

#[test]
fn test_server_query_with_trace() {
    let state = ServerState::new();
    let payload = r#"{"query":"10 más 5","enable_trace":true}"#;
    let (code, val) = relic_server::process_query_in_process(payload, &state, 1000);

    assert_eq!(code, 200);
    assert_eq!(val["status"], "VALID");
    let trace = val["trace"].as_array().expect("Debe retornar lista de etapas de traza");
    assert_eq!(trace.len(), 5);
    let stage_names: Vec<&str> = trace.iter().map(|s| s["stage"].as_str().unwrap()).collect();
    assert_eq!(stage_names, vec!["normalizacion", "intencion", "rir", "validacion", "ejecucion"]);
}

#[test]
fn test_server_blackwall_end_to_end_endpoints() {
    let state = ServerState::new();

    // 1. Proponer alias "embolatan" -> Subtract
    let prop_body = r#"{"payload":{"kind":"op_alias","alias_word":"embolatan","target_op":"Subtract"},"timestamp_ms":1000}"#;
    let (code_p, val_p) = relic_server::process_blackwall_route("/api/blackwall/propose", prop_body, &state);
    assert_eq!(code_p, 200);
    assert_eq!(val_p["current_version"], 14);

    // 2. Consultar con el alias aprendido
    let res = execute_query("Si tengo 50 y me embolatan 15, ¿cuánto queda?", &state);
    assert_eq!(res.status, "VALID");
    assert!(res.text.contains("50 - 15 = 35"));

    // 3. Diff visual entre v13 y v14
    let diff_body = r#"{"v_from":13,"v_to":14}"#;
    let (code_d, val_d) = relic_server::process_blackwall_route("/api/blackwall/diff", diff_body, &state);
    assert_eq!(code_d, 200);
    assert!(val_d["mermaid"].as_str().unwrap().contains("embolatan"));

    // 4. Activación propagada
    let act_body = r#"{"seeds":["Madrid"],"gamma":0.8,"steps":1,"max_entropy_threshold":1.5}"#;
    let (code_a, val_a) = relic_server::process_blackwall_route("/api/blackwall/activate", act_body, &state);
    assert_eq!(code_a, 200);
    assert!(val_a["shannon_entropy_bits"].as_f64().unwrap() > 0.9);

    // 5. Rollback a v13
    let rb_body = r#"{"target_version":13,"reason":"Test rollback","timestamp_ms":2000}"#;
    let (code_r, val_r) = relic_server::process_blackwall_route("/api/blackwall/rollback", rb_body, &state);
    assert_eq!(code_r, 200);
    assert_eq!(val_r["current_version"], 15);
}


