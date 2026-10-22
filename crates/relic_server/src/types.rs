//! # relic_server::types
//!
//! Estructuras de datos serializables para la API HTTP de RELIC y Simple Domain.
//! Menos de 180 líneas.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub detector_ready: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoResponse {
    pub service_version: String,
    pub package_version: String,
    pub schema_version: String,
    pub model_status: String,
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_configuration: Option<serde_json::Value>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<ChatMessage>,
    pub archive_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConversationCreate {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageCreate {
    pub role: Option<String>,
    pub content: String,
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
