//! # relic_server
//!
//! Servidor HTTP nativo ultraliviano y adaptadores de API para Simple Domain y RELIC.
//! Menos de 180 líneas.

pub mod conversations;
pub mod engine;
pub mod generate;
pub mod router;
pub mod state;
pub mod static_files;
pub mod types;

pub use engine::execute_query;
pub use router::handle_request;
pub use state::ServerState;
