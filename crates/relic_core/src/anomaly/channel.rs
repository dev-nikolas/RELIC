//! Canal de Telemetría Seis Ojos (SixEyesChannel).
//!
//! Este componente procesa datos numéricos continuos (como lecturas de sensores IoT)
//! paso a paso para detectar anomalías en tiempo real.
//!
//! Flujo de procesamiento por cada punto recibido:
//! 1. Normalización rodante: ajusta la escala restando la media y dividiendo por la desviación estándar.
//! 2. Predicción con ESN (Echo State Network): una red neuronal recurrente ligera predice el siguiente valor.
//! 3. Residuo de predicción: se calcula la diferencia entre lo que ocurrió y lo que la red predijo.
//! 4. Calibración empírica (ECDF): convierte ese residuo en un percentil (0.0 a 1.0) comparándolo con la historia reciente.
//! 5. Filtro de picos (Peak Decay): destaca picos anómalos y atenúa oscilaciones repetitivas.
//! 6. Supresor refractario: evita alertas duplicadas consecutivas durante un periodo de enfriamiento.
//! 7. Monitor de deriva CUSUM: detecta cambios lentos y sostenidos en el promedio de la señal.
//! 8. Ruptura de rango (Spatial Bypass): detecta de inmediato valores que se salgan bruscamente de los límites conocidos.

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

    /// Evaluación pura e inmutable (no modifica el estado del canal y no hace allocs de memoria).
    /// Permite calcular el score de anomalía antes de decidir si la transacción se confirma o se descarta.
    pub fn evaluate_pure(
        &self,
        value: f64,
        win: &[[f64; EMBED_DIM]; RESERVOIR_SIZE],
        wres: &[[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE],
    ) -> TelemetryAssessment {
        // Si el valor no es un número válido (NaN o Infinito), se rechaza de inmediato por seguridad.
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
                score_nab: 0.0,   // Los benchmarks de NAB ignoran valores no numéricos
                score_relic: 1.0, // RELIC bloquea valores NaN o Inf como una violación crítica
                range_violation: true,
                drift_alarm: false,
                is_refractory_suppressed: false,
            };
        }

        // Paso 1: Comprobar si el valor se sale de los límites históricos conocidos (mínimo y máximo).
        let spatial_anomaly = self.spatial.check_anomaly(value);

        // Paso 2: Ejecutar la predicción con la red neuronal recurrente (ESN) y el filtro RLS.
        // Calcula qué valor se esperaba (pred_z) y el error de predicción (err).
        let (z, vec_opt, h_cand, pred_z, err) = self.esn.evaluate_pure(value, win, wres);
        let ready = vec_opt.is_some();

        // Paso 3: Convertir el error en un percentil de 0.0 a 1.0 usando el historial empírico (ECDF).
        // Un score de 0.99 significa que el 99% de los errores pasados fueron menores que este.
        let cal_score = self.ecdf.score(err);

        // Paso 4: Filtrar picos (NMS). Si el score es un pico local, se resalta; si decae, se suaviza.
        let (is_peak, nms_score) = self.decay.evaluate_pure(cal_score);

        // Si hubo una ruptura física de rango, le damos prioridad máxima (score 1.0).
        let raw_detector_score = if spatial_anomaly { 1.0 } else { nms_score };

        // Paso 5: Compuerta refractaria. Evita emitir ráfagas de alertas consecutivas para el mismo evento.
        let (is_suppressed, score_nab) = self.refr.evaluate_pure(raw_detector_score);

        // Paso 6: Score de seguridad para RELIC. A diferencia de NAB, RELIC no suprime anomalías críticas
        // para asegurar que las reglas de integridad no se silencien durante el enfriamiento.
        let score_relic = if spatial_anomaly { 1.0 } else { raw_detector_score };

        // Paso 7: Monitor CUSUM para detectar si la media de la señal se desplazó gradualmente.
        let (drift_alarm, _, _) = self.drift.evaluate_pure(value);

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

    /// Guarda los cambios en el estado interno del detector.
    /// Solo se llama cuando la transacción completa fue aprobada y confirmada.
    pub fn commit(&mut self, value: f64, assess: &TelemetryAssessment) {
        if !value.is_finite() {
            return;
        }

        // 1. Actualizar los límites mínimo y máximo observados.
        self.spatial.commit(value);

        // 2. Avanzar el estado de la red neuronal y los pesos adaptativos del filtro RLS.
        self.esn.commit(
            value,
            assess.z_value,
            assess.ready,
            assess.h_candidate,
            assess.pred_z,
        );

        // 3. Registrar el error en la ventana de percentiles ECDF.
        self.ecdf.push(assess.raw_residual);

        // 4. Registrar el pico para el decaimiento temporal.
        self.decay.commit(assess.cal_score, assess.is_peak);

        // 5. Actualizar el temporizador de la compuerta refractaria.
        self.refr.commit(assess.score_nab);

        // 6. Calibrar o actualizar el monitor de deriva CUSUM con la media inicial tras 288 lecturas.
        if self.spatial.count() == MIN_HISTORIAL {
            let (mean, std) = self.esn.norm.mean_and_std();
            self.drift.calibrate(mean, std);
        } else if self.spatial.count() > MIN_HISTORIAL {
            self.drift.update(value);
        }
    }
}

impl Default for SixEyesChannel {
    fn default() -> Self {
        Self::new()
    }
}
