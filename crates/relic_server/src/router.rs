//! # relic_server::router
//!
//! Despacho central de rutas HTTP, cabeceras CORS y adaptadores de API.
//! Menos de 180 líneas.

use std::io::Read;
use serde_json::json;
use tiny_http::{Header, Request, Response, StatusCode};
use crate::conversations::{add_message_and_respond, create_conversation, get_conversation, list_conversations};
use crate::engine::execute_query;
use crate::generate::handle_generate_sse;
use crate::state::ServerState;
use crate::static_files::try_serve_static;
use crate::types::{ConversationCreate, HealthResponse, InfoResponse, MessageCreate, QueryRequest};

fn cors_headers() -> Vec<Header> {
    vec![
        Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, PATCH, DELETE, OPTIONS"[..]).unwrap(),
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
                message: if state.weights_loaded {
                    "RELIC activo: Pesos Safetensors cargados y razonador determinista listo.".to_string()
                } else {
                    "RELIC activo: Modo degradado (razonador determinista y memoria listos, pesos ausentes).".to_string()
                },
            };
            send_json(request, 200, &res);
        }

        ("GET", "/v1/info") => {
            let res = InfoResponse {
                service_version: "0.1.0".to_string(),
                package_version: "0.1.0".to_string(),
                schema_version: "v1".to_string(),
                model_status: if state.weights_loaded { "ready".to_string() } else { "degraded".to_string() },
                capabilities: vec!["deterministic_arithmetic".to_string(), "graph_reasoning".to_string(), "rir_v1".to_string()],
                model_configuration: None,
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

        ("GET", "/v1/conversations") => {
            let list = list_conversations(state);
            send_json(request, 200, &list);
        }

        ("POST", "/v1/conversations") => {
            let body = read_body(&mut request);
            let req: ConversationCreate = serde_json::from_str(&body).unwrap_or(ConversationCreate { title: None });
            let conv = create_conversation(req.title, state);
            send_json(request, 201, &conv);
        }

        ("GET", p) if p.starts_with("/v1/conversations/") => {
            let id = p.trim_start_matches("/v1/conversations/").split('/').next().unwrap_or("");
            if let Some(conv) = get_conversation(id, state) {
                send_json(request, 200, &conv);
            } else {
                send_json(request, 404, &json!({ "detail": "Conversación no encontrada" }));
            }
        }

        ("POST", p) if p.starts_with("/v1/conversations/") && p.ends_with("/messages") => {
            let id = p.trim_start_matches("/v1/conversations/").trim_end_matches("/messages");
            let body = read_body(&mut request);
            let req: MessageCreate = serde_json::from_str(&body).unwrap_or(MessageCreate { role: None, content: String::new() });
            match add_message_and_respond(id, &req.content, state) {
                Ok(msg) => send_json(request, 201, &msg),
                Err(err) => send_json(request, 404, &json!({ "detail": err })),
            }
        }

        ("GET", "/v1/data-sources") | ("GET", "/v1/patterns") | ("GET", "/v1/research/experiments") | ("GET", "/v1/archive") => {
            send_json(request, 200, &json!([]));
        }

        ("GET", "/v1/settings") => {
            send_json(request, 200, &json!({ "preferences": {} }));
        }

        _ => {
            if let Some(ref dir) = state.static_dir {
                if let Some((bytes, ct)) = try_serve_static(dir, path) {
                    let mut headers = cors_headers();
                    headers.push(Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap());
                    let resp = Response::new(StatusCode(200), headers, std::io::Cursor::new(bytes), None, None);
                    let _ = request.respond(resp);
                    return;
                }
            }
            send_json(request, 404, &json!({ "detail": "Ruta no encontrada" }));
        }
    }
}
