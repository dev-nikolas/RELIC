//! # relic_server::router
use std::io::Read;
use std::time::{Duration, Instant};
use relic_lexicon::{to_dot_diff, to_mermaid_diff};
use serde_json::{json, Value};
use tiny_http::{Header, Request, Response, StatusCode};
use crate::engine::execute_query_with_trace;
use crate::generate::handle_generate_sse;
use crate::state::ServerState;
use crate::types::{
    ActivateRequest, AssimilateRequest, ConsolidateRequest, DiffRequest, HealthResponse,
    ImportCapsuleRequest, ProposeMutationRequest, QueryRequest, RollbackRequest, SpectralRequest,
    SynchronizeRequest,
};

pub const MAX_REQUEST_BODY_BYTES: u64 = 16 * 1024;
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
    let _ = request.respond(Response::new(StatusCode(status), headers, std::io::Cursor::new(body), None, None));
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
    if timeout_ms == 0 {
        return (504, json!({ "detail": "Error 504: Tiempo de ejecución excedido (timeout).", "status": "TIMEOUT" }));
    }
    let req: QueryRequest = serde_json::from_str(body_str).unwrap_or_default();
    let t0 = Instant::now();
    let res = execute_query_with_trace(&req.query, state, req.enable_trace);
    if t0.elapsed() > Duration::from_millis(timeout_ms) {
        return (504, json!({ "detail": "Error 504: Tiempo de ejecución excedido (timeout).", "status": "TIMEOUT" }));
    }
    (200, serde_json::to_value(res).unwrap_or_else(|_| json!({ "status": "ERROR" })))
}

pub fn process_blackwall_route(path: &str, body_str: &str, state: &ServerState) -> (u16, Value) {
    if body_str.len() as u64 > MAX_REQUEST_BODY_BYTES {
        return (413, json!({ "detail": "Error 413: El tamaño de la petición excede el límite de 16 KB.", "status": "PAYLOAD_TOO_LARGE" }));
    }
    match path {
        "/api/blackwall/propose" => {
            let Ok(req) = serde_json::from_str::<ProposeMutationRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para ProposeMutationRequest" })); };
            let mut gate = state.blackwall_write();
            let outcome = gate.propose_mutation(req.payload, req.timestamp_ms);
            (200, json!({ "outcome": outcome, "current_version": gate.current_version(), "head_hash": gate.head_hash() }))
        }
        "/api/blackwall/rollback" => {
            let Ok(req) = serde_json::from_str::<RollbackRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para RollbackRequest" })); };
            let mut gate = state.blackwall_write();
            let outcome = gate.rollback_to(req.target_version, req.reason, req.timestamp_ms);
            (200, json!({ "outcome": outcome, "current_version": gate.current_version(), "head_hash": gate.head_hash() }))
        }
        "/api/blackwall/consolidate" => {
            let Ok(req) = serde_json::from_str::<ConsolidateRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para ConsolidateRequest" })); };
            let mut gate = state.blackwall_write();
            let outcome = gate.consolidate_memory(req.min_weight_threshold, req.min_fidelity, req.timestamp_ms);
            (200, json!({ "outcome": outcome, "current_version": gate.current_version(), "head_hash": gate.head_hash() }))
        }
        "/api/blackwall/activate" => {
            let Ok(req) = serde_json::from_str::<ActivateRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para ActivateRequest" })); };
            let seed_refs: Vec<&str> = req.seeds.iter().map(|s| s.as_str()).collect();
            let gate = state.blackwall_read();
            let res = gate.propagate_activation(&seed_refs, req.gamma, req.steps, req.max_entropy_threshold);
            (200, serde_json::to_value(res).unwrap_or_else(|_| json!({ "status": "ERROR" })))
        }
        "/api/blackwall/diff" => {
            let Ok(req) = serde_json::from_str::<DiffRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para DiffRequest" })); };
            let gate = state.blackwall_read();
            match gate.diff_versions(req.v_from, req.v_to) {
                Ok(diff) => (200, json!({ "diff": diff, "mermaid": to_mermaid_diff(&diff), "dot": to_dot_diff(&diff) })),
                Err(e) => (400, json!({ "detail": e.to_string(), "status": "INTEGRITY_ERROR" })),
            }
        }
        "/api/blackwall/spectral" => {
            let Ok(req) = serde_json::from_str::<SpectralRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para SpectralRequest" })); };
            let gate = state.blackwall_read();
            if let Some(ref alert_spec) = req.network_alert {
                (200, serde_json::to_value(gate.verify_network_alert(alert_spec)).unwrap_or_else(|_| json!({ "status": "ERROR" })))
            } else {
                let outcome = gate.analyze_spectrum(&req.signal, req.support_epsilon, req.min_cycles, req.expected_period_hint);
                (200, serde_json::to_value(outcome).unwrap_or_else(|_| json!({ "status": "ERROR" })))
            }
        }
        "/api/blackwall/synchronize" => {
            let Ok(req) = serde_json::from_str::<SynchronizeRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para SynchronizeRequest" })); };
            let gate = state.blackwall_read();
            (200, serde_json::to_value(gate.synchronize_kuramoto(&req.spec)).unwrap_or_else(|_| json!({ "status": "ERROR" })))
        }
        "/api/blackwall/assimilate" => {
            let Ok(req) = serde_json::from_str::<AssimilateRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para AssimilateRequest" })); };
            let mut gate = state.blackwall_write();
            let rep = gate.assimilate_verified_example(&req.raw_query, req.verified_op, req.timestamp_ms);
            (200, serde_json::to_value(rep).unwrap_or_else(|_| json!({ "status": "ERROR" })))
        }
        "/api/blackwall/capsule/import" => {
            let Ok(req) = serde_json::from_str::<ImportCapsuleRequest>(body_str) else { return (400, json!({ "detail": "JSON inválido para ImportCapsuleRequest" })); };
            let mut gate = state.blackwall_write();
            let outcome = gate.import_engram_capsule(&req.capsule, req.timestamp_ms);
            (200, json!({ "outcome": outcome, "current_version": gate.current_version(), "head_hash": gate.head_hash() }))
        }
        _ => (404, json!({ "detail": "Ruta no encontrada" })),
    }
}

pub fn handle_request(mut request: Request, state: &ServerState) {
    if request.method().as_str() == "OPTIONS" {
        let _ = request.respond(Response::new(StatusCode(204), cors_headers(), std::io::Cursor::new(Vec::new()), Some(0), None));
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
        ("POST", "/api/query") => match read_body(&mut request) {
            Ok(body) => { let (code, val) = process_query_in_process(&body, state, DEFAULT_QUERY_TIMEOUT_MS); send_json(request, code, &val); }
            Err((code, val)) => send_json(request, code, &val),
        },
        ("POST", "/api/generate") => match read_body(&mut request) {
            Ok(body) => {
                let req: QueryRequest = serde_json::from_str(&body).unwrap_or_default();
                let sse_bytes = handle_generate_sse(&req.query, state);
                let mut headers = cors_headers();
                headers.push(Header::from_bytes(&b"Content-Type"[..], &b"text/event-stream"[..]).unwrap());
                headers.push(Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]).unwrap());
                let _ = request.respond(Response::new(StatusCode(200), headers, std::io::Cursor::new(sse_bytes), None, None));
            }
            Err((code, val)) => send_json(request, code, &val),
        },
        ("POST", p) if p.starts_with("/api/blackwall/") => match read_body(&mut request) {
            Ok(body) => { let (code, val) = process_blackwall_route(p, &body, state); send_json(request, code, &val); }
            Err((code, val)) => send_json(request, code, &val),
        },
        _ => send_json(request, 404, &json!({ "detail": "Ruta no encontrada" })),
    }
}
