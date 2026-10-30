//! # relic_server
//!
//! Servidor HTTP nativo ultraliviano y servicio de inferencia sin estado para RELIC.
pub mod engine;
pub mod generate;
pub mod router;
pub mod state;
pub mod types;

pub use engine::execute_query;
pub use router::{handle_request, process_blackwall_route, process_query_in_process};
pub use state::ServerState;
