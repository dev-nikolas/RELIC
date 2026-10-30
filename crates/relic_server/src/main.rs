//! # relic_server
//!
//! Servicio HTTP de inferencia sin estado para RELIC.
//! Diagnóstico de arranque y traza de ejecución detallada.
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
    let num_workers = 4;

    println!("============================================================");
    println!("  RELIC HTTP Server — Servicio de Inferencia y Razonamiento");
    println!("============================================================");
    println!("  [INIT] Puerto de escucha:      http://127.0.0.1:{}", port);
    println!("  [INIT] Modo Operativo:         {}", if state.weights_loaded { "COMPLETO (Pesos Safetensors Activos)" } else { "DEGRADADO SEGURO (Motor Determinista RIR)" });
    println!("  [INIT] Motor Determinista:     ACTIVO (Aritmética, Grafos BFS, RIR AST)");
    println!("  [INIT] Memoria Episódica:      CARGADA (Axiomas relacionales S1 inicializados)");
    println!("  [INIT] Tokenizador BPE:        {}", if state.tokenizer.is_some() { "CARGADO (Vocabulario activo)" } else { "NO REQUERIDO (Modo RIR Simbólico)" });
    println!("  [INIT] Workers concurrentes:   {} hilos de despacho HTTP", num_workers);
    println!("  [INIT] Endpoints Disponibles:");
    println!("         * GET  /health           -> Diagnóstico de salud y procedencia");
    println!("         * POST /api/query        -> Inferencia unificada RIR con traza de 5 etapas");
    println!("         * POST /api/generate     -> Generación causal con streaming SSE");
    println!("============================================================");
    println!("  [RELIC READY] Servidor operativo a la espera de peticiones.");
    println!("============================================================");

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
