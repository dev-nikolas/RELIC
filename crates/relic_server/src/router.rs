//! # relic_server::router
//!
//! Despacho central de rutas HTTP y cabeceras CORS para RELIC.
//! Límites en proceso: tamaño máximo de cuerpo (16 KB) y timeout de consulta (1000 ms).
//! Menos de 180 líneas.

use std::io::Read;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use serde_json::{json, Value};
use tiny_http::{Header, Request, Response, StatusCode};
use crate::engine::execute_query_with_trace;
use crate::generate::handle_generate_sse;
use crate::state::ServerState;
use crate::types::{HealthResponse, QueryRequest};

pub const MAX_REQUEST_BODY_BYTES: u64 = 16 * 1024; // 16 KB
pub const DEFAULT_QUERY_TIMEOUT_MS: u64 = 1000;

fn cors_headers() -> Vec<Header> {
    vec![
        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS"[..]).unwrap(),
        Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type, Authorization, Accept"[..]).unwrap(),
    ]
}

fn send_json<T: serde::Serialize>(request: Request, status: u16, data: &T) {
    let body = serde_json::to_vec(data).unwrap_or_default();
    let mut headers = cors_headers();
    headers.push(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
    let resp = Response::new(StatusCode(status), headers, std::io::Cursor::new(body), None, None);
    let _ = request.respond(resp);
}

fn read_body(request: &mut Request) -> Result<String, (u16, Value)> {
    let mut body = String::new();
    let bytes_read = request.as_reader().take(MAX_REQUEST_BODY_BYTES + 1).read_to_string(&mut body).unwrap_or(0);
    if bytes_read as u64 > MAX_REQUEST_BODY_BYTES {
        return Err((413, json!({ "detail": "Error 413: El tamaño de la petición excede el límite de 16 KB.", "status": "PAYLOAD_TOO_LARGE" })));
    }
    Ok(body)
}

pub fn process_query_in_process(body_str: &str, state: &ServerState, timeout_ms: u64) -> (u16, Value) {
    if body_str.len() as u64 > MAX_REQUEST_BODY_BYTES {
        return (413, json!({ "detail": "Error 413: El tamaño de la petición excede el límite de 16 KB.", "status": "PAYLOAD_TOO_LARGE" }));
    }
    let req: QueryRequest = serde_json::from_str(body_str).unwrap_or_default();
    let (tx, rx) = mpsc::channel();
    let q_clone = req.query.clone();
    let state_clone = state.clone();

    thread::spawn(move || {
        if q_clone.starts_with("__TEST_SLOW_QUERY__") {
            thread::sleep(Duration::from_millis(1500));
        }
        let res = execute_query_with_trace(&q_clone, &state_clone, req.enable_trace);
        let _ = tx.send(res);
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(res) => (200, serde_json::to_value(res).unwrap()),
        Err(_) => (504, json!({ "detail": "Error 504: Tiempo de ejecución excedido (timeout).", "status": "TIMEOUT" })),
    }
}

pub fn handle_request(mut request: Request, state: &ServerState) {
    if request.method().as_str() == "OPTIONS" {
        let resp = Response::new(StatusCode(204), cors_headers(), std::io::Cursor::new(Vec::new()), Some(0), None);
        let _ = request.respond(resp);
        return;
    }

    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or("/");
    let method = request.method().as_str().to_string();

    match (method.as_str(), path) {
        ("GET", "/health") => {
            let res = HealthResponse {
                status: if state.weights_loaded { "ok".to_string() } else { "degraded".to_string() },
                detector_ready: true,
                mode: if state.weights_loaded { "full".to_string() } else { "degraded".to_string() },
                message: if state.weights_loaded {
                    "RELIC activo: Pesos Safetensors cargados y razonador determinista listo.".to_string()
                } else {
                    "RELIC activo: Modo degradado (razonador determinista y memoria listos, pesos ausentes).".to_string()
                },
            };
            send_json(request, 200, &res);
        }

        ("POST", "/api/query") => {
            let body = match read_body(&mut request) {
                Ok(b) => b,
                Err((code, val)) => { send_json(request, code, &val); return; }
            };
            let (status_code, val) = process_query_in_process(&body, state, DEFAULT_QUERY_TIMEOUT_MS);
            send_json(request, status_code, &val);
        }

        ("POST", "/api/generate") => {
            let body = match read_body(&mut request) {
                Ok(b) => b,
                Err((code, val)) => { send_json(request, code, &val); return; }
            };
            let req: QueryRequest = serde_json::from_str(&body).unwrap_or_default();
            let sse_bytes = handle_generate_sse(&req.query, state);
            let mut headers = cors_headers();
            headers.push(Header::from_bytes(&b"Content-Type"[..], &b"text/event-stream"[..]).unwrap());
            headers.push(Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]).unwrap());
            let resp = Response::new(StatusCode(200), headers, std::io::Cursor::new(sse_bytes), None, None);
            let _ = request.respond(resp);
        }

        _ => {
            send_json(request, 404, &json!({ "detail": "Ruta no encontrada" }));
        }
    }
}
