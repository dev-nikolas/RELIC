//! Submódulos de pruebas de paridad numérica, expulsión de ventanas y deriva CUSUM.

pub mod cusum_and_stability;
pub mod eviction_and_long_horizon;
pub mod level1_components;
pub mod level2_channel;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GoldenStep {
    pub t: usize,
    pub raw_val: f64,
    #[allow(dead_code)]
    pub timestamp: String,
    pub z: f64,
    pub ready: bool,
    pub pred_z: f64,
    pub err: f64,
    pub cal_score: f64,
    pub is_peak: bool,
    pub nms_score: f64,
    pub is_spatial: bool,
    pub score_nab: f64,
    #[allow(dead_code)]
    pub score_relic: f64,
    #[allow(dead_code)]
    pub h_first4: Vec<f64>,
    #[allow(dead_code)]
    pub wout_first4: Vec<f64>,
    #[allow(dead_code)]
    pub p_diag_first4: Vec<f64>,
}

pub fn load_500_golden_steps() -> Vec<GoldenStep> {
    let json_bytes = include_bytes!("../../src/anomaly/data/golden_stream_sample500.json");
    serde_json::from_slice(json_bytes).expect("Error al deserializar golden_stream_sample500.json")
}

pub fn load_1000_golden_steps() -> Vec<GoldenStep> {
    let json_bytes = include_bytes!("../../src/anomaly/data/golden_stream_1000.json");
    serde_json::from_slice(json_bytes).expect("Error al deserializar golden_stream_1000.json")
}

