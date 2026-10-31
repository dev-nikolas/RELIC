//! # SixEyesChannel Engine
//!
//! Canal de telemetría unificado integrando ESN, ECDF, CUSUM, Peak Decay,
//! supresor refractario y acoplamiento topológico con martingala SPRT.

use crate::anomaly::cusum::TelemetryDriftMonitor;
use crate::anomaly::ecdf::{EcdfWindow, MIN_HISTORIAL};
use crate::anomaly::echo_state::{EchoStatePredictor, EMBED_DIM, RESERVOIR_SIZE};
use crate::anomaly::peak_decay::StreamingPeakDecay;
use crate::anomaly::refractory::RefractorySuppressor;
use crate::anomaly::spatial_bypass::SpatialSpikeBypass;
use crate::anomaly::topology_coupling::{NodeSpatialNeighborhood, ResidualSprtMartingale};
use super::types::TelemetryAssessment;

#[derive(Clone)]
pub struct SixEyesChannel {
    pub esn: EchoStatePredictor,
    pub ecdf: EcdfWindow,
    pub decay: StreamingPeakDecay,
    pub spatial: SpatialSpikeBypass,
    pub refr: RefractorySuppressor,
    pub drift: TelemetryDriftMonitor,
    pub martingale: ResidualSprtMartingale,
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
            martingale: ResidualSprtMartingale::default(),
        }
    }

    pub fn reset(&mut self) {
        self.esn.reset();
        self.ecdf.reset();
        self.decay.reset();
        self.spatial.reset();
        self.refr.reset();
        self.drift.reset();
        self.martingale.reset();
    }

    pub fn evaluate_with_topology(
        &self,
        value: f64,
        neighborhood: &NodeSpatialNeighborhood,
        sensor_snapshot: &[f32],
        win: &[[f64; EMBED_DIM]; RESERVOIR_SIZE],
        wres: &[[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE],
    ) -> TelemetryAssessment {
        if !value.is_finite() {
            return TelemetryAssessment {
                raw_residual: 0.0, z_value: 0.0, ready: false,
                h_candidate: self.esn.h, pred_z: 0.0, cal_score: 0.0,
                is_peak: false, nms_score: 0.0, score_nab: 0.0, score_relic: 1.0,
                range_violation: true, drift_alarm: false, is_refractory_suppressed: false,
                topological_residual: 0.0, spatial_divergence: f64::INFINITY,
                martingale_alarm: true, martingale_log_l: f64::INFINITY,
            };
        }

        let is_coupled = neighborhood.count > 0;
        let (topo_residual, spatial_div, mart_alarm, mart_log_l) = if is_coupled {
            let res = neighborhood.compute_residual(value as f32, sensor_snapshot) as f64;
            let (alarm, log_l) = self.martingale.evaluate_pure(res as f32);
            (res, res.abs(), alarm, log_l as f64)
        } else {
            (0.0, 0.0, false, 0.0)
        };

        let effective_range_violation = if is_coupled { mart_alarm } else { self.spatial.check_anomaly(value) };
        let (z, vec_opt, h_cand, pred_z, err) = self.esn.evaluate_pure(value, win, wres);
        let cal_score = self.ecdf.score(err);
        let (is_peak, nms_score) = self.decay.evaluate_pure(cal_score);
        let raw_detector_score = if effective_range_violation { 1.0 } else { nms_score };
        let (is_suppressed, score_nab) = self.refr.evaluate_pure(raw_detector_score);

        let score_relic = if effective_range_violation {
            1.0
        } else if is_coupled && !mart_alarm {
            nms_score.min(0.49)
        } else {
            raw_detector_score
        };

        let (drift_alarm, _, _) = self.drift.evaluate_pure(value);

        TelemetryAssessment {
            raw_residual: err, z_value: z, ready: vec_opt.is_some(),
            h_candidate: h_cand, pred_z, cal_score, is_peak, nms_score,
            score_nab, score_relic, range_violation: effective_range_violation,
            drift_alarm: if is_coupled && !mart_alarm { false } else { drift_alarm },
            is_refractory_suppressed: is_suppressed, topological_residual: topo_residual,
            spatial_divergence: spatial_div, martingale_alarm: mart_alarm, martingale_log_l: mart_log_l,
        }
    }

    #[inline(always)]
    pub fn evaluate_pure(
        &self,
        value: f64,
        win: &[[f64; EMBED_DIM]; RESERVOIR_SIZE],
        wres: &[[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE],
    ) -> TelemetryAssessment {
        let empty_nh = NodeSpatialNeighborhood::empty();
        self.evaluate_with_topology(value, &empty_nh, &[], win, wres)
    }

    pub fn commit(&mut self, value: f64, assess: &TelemetryAssessment) {
        if !value.is_finite() { return; }

        self.spatial.commit(value);
        self.esn.commit(value, assess.z_value, assess.ready, assess.h_candidate, assess.pred_z);
        self.ecdf.push(assess.raw_residual);
        self.decay.commit(assess.cal_score, assess.is_peak);
        self.refr.commit(assess.score_nab);

        if self.spatial.count() == MIN_HISTORIAL {
            let (mean, std) = self.esn.norm.mean_and_std();
            self.drift.calibrate(mean, std);
        } else if self.spatial.count() > MIN_HISTORIAL {
            self.drift.update(value);
        }

        self.martingale.update(assess.topological_residual as f32);
    }
}

impl Default for SixEyesChannel {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

