//! # TelemetryAssessment & Channel Types
//!
//! Estructuras de datos para el reporte de inferencia y diagnóstico de telemetría.

use crate::anomaly::echo_state::RESERVOIR_SIZE;

/// Diagnóstico de telemetría emitido por un canal de Seis Ojos.
#[derive(Clone, Debug)]
pub struct TelemetryAssessment {
    pub raw_residual: f64,
    pub z_value: f64,
    pub ready: bool,
    pub h_candidate: [f64; RESERVOIR_SIZE],
    pub pred_z: f64,
    pub cal_score: f64,
    pub is_peak: bool,
    pub nms_score: f64,
    pub score_nab: f64,
    pub score_relic: f64,
    pub range_violation: bool,
    pub drift_alarm: bool,
    pub is_refractory_suppressed: bool,
    pub topological_residual: f64,
    pub spatial_divergence: f64,
    pub martingale_alarm: bool,
    pub martingale_log_l: f64,
}

