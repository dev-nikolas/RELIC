//! Servidor JSON-RPC 2.0 y gestor del ciclo de vida MCP stdio (2025-03-26).
//! Estado protegido por `BlackwallGate`: cero escrituras directas sin verificación.
use std::time::{Duration, Instant};
use relic_blackwall::BlackwallGate;
use serde_json::{json, Value};
use crate::tools::{
    handle_activate, handle_assimilate, handle_capsule, handle_consolidate, handle_diff,
    handle_interpret, handle_propose, handle_retrieve, handle_rollback, handle_spectral,
    handle_synchronize, handle_verify, list_tools, VALID_MCP_TOOLS,
};
use crate::types::*;

pub struct McpServer {
    pub blackwall: BlackwallGate,
    pub is_initialized: bool,
    pub timeout: Duration,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            blackwall: BlackwallGate::with_genesis(),
            is_initialized: false,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            blackwall: BlackwallGate::with_genesis(),
            is_initialized: false,
            timeout,
        }
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

                if !VALID_MCP_TOOLS.contains(&tool_name) {
                    return Some(JsonRpcResponse::error(id, INVALID_PARAMS, format!("Herramienta no encontrada: '{tool_name}'")));
                }

                if self.timeout == Duration::ZERO {
                    return Some(JsonRpcResponse::error(
                        id,
                        SERVER_TIMEOUT,
                        format!("Tiempo de ejecución excedido (timeout de {} ms)", self.timeout.as_millis()),
                    ));
                }

                let t0 = Instant::now();
                let res = match tool_name {
                    "relic_interpret" => handle_interpret(&args, &mut self.blackwall),
                    "relic_verify" => handle_verify(&args),
                    "relic_retrieve" => handle_retrieve(&args, &mut self.blackwall),
                    "relic_propose" => handle_propose(&args, &mut self.blackwall),
                    "relic_rollback" => handle_rollback(&args, &mut self.blackwall),
                    "relic_consolidate" => handle_consolidate(&args, &mut self.blackwall),
                    "relic_activate" => handle_activate(&args, &self.blackwall),
                    "relic_diff" => handle_diff(&args, &self.blackwall),
                    "relic_spectral" => handle_spectral(&args, &self.blackwall),
                    "relic_synchronize" => handle_synchronize(&args, &self.blackwall),
                    "relic_assimilate" => handle_assimilate(&args, &mut self.blackwall),
                    "relic_capsule" => handle_capsule(&args, &mut self.blackwall),
                    _ => unreachable!(),
                };

                if t0.elapsed() > self.timeout {
                    return Some(JsonRpcResponse::error(
                        id,
                        SERVER_TIMEOUT,
                        format!("Tiempo de ejecución excedido (timeout de {} ms)", self.timeout.as_millis()),
                    ));
                }

                Some(JsonRpcResponse::success(id, serde_json::to_value(res).unwrap_or(Value::Null)))
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
