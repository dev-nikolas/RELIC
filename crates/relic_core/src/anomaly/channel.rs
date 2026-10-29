//! Canal de Telemetría Completo Seis Ojos (SixEyesChannel).
//!
//! Integra la cadena completa:
//! Takens (d=16) -> RollingNorm (W=6400) -> ESN (N=64, ρ=0.9) ->
//! Full RLS (P: 64x64) -> EcdfWindow (W=16000) -> StreamingPeakDecay (K=24, τ=2.0) ->
//! SpatialSpikeBypass (warmup=100, tol=0.05) -> Refractory (w=80, th=0.990) ->
//! TelemetryDriftMonitor (CUSUM lateral).

use crate::anomaly::cusum::TelemetryDriftMonitor;
use crate::anomaly::ecdf::{EcdfWindow, MIN_HISTORIAL};
use crate::anomaly::echo_state::{EchoStatePredictor, EMBED_DIM, RESERVOIR_SIZE};
use crate::anomaly::peak_decay::StreamingPeakDecay;
use crate::anomaly::refractory::RefractorySuppressor;
use crate::anomaly::spatial_bypass::SpatialSpikeBypass;

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
}

#[derive(Clone)]
pub struct SixEyesChannel {
    pub esn: EchoStatePredictor,
    pub ecdf: EcdfWindow,
    pub decay: StreamingPeakDecay,
    pub spatial: SpatialSpikeBypass,
    pub refr: RefractorySuppressor,
    pub drift: TelemetryDriftMonitor,
}

impl SixEyesChannel {
    pub fn new() -> Self {
        Self {
            esn: EchoStatePredictor::new(),
            ecdf: EcdfWindow::new(),
            decay: StreamingPeakDecay::default_config(),
            spatial: SpatialSpikeBypass::default_config(),
            refr: RefractorySuppressor::default_config(),
            drift: TelemetryDriftMonitor::new(),
        }
    }

    pub fn reset(&mut self) {
        self.esn.reset();
        self.ecdf.reset();
        self.decay.reset();
        self.spatial.reset();
        self.refr.reset();
        self.drift.reset();
    }

    /// Inferencia pura e inmutable ($0\text{ mallocs}$).
    pub fn evaluate_pure(
        &self,
        value: f64,
        win: &[[f64; EMBED_DIM]; RESERVOIR_SIZE],
        wres: &[[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE],
    ) -> TelemetryAssessment {
        if !value.is_finite() {
            return TelemetryAssessment {
                raw_residual: 0.0,
                z_value: 0.0,
                ready: false,
                h_candidate: self.esn.h,
                pred_z: 0.0,
                cal_score: 0.0,
                is_peak: false,
                nms_score: 0.0,
                score_nab: 0.0,   // NAB omite NaNs
                score_relic: 1.0, // RELIC bloquea NaNs como violación de invariante
                range_violation: true,
                drift_alarm: false,
                is_refractory_suppressed: false,
            };
        }

        // 1. Chequeo de ruptura espacial
        let spatial_anomaly = self.spatial.check_anomaly(value);

        // 2. Inferencia en ESN (Takens + Reservorio + RLS)
        let (z, vec_opt, h_cand, pred_z, err) = self.esn.evaluate_pure(value, win, wres);
        let ready = vec_opt.is_some();

        // 3. Calibrador Percentil ECDF
        let cal_score = self.ecdf.score(err);

        // 4. Supresión de No-Máximos (NMS)
        let (is_peak, nms_score) = self.decay.evaluate_pure(cal_score);

        // Score crudo antes de refractario: ruptura espacial tiene precedencia
        let raw_detector_score = if spatial_anomaly { 1.0 } else { nms_score };

        // 5. Compuerta Refractaria para NAB
        let (is_suppressed, score_nab) = self.refr.evaluate_pure(raw_detector_score);

        // 6. Score para RELIC: evalúa la sorpresa dinámica pura y no suprime rupturas físicas
        let score_relic = if spatial_anomaly { 1.0 } else { raw_detector_score };

        // 7. Alarma lateral de deriva
        let drift_alarm = self.drift.is_alarm();

        TelemetryAssessment {
            raw_residual: err,
            z_value: z,
            ready,
            h_candidate: h_cand,
            pred_z,
            cal_score,
            is_peak,
            nms_score,
            score_nab,
            score_relic,
            range_violation: spatial_anomaly,
            drift_alarm,
            is_refractory_suppressed: is_suppressed,
        }
    }

    /// Actualización atómica de estado (sólo en confirmación de transacción).
    pub fn commit(&mut self, value: f64, assess: &TelemetryAssessment) {
        if !value.is_finite() {
            return;
        }

        // 1. Cotas espaciales
        self.spatial.commit(value);

        // 2. ESN y RLS
        self.esn.commit(
            value,
            assess.z_value,
            assess.ready,
            assess.h_candidate,
            assess.pred_z,
        );

        // 3. ECDF
        self.ecdf.push(assess.raw_residual);

        // 4. Peak Decay: almacena estrictamente cal_score (magnitud un-decayed)
        self.decay.commit(assess.cal_score, assess.is_peak);

        // 5. Refractario: se alimenta con el score de NAB
        self.refr.commit(assess.score_nab);

        // 6. Monitor CUSUM
        if self.spatial.count() == MIN_HISTORIAL {
            let n = self.esn.norm.len() as f64;
            if n > 1.0 {
                let _mean = self.esn.norm.standardize(value); // calibración relativa
                self.drift.calibrate(value, 1.0);
            }
        } else if self.spatial.count() > MIN_HISTORIAL {
            self.drift.update(value);
        }
    }
}
