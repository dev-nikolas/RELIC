//! # relic_server::router
//!
//! Despacho central de rutas HTTP y cabeceras CORS para RELIC.
//! Menos de 180 líneas.

use std::io::Read;
use serde_json::json;
use tiny_http::{Header, Request, Response, StatusCode};
use crate::engine::execute_query;
use crate::generate::handle_generate_sse;
use crate::state::ServerState;
use crate::types::{HealthResponse, QueryRequest};

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

fn read_body(request: &mut Request) -> String {
    let mut body = String::new();
    let _ = request.as_reader().take(65536).read_to_string(&mut body);
    body
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
            let body = read_body(&mut request);
            let req: QueryRequest = serde_json::from_str(&body).unwrap_or(QueryRequest { query: String::new() });
            let res = execute_query(&req.query, state);
            send_json(request, 200, &res);
        }

        ("POST", "/api/generate") => {
            let body = read_body(&mut request);
            let req: QueryRequest = serde_json::from_str(&body).unwrap_or(QueryRequest { query: String::new() });
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
