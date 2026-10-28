//! Pruebas de integración del protocolo MCP stdio (revisión 2025-03-26). Menos de 180 líneas.

use relic_mcp::McpServer;
use serde_json::{json, Value};

#[test]
fn test_mcp_handshake_initialize() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","clientInfo":{"name":"test-client","version":"1.0"}}}"#;
    let resp: Value = serde_json::from_str(&server.handle_line(req).expect("Debe responder")).unwrap();
    assert_eq!((&resp["jsonrpc"], &resp["id"], &resp["result"]["protocolVersion"], &resp["result"]["serverInfo"]["name"]), (&json!("2.0"), &json!(1), &json!("2025-03-26"), &json!("relic-mcp")));
    assert!(server.is_initialized);
}

#[test]
fn test_mcp_tools_list() {
    let mut server = McpServer::new();
    let resp: Value = serde_json::from_str(&server.handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap()).unwrap();
    let tools = resp["result"]["tools"].as_array().expect("Lista de herramientas");
    assert_eq!(tools.len(), 12);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in [
        "relic_interpret", "relic_verify", "relic_retrieve", "relic_propose", "relic_rollback",
        "relic_consolidate", "relic_activate", "relic_diff", "relic_spectral",
        "relic_synchronize", "relic_assimilate", "relic_capsule",
    ] {
        assert!(names.contains(&expected));
    }
}

#[test]
fn test_mcp_tool_interpret_valid_arithmetic() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"relic_interpret","arguments":{"text":"tengo 10 monedas y gasto 3"}}}"#;
    let resp: Value = serde_json::from_str(&server.handle_line(req).unwrap()).unwrap();
    let content = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("Entendí:") && content.contains("Estado: VALID") && content.contains("7"));
}

#[test]
fn test_mcp_tool_interpret_incomplete_epistemic() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"relic_interpret","arguments":{"text":"compré varias cosas y perdí algunas"}}}"#;
    let resp: Value = serde_json::from_str(&server.handle_line(req).unwrap()).unwrap();
    let content = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("Estado: INCOMPLETE") || content.contains("Estado: UNSUPPORTED"));
}

#[test]
fn test_mcp_tool_verify_valid_and_invalid() {
    let mut server = McpServer::new();
    let valid_rir = json!({
        "schema_version": 1, "intent": "inventory_update", "domain": "inventory", "status": "VALID",
        "initial_quantity": 20.0, "operations": [{ "operator": "add", "operand": 5.0 }],
        "uncertainty": { "is_rejected": false, "status": "VALID", "missing_fields": [], "rejection_reason": null }
    });
    let req_ok = json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "relic_verify", "arguments": { "rir": valid_rir } } }).to_string();
    let resp_ok: Value = serde_json::from_str(&server.handle_line(&req_ok).unwrap()).unwrap();
    assert!(resp_ok["result"]["content"][0]["text"].as_str().unwrap().contains("Verificación: VÁLIDA"));

    let invalid_rir = json!({
        "schema_version": 1, "intent": "inventory_update", "domain": "inventory", "status": "VALID",
        "initial_quantity": 10.0, "operations": [{ "operator": "divide", "operand": 0.0 }],
        "uncertainty": { "is_rejected": false, "status": "VALID", "missing_fields": [], "rejection_reason": null }
    });
    let req_err = json!({ "jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": { "name": "relic_verify", "arguments": { "rir": invalid_rir } } }).to_string();
    let resp_err: Value = serde_json::from_str(&server.handle_line(&req_err).unwrap()).unwrap();
    assert!(resp_err["result"]["content"][0]["text"].as_str().unwrap().contains("Verificación: RECHAZADA"));
}

#[test]
fn test_mcp_tool_retrieve_and_errors() {
    let mut server = McpServer::new();
    let outcome = server.blackwall.propose_mutation(relic_lexicon::LexiconPayload::graph_fact("Bogotá", "ubicada_en", "Colombia"), 1000);
    assert!(matches!(outcome, relic_blackwall::TransactionOutcome::Committed { .. }));

    let req_ret = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"relic_retrieve","arguments":{"entity":"Bogotá"}}}"#;
    let resp_ret: Value = serde_json::from_str(&server.handle_line(req_ret).unwrap()).unwrap();
    let text = resp_ret["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Colombia") && text.contains("Estado: VALID"));

    let resp_unk: Value = serde_json::from_str(&server.handle_line(r#"{"jsonrpc":"2.0","id":8,"method":"unknown_method"}"#).unwrap()).unwrap();
    assert_eq!(resp_unk["error"]["code"], -32601);

    let resp_parse: Value = serde_json::from_str(&server.handle_line("invalid json { {").unwrap()).unwrap();
    assert_eq!(resp_parse["error"]["code"], -32700);

    let resp_inv: Value = serde_json::from_str(&server.handle_line(r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{}}"#).unwrap()).unwrap();
    assert_eq!(resp_inv["error"]["code"], -32602);

    let resp_tool: Value = serde_json::from_str(&server.handle_line(r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"non_existent_tool"}}"#).unwrap()).unwrap();
    assert_eq!(resp_tool["error"]["code"], -32602);
}

#[test]
fn test_mcp_giant_input_rejected() {
    let mut server = McpServer::new();
    let resp: Value = serde_json::from_str(&server.handle_line(&"a".repeat(20_000)).unwrap()).unwrap();
    assert_eq!(resp["error"]["code"], -32600);
    assert!(resp["error"]["message"].as_str().unwrap().contains("16 KB"));
}

#[test]
fn test_mcp_query_timeout_rejected() {
    let mut server = McpServer::with_timeout(std::time::Duration::ZERO);
    let req = r#"{"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"name":"relic_interpret","arguments":{"text":"suma 2 y 2"}}}"#;
    let resp: Value = serde_json::from_str(&server.handle_line(req).unwrap()).unwrap();
    assert_eq!(resp["error"]["code"], -32000);
    assert!(resp["error"]["message"].as_str().unwrap().contains("timeout"));
}

#[test]
fn test_mcp_blackwall_tools_end_to_end() {
    let mut server = McpServer::new();

    let req_prop = json!({
        "jsonrpc": "2.0", "id": 20, "method": "tools/call",
        "params": { "name": "relic_propose", "arguments": { "payload": { "kind": "op_alias", "alias_word": "embolatan", "target_op": "Subtract" }, "timestamp_ms": 100 } }
    }).to_string();
    let resp_prop: Value = serde_json::from_str(&server.handle_line(&req_prop).unwrap()).unwrap();
    assert!(resp_prop["result"]["content"][0]["text"].as_str().unwrap().contains("Committed"));

    let req_diff = json!({ "jsonrpc": "2.0", "id": 21, "method": "tools/call", "params": { "name": "relic_diff", "arguments": { "v_from": 13, "v_to": 14 } } }).to_string();
    let resp_diff: Value = serde_json::from_str(&server.handle_line(&req_diff).unwrap()).unwrap();
    assert!(resp_diff["result"]["content"][0]["text"].as_str().unwrap().contains("flowchart LR"));

    let req_act = json!({ "jsonrpc": "2.0", "id": 22, "method": "tools/call", "params": { "name": "relic_activate", "arguments": { "seeds": ["Madrid"], "gamma": 0.8, "steps": 1 } } }).to_string();
    let resp_act: Value = serde_json::from_str(&server.handle_line(&req_act).unwrap()).unwrap();
    assert!(resp_act["result"]["content"][0]["text"].as_str().unwrap().contains("Ambiguous"));

    let req_rb = json!({ "jsonrpc": "2.0", "id": 23, "method": "tools/call", "params": { "name": "relic_rollback", "arguments": { "target_version": 13, "reason": "test" } } }).to_string();
    let resp_rb: Value = serde_json::from_str(&server.handle_line(&req_rb).unwrap()).unwrap();
    assert!(resp_rb["result"]["content"][0]["text"].as_str().unwrap().contains("RolledBack"));

    let req_spec = json!({
        "jsonrpc": "2.0", "id": 24, "method": "tools/call",
        "params": { "name": "relic_spectral", "arguments": { "signal": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0], "min_cycles": 3.0 } }
    }).to_string();
    let resp_spec: Value = serde_json::from_str(&server.handle_line(&req_spec).unwrap()).unwrap();
    assert!(resp_spec["result"]["content"][0]["text"].as_str().unwrap().contains("PeriodicBeaconDetected"));
}
