//! # relic_server
//!
//! Servicio HTTP de inferencia sin estado para RELIC.
//! Menos de 180 líneas.

use std::sync::Arc;
use std::thread;
use tiny_http::Server;
use relic_server::{handle_request, ServerState};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port = if args.len() > 1 {
        args[1].parse::<u16>().unwrap_or(5000)
    } else {
        std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(5000)
    };

    let bind_addr = format!("0.0.0.0:{}", port);
    let server = match Server::http(&bind_addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[ERROR] No se pudo enlazar servidor en {}: {}", bind_addr, e);
            std::process::exit(1);
        }
    };

    let state = Arc::new(ServerState::new());
    let server = Arc::new(server);

    println!("============================================================");
    println!("  RELIC HTTP Server — Servicio de Inferencia Sin Estado");
    println!("============================================================");
    println!("  Escuchando en: http://127.0.0.1:{}", port);
    println!("  Modo:          {}", if state.weights_loaded { "Completo (Pesos Cargados)" } else { "Degradado (Motor Determinista Activo)" });
    println!("  Endpoints:");
    println!("    * GET  /health           (Diagnóstico de salud)");
    println!("    * POST /api/query        (Inferencia unificada RELIC)");
    println!("    * POST /api/generate     (Streaming SSE token a token)");
    println!("============================================================");

    let num_workers = 4;
    let mut handles = Vec::new();

    for worker_id in 0..num_workers {
        let server_ref = Arc::clone(&server);
        let state_ref = Arc::clone(&state);

        let handle = thread::spawn(move || {
            loop {
                match server_ref.recv() {
                    Ok(request) => {
                        handle_request(request, &state_ref);
                    }
                    Err(e) => {
                        eprintln!("[Worker {}] Error al recibir petición: {}", worker_id, e);
                        break;
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }
}
