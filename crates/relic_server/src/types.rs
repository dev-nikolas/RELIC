//! # relic_server::types
//!
//! Estructuras de datos serializables para el servicio de inferencia sin estado RELIC.
//! Menos de 180 líneas.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub detector_ready: bool,
    pub mode: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryRequest {
    #[serde(alias = "text")]
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub text: String,
    pub route: String,
    pub status: String,
    pub latency_ms: f64,
    pub rss_mb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SseTokenChunk {
    pub token: String,
    pub token_id: u32,
    pub finished: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
}
