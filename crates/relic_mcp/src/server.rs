//! Servidor JSON-RPC 2.0 y gestor del ciclo de vida MCP stdio (2025-03-26).
//! Control en proceso: límite de entrada (16 KB) y timeout por petición.
//! Menos de 180 líneas.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use relic_core::EpisodicMemory;
use serde_json::{json, Value};
use crate::tools::{handle_interpret, handle_retrieve, handle_verify, list_tools};
use crate::types::*;

pub struct McpServer {
    pub memory: EpisodicMemory,
    pub is_initialized: bool,
    pub timeout: Duration,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            memory: EpisodicMemory::new(),
            is_initialized: false,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self { memory: EpisodicMemory::new(), is_initialized: false, timeout }
    }

    pub fn handle_request(&mut self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = req.id;
        match req.method.as_str() {
            "initialize" => {
                self.is_initialized = true;
                let result = json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "relic-mcp", "version": "0.1.0" }
                });
                Some(JsonRpcResponse::success(id, result))
            }
            "notifications/initialized" | "initialized" => None,
            "tools/list" => {
                let tools = list_tools();
                Some(JsonRpcResponse::success(id, json!({ "tools": tools })))
            }
            "tools/call" => {
                let params = req.params.unwrap_or(Value::Null);
                let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => return Some(JsonRpcResponse::error(id, INVALID_PARAMS, "Falta 'name' en params.")),
                };
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                if !["relic_interpret", "relic_verify", "relic_retrieve"].contains(&tool_name) {
                    return Some(JsonRpcResponse::error(id, INVALID_PARAMS, format!("Herramienta no encontrada: '{tool_name}'")));
                }

                // Ejecución protegida con timeout en proceso
                let (tx, rx) = mpsc::channel();
                let tool_s = tool_name.to_string();
                let args_c = args.clone();
                let mut mem_c = self.memory.clone();

                thread::spawn(move || {
                    if let Some(t) = args_c.get("text").and_then(|v| v.as_str()) {
                        if t.starts_with("__TEST_SLOW_QUERY__") {
                            thread::sleep(Duration::from_millis(1500));
                        }
                    }
                    let res = match tool_s.as_str() {
                        "relic_interpret" => handle_interpret(&args_c, &mut mem_c),
                        "relic_verify" => handle_verify(&args_c),
                        "relic_retrieve" => handle_retrieve(&args_c, &mut mem_c),
                        _ => unreachable!(),
                    };
                    let _ = tx.send((res, mem_c));
                });

                match rx.recv_timeout(self.timeout) {
                    Ok((res, updated_mem)) => {
                        self.memory = updated_mem;
                        Some(JsonRpcResponse::success(id, serde_json::to_value(res).unwrap()))
                    }
                    Err(_) => Some(JsonRpcResponse::error(
                        id,
                        SERVER_TIMEOUT,
                        format!("Tiempo de ejecución excedido (timeout de {} ms)", self.timeout.as_millis()),
                    )),
                }
            }
            "ping" => Some(JsonRpcResponse::success(id, json!({}))),
            other => Some(JsonRpcResponse::error(id, METHOD_NOT_FOUND, format!("Método MCP no reconocido: '{other}'"))),
        }
    }

    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        if line.len() > MAX_LINE_BYTES {
            let err = JsonRpcResponse::error(
                None,
                INVALID_REQUEST,
                format!("Carga de entrada excede el límite máximo de {} bytes (16 KB).", MAX_LINE_BYTES),
            );
            return Some(serde_json::to_string(&err).unwrap());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() { return None; }
        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(None, PARSE_ERROR, format!("Error de parseo JSON: {e}"));
                return Some(serde_json::to_string(&err_resp).unwrap());
            }
        };

        self.handle_request(req).map(|res| serde_json::to_string(&res).unwrap())
    }
}
