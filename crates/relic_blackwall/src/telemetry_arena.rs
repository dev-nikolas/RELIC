//! Arena de Telemetría Fuera de Banda para relic_blackwall.
//!
//! Monitorea canales continuos mediante Seis Ojos sin contaminar el grafo episódico.
//! Soporta modos: Disabled, PassiveMonitor (observabilidad aislada) y Enforcing.

use relic_core::anomaly::{
    load_weights_from_bytes, SixEyesChannel, TelemetryAssessment, EMBED_DIM, RESERVOIR_SIZE,
    WEIGHTS_SEED42_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

pub const DEFAULT_MAX_CHANNELS: usize = 16;
pub const DEFAULT_MAX_AUDIT_RECORDS: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyGateMode {
    Disabled,
    PassiveMonitor,
    Enforcing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingTarget {
    ObservabilityOnly,
    Node(String),
    Edge { source: String, target: String },
}

#[derive(Debug, Error, PartialEq, Clone)]
pub enum BlackwallError {
    #[error("Capacidad máxima de la TelemetryArena ({max} canales) agotada")]
    ArenaExhausted { max: usize },
    #[error("Conflicto intra-lote: canal {channel_id} recibido múltiples veces en la misma transacción")]
    ChannelContention { channel_id: u32 },
    #[error("Error de inicialización de pesos en el reservorio: {reason}")]
    WeightInitError { reason: String },
    #[error("Error de integridad o transacción: {reason}")]
    TransactionError { reason: String },
    #[error("Anomalía detectada en modo Enforcing en canal {channel_id}: score_relic={score_relic:.4}, range_violation={range_violation}, drift_alarm={drift_alarm}")]
    AnomalyRejected {
        channel_id: u32,
        score_relic: f64,
        range_violation: bool,
        drift_alarm: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryAuditRecord {
    pub timestamp: u64,
    pub channel_id: u32,
    pub value: f64,
    pub score_nab: f64,
    pub score_relic: f64,
    pub range_violation: bool,
    pub drift_alarm: bool,
    pub mode: AnomalyGateMode,
}

#[derive(Clone)]
pub struct TelemetryArena {
    pub mode: AnomalyGateMode,
    pub max_channels: usize,
    pub max_audit_records: usize,
    pub channels: HashMap<u32, SixEyesChannel>,
    pub bindings: HashMap<u32, BindingTarget>,
    pub win: Box<[[f64; EMBED_DIM]; RESERVOIR_SIZE]>,
    pub wres: Box<[[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE]>,
    pub audit_log: VecDeque<TelemetryAuditRecord>,
}

impl std::fmt::Debug for TelemetryArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryArena")
            .field("mode", &self.mode)
            .field("max_channels", &self.max_channels)
            .field("max_audit_records", &self.max_audit_records)
            .field("channels_count", &self.channels.len())
            .field("bindings_count", &self.bindings.len())
            .field("audit_log_len", &self.audit_log.len())
            .finish()
    }
}

impl TelemetryArena {
    pub fn new(mode: AnomalyGateMode) -> Result<Self, BlackwallError> {
        let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES)
            .map_err(|e| BlackwallError::WeightInitError { reason: e.to_string() })?;

        Ok(Self {
            mode,
            max_channels: DEFAULT_MAX_CHANNELS,
            max_audit_records: DEFAULT_MAX_AUDIT_RECORDS,
            channels: HashMap::new(),
            bindings: HashMap::new(),
            win: Box::new(win),
            wres: Box::new(wres),
            audit_log: VecDeque::new(),
        })
    }

    pub fn with_capacity(mode: AnomalyGateMode, max_channels: usize) -> Result<Self, BlackwallError> {
        let mut arena = Self::new(mode)?;
        arena.max_channels = max_channels;
        Ok(arena)
    }

    /// Registra explícitamente un canal con su objetivo de enlace topológico o de observabilidad.
    pub fn register_channel(
        &mut self,
        channel_id: u32,
        binding: BindingTarget,
    ) -> Result<(), BlackwallError> {
        if !self.channels.contains_key(&channel_id) {
            if self.channels.len() >= self.max_channels {
                return Err(BlackwallError::ArenaExhausted { max: self.max_channels });
            }
            self.channels.insert(channel_id, SixEyesChannel::new());
        }
        self.bindings.insert(channel_id, binding);
        Ok(())
    }

    pub fn get_binding(&self, channel_id: u32) -> Option<&BindingTarget> {
        self.bindings.get(&channel_id)
    }

    pub fn is_channel_ready(&self, channel_id: u32) -> bool {
        self.channels
            .get(&channel_id)
            .map(|ch| ch.esn.embed.is_ready())
            .unwrap_or(false)
    }

    /// Evalúa en modo puro un conjunto de observaciones sin mutar estado interno.
    /// Valida que no haya contención (máximo 1 punto por canal por transacción)
    /// y gestiona el alojamiento perezoso respetando `max_channels`.
    pub fn evaluate_batch_pure(
        &mut self,
        telemetry: &[(u32, f64)],
    ) -> Result<Vec<(u32, f64, TelemetryAssessment)>, BlackwallError> {
        if self.mode == AnomalyGateMode::Disabled || telemetry.is_empty() {
            return Ok(Vec::new());
        }

        // Regla: 1 punto por canal por transacción (evita contención)
        let mut seen = HashSet::with_capacity(telemetry.len());
        for &(ch_id, _) in telemetry {
            if !seen.insert(ch_id) {
                return Err(BlackwallError::ChannelContention { channel_id: ch_id });
            }
        }

        // Alojamiento perezoso de canales
        for &(ch_id, _) in telemetry {
            if !self.channels.contains_key(&ch_id) {
                if self.channels.len() >= self.max_channels {
                    return Err(BlackwallError::ArenaExhausted { max: self.max_channels });
                }
                self.channels.insert(ch_id, SixEyesChannel::new());
                self.bindings.entry(ch_id).or_insert(BindingTarget::ObservabilityOnly);
            }
        }

        let mut assessments = Vec::with_capacity(telemetry.len());
        for &(ch_id, val) in telemetry {
            let channel = self.channels.get(&ch_id).unwrap();
            let assess = channel.evaluate_pure(val, &self.win, &self.wres);
            assessments.push((ch_id, val, assess));
        }

        Ok(assessments)
    }

    /// Compromete los puntos evaluados atómicamente tras la confirmación del WAL.
    pub fn commit_batch(
        &mut self,
        assessments: &[(u32, f64, TelemetryAssessment)],
        ts: u64,
    ) {
        if self.mode == AnomalyGateMode::Disabled {
            return;
        }

        for &(ch_id, val, ref assess) in assessments {
            if let Some(channel) = self.channels.get_mut(&ch_id) {
                channel.commit(val, assess);
            }

            // Registrar en auditoría separada con acotamiento estricto de memoria (anillo FIFO)
            if self.max_audit_records > 0 {
                if self.audit_log.len() >= self.max_audit_records {
                    self.audit_log.pop_front();
                }
                self.audit_log.push_back(TelemetryAuditRecord {
                    timestamp: ts,
                    channel_id: ch_id,
                    value: val,
                    score_nab: assess.score_nab,
                    score_relic: assess.score_relic,
                    range_violation: assess.range_violation,
                    drift_alarm: assess.drift_alarm,
                    mode: self.mode,
                });
            }
        }
    }

    /// Drena los registros de auditoría acumulados para persistencia out-of-band.
    pub fn drain_audit_log(&mut self) -> Vec<TelemetryAuditRecord> {
        self.audit_log.drain(..).collect()
    }

    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }
}
