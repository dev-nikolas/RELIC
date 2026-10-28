//! # relic_server::types
//!
//! Estructuras de datos serializables para el servicio HTTP y los endpoints transaccionales del Blackwall.
//! Menos de 180 líneas.

use relic_core::NetworkAlertSpec;
use relic_lexicon::LexiconPayload;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub detector_ready: bool,
    pub mode: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStage {
    pub stage: String,
    pub latency_ms: f64,
    pub status: String,
    pub output: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_illustrative: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct QueryRequest {
    #[serde(alias = "text")]
    pub query: String,
    #[serde(default)]
    pub enable_trace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub text: String,
    pub route: String,
    pub status: String,
    pub latency_ms: f64,
    pub rss_mb: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<TraceStage>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SseTokenChunk {
    pub token: String,
    pub token_id: u32,
    #[serde(rename = "finished")]
    pub finished: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
}

/// Petición para proponer una mutación léxica o relacional al `BlackwallGate`.
#[derive(Debug, Clone, Deserialize)]
pub struct ProposeMutationRequest {
    pub payload: LexiconPayload,
    #[serde(default)]
    pub timestamp_ms: u64,
}

/// Petición para ejecutar un rollback append-only con cicatriz hacia `target_version`.
#[derive(Debug, Clone, Deserialize)]
pub struct RollbackRequest {
    pub target_version: u64,
    pub reason: String,
    #[serde(default)]
    pub timestamp_ms: u64,
}

/// Petición para consolidar memoria ("Sueño") podando episodios efímeros bajo garantía de fidelidad.
#[derive(Debug, Clone, Deserialize)]
pub struct ConsolidateRequest {
    pub min_weight_threshold: f64,
    #[serde(default = "default_min_fidelity")]
    pub min_fidelity: f64,
    #[serde(default)]
    pub timestamp_ms: u64,
}

fn default_min_fidelity() -> f64 {
    1.0
}

/// Petición para ejecutar activación propagada (`propagate_activation`) sobre la memoria asociativa.
#[derive(Debug, Clone, Deserialize)]
pub struct ActivateRequest {
    pub seeds: Vec<String>,
    #[serde(default = "default_gamma")]
    pub gamma: f64,
    #[serde(default = "default_steps")]
    pub steps: usize,
    #[serde(default = "default_entropy_threshold")]
    pub max_entropy_threshold: f64,
}

fn default_gamma() -> f64 {
    0.8
}
fn default_steps() -> usize {
    2
}
fn default_entropy_threshold() -> f64 {
    1.5
}

/// Petición para inspeccionar el diff estructurado y visual (Mermaid y DOT) entre dos versiones.
#[derive(Debug, Clone, Deserialize)]
pub struct DiffRequest {
    pub v_from: u64,
    pub v_to: u64,
}

/// Petición para análisis espectral (DFT + Donoho-Stark) o verificación completa de alerta C2 + BFS.
#[derive(Debug, Clone, Deserialize)]
pub struct SpectralRequest {
    #[serde(default)]
    pub signal: Vec<f64>,
    #[serde(default = "default_support_epsilon")]
    pub support_epsilon: f64,
    #[serde(default = "default_min_cycles")]
    pub min_cycles: f64,
    #[serde(default)]
    pub expected_period_hint: Option<f64>,
    #[serde(default)]
    pub network_alert: Option<NetworkAlertSpec>,
}

fn default_support_epsilon() -> f64 {
    0.05
}
fn default_min_cycles() -> f64 {
    3.0
}
