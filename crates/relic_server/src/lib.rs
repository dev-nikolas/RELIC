//! # relic_server
//!
//! Servidor HTTP nativo ultraliviano y servicio de inferencia sin estado para RELIC.
//! Menos de 180 líneas.

pub mod engine;
pub mod generate;
pub mod router;
pub mod state;
pub mod types;

pub use engine::execute_query;
pub use router::handle_request;
pub use state::ServerState;
