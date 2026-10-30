//! Módulo de Telemetría y Monitoreo de Sensores (TelemetryArena).
//!
//! Este módulo recibe y analiza lecturas continuas de sensores en tiempo real
//! usando el detector Seis Ojos, manteniéndolo separado del grafo de conocimiento.
//!
//! Modos de operación:
//! - Disabled: el monitoreo está apagado (no consume CPU).
//! - PassiveMonitor (por defecto): analiza y registra anomalías en memoria para auditoría,
//!   sin bloquear ninguna operación ni alterar el archivo en disco (engram.jsonl).
//! - Enforcing: rechaza y revierte transacciones si un sensor supera el umbral de bloqueo.

use relic_core::anomaly::{
    load_weights_from_bytes, SixEyesChannel, TelemetryAssessment, EMBED_DIM, RESERVOIR_SIZE,
    WEIGHTS_SEED42_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

pub const DEFAULT_MAX_CHANNELS: usize = 16;
pub const DEFAULT_MAX_AUDIT_RECORDS: usize = 1000;
pub const DEFAULT_MAX_QUARANTINE_SLOTS: usize = 16;
pub const DEFAULT_QUARANTINE_TTL_OBSERVATIONS: u32 = 100;
pub const DEFAULT_ENFORCING_BLOCK_THRESHOLD: f64 = 0.999;
pub const DEFAULT_ALERT_THRESHOLD: f64 = 0.990;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantinedItem {
    pub channel_id: u32,
    pub target: BindingTarget,
    pub quarantined_at: u64,
    pub reason: String,
    pub lost_reachability: Vec<(String, String)>,
    pub clean_observations_remaining: u32,
}

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
    pub quarantine_target: Option<BindingTarget>,
    pub lost_reachability: Vec<(String, String)>,
    pub pool_overflow: bool,
    pub quarantine_expired: bool,
}

#[derive(Clone)]
pub struct TelemetryArena {
    pub mode: AnomalyGateMode,
    pub max_channels: usize,
    pub max_audit_records: usize,
    pub max_quarantine_slots: usize,
    pub quarantine_ttl_observations: u32,
    pub alert_threshold: f64,
    pub enforcing_block_threshold: f64,
    pub audit_file_path: Option<std::path::PathBuf>,
    pub channels: HashMap<u32, SixEyesChannel>,
    pub bindings: HashMap<u32, BindingTarget>,
    pub win: Box<[[f64; EMBED_DIM]; RESERVOIR_SIZE]>,
    pub wres: Box<[[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE]>,
    pub audit_log: VecDeque<TelemetryAuditRecord>,
    pub quarantine_pool: Vec<QuarantinedItem>,
    pub critical_pairs: Vec<(String, String)>,
}

impl std::fmt::Debug for TelemetryArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryArena")
            .field("mode", &self.mode)
            .field("max_channels", &self.max_channels)
            .field("max_audit_records", &self.max_audit_records)
            .field("max_quarantine_slots", &self.max_quarantine_slots)
            .field("channels_count", &self.channels.len())
            .field("bindings_count", &self.bindings.len())
            .field("audit_log_len", &self.audit_log.len())
            .field("quarantine_pool_len", &self.quarantine_pool.len())
            .field("critical_pairs_count", &self.critical_pairs.len())
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
            max_quarantine_slots: DEFAULT_MAX_QUARANTINE_SLOTS,
            quarantine_ttl_observations: DEFAULT_QUARANTINE_TTL_OBSERVATIONS,
            alert_threshold: DEFAULT_ALERT_THRESHOLD,
            enforcing_block_threshold: DEFAULT_ENFORCING_BLOCK_THRESHOLD,
            audit_file_path: None,
            channels: HashMap::new(),
            bindings: HashMap::new(),
            win: Box::new(win),
            wres: Box::new(wres),
            audit_log: VecDeque::new(),
            quarantine_pool: Vec::new(),
            critical_pairs: Vec::new(),
        })
    }

    pub fn with_capacity(mode: AnomalyGateMode, max_channels: usize) -> Result<Self, BlackwallError> {
        let mut arena = Self::new(mode)?;
        arena.max_channels = max_channels;
        Ok(arena)
    }

    /// Registra un par crítico de alcanzabilidad para la verificación de impacto de la SLO.
    pub fn register_critical_pair(
        &mut self,
        source: impl Into<String>,
        target: impl Into<String>,
    ) {
        self.critical_pairs.push((source.into(), target.into()));
    }

    /// Limpia los pares críticos registrados.
    pub fn clear_critical_pairs(&mut self) {
        self.critical_pairs.clear();
    }

    /// Verifica si un objetivo se encuentra actualmente bajo cuarentena en RAM.
    pub fn is_quarantined(&self, target: &BindingTarget) -> bool {
        self.quarantine_pool.iter().any(|item| &item.target == target)
    }

    /// Libera un canal de la cuarentena en RAM.
    pub fn release_quarantine(&mut self, channel_id: u32) -> bool {
        let initial_len = self.quarantine_pool.len();
        self.quarantine_pool.retain(|item| item.channel_id != channel_id);
        self.quarantine_pool.len() < initial_len
    }

    /// Limpia todos los elementos bajo cuarentena en RAM.
    pub fn clear_quarantine_pool(&mut self) {
        self.quarantine_pool.clear();
    }

    /// Retorna una referencia a los elementos actualmente en cuarentena.
    pub fn quarantined_items(&self) -> &[QuarantinedItem] {
        &self.quarantine_pool
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

    /// Evalúa un grupo de lecturas de sensores sin modificar el estado del sistema.
    /// 
    /// Verificaciones realizadas:
    /// - Cada canal solo puede enviar una lectura por transacción para evitar colisiones.
    /// - Si un canal es nuevo, se crea automáticamente respetando el límite máximo permitido (`max_channels`).
    /// - Calcula el score de anomalía para cada lectura y lo devuelve en una lista.
    pub fn evaluate_batch_pure(
        &mut self,
        telemetry: &[(u32, f64)],
    ) -> Result<Vec<(u32, f64, TelemetryAssessment)>, BlackwallError> {
        if self.mode == AnomalyGateMode::Disabled || telemetry.is_empty() {
            return Ok(Vec::new());
        }

        // Cada sensor solo puede tener 1 lectura por transacción
        let mut seen = HashSet::with_capacity(telemetry.len());
        for &(ch_id, _) in telemetry {
            if !seen.insert(ch_id) {
                return Err(BlackwallError::ChannelContention { channel_id: ch_id });
            }
        }

        // Inicializar el canal si es la primera vez que se recibe
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

    /// Aplica los cambios definitivos tras la confirmación de la transacción:
    /// 1. Actualiza el estado matemático de los detectores Seis Ojos en cada canal.
    /// 2. Si se detectó anomalía, pone el nodo o arista en cuarentena en memoria RAM.
    /// 3. Si el canal envía lecturas limpias, descuenta su tiempo de cuarentena hasta liberarlo.
    /// 4. Guarda un registro de auditoría en memoria (y opcionalmente en un archivo de log externo).
    pub fn commit_batch_with_impact(
        &mut self,
        assessments: &[(u32, f64, TelemetryAssessment)],
        impacts: &[(Option<BindingTarget>, Vec<(String, String)>)],
        ts: u64,
    ) {
        if self.mode == AnomalyGateMode::Disabled {
            return;
        }

        for (i, &(ch_id, val, ref assess)) in assessments.iter().enumerate() {
            // Actualizar el estado interno del detector en este canal
            if let Some(channel) = self.channels.get_mut(&ch_id) {
                channel.commit(val, assess);
            }

            // Obtener el objetivo enlazado a este sensor y los caminos afectados
            let (target_opt, lost_pairs) = if i < impacts.len() {
                (impacts[i].0.clone(), impacts[i].1.clone())
            } else {
                (self.bindings.get(&ch_id).cloned(), Vec::new())
            };

            let is_anomaly = assess.range_violation || assess.drift_alarm || assess.score_relic >= self.alert_threshold;
            let mut pool_overflow = false;
            let mut quarantine_expired = false;

            if is_anomaly {
                // Si hay anomalía y el sensor está enlazado a un elemento del grafo, se aísla en memoria RAM
                if let Some(ref target) = target_opt {
                    if *target != BindingTarget::ObservabilityOnly {
                        if let Some(existing) = self.quarantine_pool.iter_mut().find(|it| &it.target == target) {
                            existing.quarantined_at = ts;
                            existing.reason = format!(
                                "score_relic={:.4}, range={}, drift={}",
                                assess.score_relic, assess.range_violation, assess.drift_alarm
                            );
                            existing.lost_reachability = lost_pairs.clone();
                            // Si reincide en la anomalía, se reinicia el tiempo de espera limpio
                            existing.clean_observations_remaining = self.quarantine_ttl_observations;
                        } else if self.quarantine_pool.len() < self.max_quarantine_slots {
                            self.quarantine_pool.push(QuarantinedItem {
                                channel_id: ch_id,
                                target: target.clone(),
                                quarantined_at: ts,
                                reason: format!(
                                    "score_relic={:.4}, range={}, drift={}",
                                    assess.score_relic, assess.range_violation, assess.drift_alarm
                                ),
                                lost_reachability: lost_pairs.clone(),
                                clean_observations_remaining: self.quarantine_ttl_observations,
                            });
                        } else {
                            pool_overflow = true; // Se alcanzó el límite de cuarentenas activas
                        }
                    }
                }
            } else {
                // Lectura normal: se descuenta una observación limpia del contador del canal
                if let Some(pos) = self.quarantine_pool.iter().position(|it| it.channel_id == ch_id) {
                    if self.quarantine_pool[pos].clean_observations_remaining > 1 {
                        self.quarantine_pool[pos].clean_observations_remaining -= 1;
                    } else {
                        // Cumplió 100 observaciones normales consecutivas: se libera de la cuarentena
                        self.quarantine_pool.swap_remove(pos);
                        quarantine_expired = true;
                    }
                }
            }

            let audit_rec = TelemetryAuditRecord {
                timestamp: ts,
                channel_id: ch_id,
                value: val,
                score_nab: assess.score_nab,
                score_relic: assess.score_relic,
                range_violation: assess.range_violation,
                drift_alarm: assess.drift_alarm,
                mode: self.mode,
                quarantine_target: target_opt,
                lost_reachability: lost_pairs,
                pool_overflow,
                quarantine_expired,
            };

            // Escribir en archivo de auditoría externo si está habilitado (nunca toca engram.jsonl)
            if let Some(ref path) = self.audit_file_path {
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                    use std::io::Write;
                    if let Ok(json_line) = serde_json::to_string(&audit_rec) {
                        let _ = writeln!(f, "{}", json_line);
                    }
                }
            }

            // Registrar en auditoría en memoria (anillo FIFO acotado)
            if self.max_audit_records > 0 {
                if self.audit_log.len() >= self.max_audit_records {
                    self.audit_log.pop_front();
                }
                self.audit_log.push_back(audit_rec);
            }
        }
    }

    /// Compromete los puntos evaluados atómicamente tras la confirmación del WAL (compatibilidad estándar).
    pub fn commit_batch(
        &mut self,
        assessments: &[(u32, f64, TelemetryAssessment)],
        ts: u64,
    ) {
        self.commit_batch_with_impact(assessments, &[], ts);
    }

    /// Drena los registros de auditoría acumulados para persistencia out-of-band.
    pub fn drain_audit_log(&mut self) -> Vec<TelemetryAuditRecord> {
        self.audit_log.drain(..).collect()
    }

    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }
}
