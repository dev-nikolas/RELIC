//! # relic_mcp
//!
//! Servidor MCP (Model Context Protocol) para RELIC sobre transporte stdio.
//! Especificación: 2025-03-26.
//! Menos de 180 líneas.

pub mod server;
pub mod tools;
pub mod tools_blackwall;
pub mod types;

pub use server::McpServer;
pub use tools::list_tools;
pub use types::*;
