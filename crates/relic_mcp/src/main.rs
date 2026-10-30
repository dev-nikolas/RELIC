//! Punto de entrada stdio para el servidor MCP de RELIC.
//! Lee peticiones JSON-RPC 2.0 por stdin línea a línea y emite respuestas por stdout.
use std::io::{self, BufRead, Write};
use relic_mcp::McpServer;

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut server = McpServer::new();

    for line_result in stdin.lock().lines() {
        match line_result {
            Ok(line) => {
                if let Some(resp) = server.handle_line(&line) {
                    let _ = writeln!(stdout, "{}", resp);
                    let _ = stdout.flush();
                }
            }
            Err(e) => {
                eprintln!("[relic_mcp] Error leyendo stdin: {e}");
                break;
            }
        }
    }
}
