//! Pruebas de integración del protocolo MCP stdio (revisión 2025-03-26).
//! Menos de 180 líneas.

use relic_mcp::McpServer;
use serde_json::{json, Value};

#[test]
fn test_mcp_handshake_initialize() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","clientInfo":{"name":"test-client","version":"1.0"}}}"#;
    let resp_str = server.handle_line(req).expect("Debe responder");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(resp["result"]["serverInfo"]["name"], "relic-mcp");
    assert!(server.is_initialized);
}

#[test]
fn test_mcp_tools_list() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    let resp_str = server.handle_line(req).expect("Debe responder");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    let tools = resp["result"]["tools"].as_array().expect("Lista de herramientas");
    assert_eq!(tools.len(), 3);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"relic_interpret"));
    assert!(names.contains(&"relic_verify"));
    assert!(names.contains(&"relic_retrieve"));
}

#[test]
fn test_mcp_tool_interpret_valid_arithmetic() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"relic_interpret","arguments":{"text":"tengo 10 monedas y gasto 3"}}}"#;
    let resp_str = server.handle_line(req).expect("Debe responder");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    let content = &resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("Entendí:"));
    assert!(content.contains("Estado: VALID"));
    assert!(content.contains("7"));
}

#[test]
fn test_mcp_tool_interpret_incomplete_epistemic() {
    let mut server = McpServer::new();
    let req = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"relic_interpret","arguments":{"text":"compré varias cosas y perdí algunas"}}}"#;
    let resp_str = server.handle_line(req).expect("Debe responder");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    let content = &resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("Estado: INCOMPLETE") || content.contains("Estado: UNSUPPORTED"));
}

#[test]
fn test_mcp_tool_verify_valid_and_invalid() {
    let mut server = McpServer::new();
    // 1. RIR Válido
    let valid_rir = json!({
        "schema_version": 1,
        "intent": "inventory_update",
        "domain": "inventory",
        "status": "VALID",
        "initial_quantity": 20.0,
        "operations": [{ "operator": "add", "operand": 5.0 }],
        "uncertainty": { "is_rejected": false, "status": "VALID", "missing_fields": [], "rejection_reason": null }
    });
    let req_ok = json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": { "name": "relic_verify", "arguments": { "rir": valid_rir } }
    }).to_string();
    let resp_ok: Value = serde_json::from_str(&server.handle_line(&req_ok).unwrap()).unwrap();
    assert!(resp_ok["result"]["content"][0]["text"].as_str().unwrap().contains("Verificación: VÁLIDA"));

    // 2. RIR Inválido (División por cero rechazada)
    let invalid_rir = json!({
        "schema_version": 1,
        "intent": "inventory_update",
        "domain": "inventory",
        "status": "VALID",
        "initial_quantity": 10.0,
        "operations": [{ "operator": "divide", "operand": 0.0 }],
        "uncertainty": { "is_rejected": false, "status": "VALID", "missing_fields": [], "rejection_reason": null }
    });
    let req_err = json!({
        "jsonrpc": "2.0", "id": 6, "method": "tools/call",
        "params": { "name": "relic_verify", "arguments": { "rir": invalid_rir } }
    }).to_string();
    let resp_err: Value = serde_json::from_str(&server.handle_line(&req_err).unwrap()).unwrap();
    assert!(resp_err["result"]["content"][0]["text"].as_str().unwrap().contains("Verificación: RECHAZADA"));
}

#[test]
fn test_mcp_tool_retrieve_and_errors() {
    let mut server = McpServer::new();
    // Insertar un hecho en memoria episódica
    server.memory.insert_edge("Bogotá", "Colombia");

    let req_ret = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"relic_retrieve","arguments":{"entity":"Bogotá"}}}"#;
    let resp_ret: Value = serde_json::from_str(&server.handle_line(req_ret).unwrap()).unwrap();
    let text = resp_ret["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Colombia"));
    assert!(text.contains("Estado: VALID"));

    // Error 1: Método desconocido (-32601)
    let req_unknown = r#"{"jsonrpc":"2.0","id":8,"method":"unknown_method"}"#;
    let resp_unk: Value = serde_json::from_str(&server.handle_line(req_unknown).unwrap()).unwrap();
    assert_eq!(resp_unk["error"]["code"], -32601);

    // Error 2: Parse error (-32700)
    let resp_parse: Value = serde_json::from_str(&server.handle_line("invalid json { {").unwrap()).unwrap();
    assert_eq!(resp_parse["error"]["code"], -32700);

    // Error 3: Parámetros inválidos (-32602) por falta de name
    let req_inv_params = r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{}}"#;
    let resp_inv: Value = serde_json::from_str(&server.handle_line(req_inv_params).unwrap()).unwrap();
    assert_eq!(resp_inv["error"]["code"], -32602);

    // Error 4: Herramienta desconocida (-32602 según spec de MCP tools/call)
    let req_unk_tool = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"non_existent_tool"}}"#;
    let resp_tool: Value = serde_json::from_str(&server.handle_line(req_unk_tool).unwrap()).unwrap();
    assert_eq!(resp_tool["error"]["code"], -32602);
}

#[test]
fn test_mcp_giant_input_rejected() {
    let mut server = McpServer::new();
    let giant_payload = "a".repeat(20_000); // 20 KB > 16 KB
    let resp_str = server.handle_line(&giant_payload).expect("Debe retornar respuesta de error");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["error"]["code"], -32600);
    assert!(resp["error"]["message"].as_str().unwrap().contains("16 KB"));
}

#[test]
fn test_mcp_query_timeout_rejected() {
    let mut server = McpServer::with_timeout(std::time::Duration::from_millis(100));
    let req = r#"{"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"name":"relic_interpret","arguments":{"text":"__TEST_SLOW_QUERY__ suma 2 y 2"}}}"#;
    let resp_str = server.handle_line(req).expect("Debe retornar error de timeout");
    let resp: Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["error"]["code"], -32000);
    assert!(resp["error"]["message"].as_str().unwrap().contains("timeout"));
}

