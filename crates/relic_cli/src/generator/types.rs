//! Tipos de datos para el generador curricular determinista.
use serde::{Deserialize, Serialize};
use relic_core::RirDocumentV1;

/// Split del dataset: entrenamiento o evaluación retenida.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSplit {
    Train,
    HoldoutTemplate,
}

/// Nivel curricular.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurriculumLevel {
    L01DirectArithmetic,
    L02InventoryUnits,
    L03EpistemicRejection,
}

/// Registro individual del dataset curado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumSample {
    pub id: String,
    pub seed: u64,
    pub level: CurriculumLevel,
    pub split: DatasetSplit,
    pub template_id: String,
    pub prompt: String,
    pub gold_rir: RirDocumentV1,
    pub expected_status: String,
    pub oracle_answer: String,
}
