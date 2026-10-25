//! Servidor JSON-RPC 2.0 y gestor del ciclo de vida MCP stdio (2025-03-26).
//! Menos de 180 líneas.

use relic_core::EpisodicMemory;
use serde_json::{json, Value};
use crate::tools::{handle_interpret, handle_retrieve, handle_verify, list_tools};
use crate::types::*;

pub struct McpServer {
    pub memory: EpisodicMemory,
    pub is_initialized: bool,
}

impl McpServer {
    pub fn new() -> Self {
        Self { memory: EpisodicMemory::new(), is_initialized: false }
    }

    pub fn handle_request(&mut self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = req.id;
        match req.method.as_str() {
            "initialize" => {
                self.is_initialized = true;
                let result = json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "relic-mcp",
                        "version": "0.1.0"
                    }
                });
                Some(JsonRpcResponse::success(id, result))
            }
            "notifications/initialized" | "initialized" => None,
            "tools/list" => {
                let tools = list_tools();
                let result = json!({ "tools": tools });
                Some(JsonRpcResponse::success(id, result))
            }
            "tools/call" => {
                let params = req.params.unwrap_or(Value::Null);
                let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => return Some(JsonRpcResponse::error(id, INVALID_PARAMS, "Falta 'name' en params.")),
                };
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let tool_result = match tool_name {
                    "relic_interpret" => handle_interpret(&args, &mut self.memory),
                    "relic_verify" => handle_verify(&args),
                    "relic_retrieve" => handle_retrieve(&args, &mut self.memory),
                    other => {
                        return Some(JsonRpcResponse::error(
                            id,
                            METHOD_NOT_FOUND,
                            format!("Herramienta no encontrada: '{other}'"),
                        ))
                    }
                };

                Some(JsonRpcResponse::success(id, serde_json::to_value(tool_result).unwrap()))
            }
            "ping" => Some(JsonRpcResponse::success(id, json!({}))),
            other => Some(JsonRpcResponse::error(
                id,
                METHOD_NOT_FOUND,
                format!("Método MCP no reconocido: '{other}'"),
            )),
        }
    }

    pub fn handle_line(&mut self, line: &str) -> Option<String> {
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
