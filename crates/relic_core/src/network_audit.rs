//! # relic_core::network_audit
//!
//! Auditoría defensiva determinista de telemetría de red (C2 beaconing + aislamiento topológico BFS).
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::episodic_memory::EpisodicMemory;
use crate::reasoning_engine::ReasoningEngine;
use crate::spectral::{analyze_spectrum_donoho_stark, SpectralOutcome};

/// Calcula el ratio de asimetría de bandera $R_{\text{syn}} = \frac{\text{SYN}}{\max(\text{ACK}, 1)}$.
#[inline]
pub fn compute_syn_ack_ratio(syn_count: u64, ack_count: u64) -> f64 {
    (syn_count as f64) / (ack_count.max(1) as f64)
}

/// Calcula la entropía de Shannon $H = -\sum_i p_i \log_2(p_i)$ sobre conteos de IPs/destinos.
pub fn compute_destination_entropy(dest_counts: &HashMap<String, u64>) -> f64 {
    let total: u64 = dest_counts.values().sum();
    if total == 0 {
        return 0.0;
    }
    let total_f = total as f64;
    let mut h = 0.0;
    for &c in dest_counts.values() {
        if c > 0 {
            let p = (c as f64) / total_f;
            h -= p * p.log2();
        }
    }
    h.max(0.0)
}

/// Registro estructurado de telemetría/alerta de red para verificación determinista (C2 + BFS + Espectral).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkAlertSpec {
    pub syn_count: u64,
    pub ack_count: u64,
    pub destination_ip_counts: HashMap<String, u64>,
    pub packet_time_series: Vec<f64>,
    pub topology_edges: Vec<(String, String)>,
    pub compromised_origins: Vec<String>,
    pub critical_targets: Vec<String>,
    pub bridge_node_to_isolate: String,
    pub min_cycles: f64,
}

/// Reporte verificado de auditoría defensiva combinando métricas espectrales, estadísticas y topológicas (BFS).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedAlertReport {
    pub syn_ack_ratio: f64,
    pub destination_entropy_bits: f64,
    pub is_c2_beacon_confirmed: bool,
    pub spectral: SpectralOutcome,
    pub compromised_hosts: Vec<String>,
    pub reachable_before_isolation: Vec<String>,
    pub protected_targets_after_isolation: Vec<String>,
    pub still_exposed_targets: Vec<String>,
    pub trace: Vec<String>,
}

/// Evalúa y verifica matemáticamente una alerta estructurada de red (detección de beaconing C2 + aislamiento BFS).
pub fn verify_network_alert(spec: &NetworkAlertSpec) -> VerifiedAlertReport {
    let mut trace = Vec::new();
    let syn_ack_ratio = compute_syn_ack_ratio(spec.syn_count, spec.ack_count);
    let destination_entropy_bits = compute_destination_entropy(&spec.destination_ip_counts);
    trace.push(format!(
        "Métricas de flujo: R_syn={syn_ack_ratio:.2} (SYN={}, ACK={}), Entropía destinos H={destination_entropy_bits:.4} bits",
        spec.syn_count, spec.ack_count
    ));

    let spectral = analyze_spectrum_donoho_stark(&spec.packet_time_series, 0.05, spec.min_cycles, None);
    let is_c2_beacon_confirmed = matches!(spectral, SpectralOutcome::PeriodicBeaconDetected { .. });

    let mut mem = EpisodicMemory::new();
    for (u, v) in &spec.topology_edges {
        mem.insert_edge(u.clone(), v.clone());
    }

    let mut reachable_before_set = HashSet::new();
    let mut still_exposed_set = HashSet::new();
    let avoid = [spec.bridge_node_to_isolate.as_str()];

    for origin in &spec.compromised_origins {
        for target in &spec.critical_targets {
            let before = ReasoningEngine::solve_reachability_bfs(&mut mem, origin, target, 32);
            if before.is_reachable {
                reachable_before_set.insert(target.clone());
            }
            let after = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, origin, target, 32, &avoid);
            if after.is_reachable {
                still_exposed_set.insert(target.clone());
            }
        }
    }

    let mut reachable_before_isolation: Vec<String> = reachable_before_set.into_iter().collect();
    reachable_before_isolation.sort();

    let mut still_exposed_targets: Vec<String> = still_exposed_set.iter().cloned().collect();
    still_exposed_targets.sort();

    let mut protected_targets_after_isolation: Vec<String> = reachable_before_isolation
        .iter()
        .filter(|t| !still_exposed_set.contains(*t))
        .cloned()
        .collect();
    protected_targets_after_isolation.sort();

    trace.push(format!(
        "Simulación BFS aislando puente '{}': objetivos antes={:?}, protegidos tras aislamiento={:?}, aún expuestos={:?}",
        spec.bridge_node_to_isolate, reachable_before_isolation, protected_targets_after_isolation, still_exposed_targets
    ));

    VerifiedAlertReport {
        syn_ack_ratio,
        destination_entropy_bits,
        is_c2_beacon_confirmed,
        spectral,
        compromised_hosts: spec.compromised_origins.clone(),
        reachable_before_isolation,
        protected_targets_after_isolation,
        still_exposed_targets,
        trace,
    }
}

