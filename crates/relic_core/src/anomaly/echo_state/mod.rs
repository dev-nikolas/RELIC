//! # EchoState Module
//!
//! Submódulo cohesivo del predictor recurrente ESN, normalización rodante y Takens embedding.

pub mod types;
pub mod norm;
pub mod embedding;
pub mod predictor;

pub use types::{EMBED_DIM, NORM_WINDOW, RESERVOIR_SIZE, RLS_DELTA, RLS_FORGET};
pub use norm::RollingNorm;
pub use embedding::DelayEmbedding;
pub use predictor::EchoStatePredictor;

