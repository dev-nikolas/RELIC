//! Benchmark 3 — Auditoría de Telemetría y Ciberseguridad (N-BaIoT / CTU-13 / Mirai).
//! Menos de 180 líneas.

use std::collections::HashMap;
use std::time::Instant;
use relic_core::{analyze_spectrum_donoho_stark, verify_network_alert, NetworkAlertSpec, SpectralOutcome};
use serde::{Deserialize, Serialize};
use crate::bench::compute_mean_and_p99;

/// Métricas del Benchmark 3 (Auditoría Espectral y Topológica de Red C2 / N-BaIoT / CTU-13).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkTelemetryMetrics {
    pub total_traces: usize,
    pub attack_traces: usize,
    pub benign_jitter_traces: usize,
    pub truncated_window_traces: usize,
    pub beacon_detection_recall_pct: f64,
    pub false_positive_rate_pct: f64,
    pub uncertainty_abstention_accuracy_pct: f64,
    pub topological_isolation_exact_match_pct: f64,
    pub mean_latency_us: f64,
    pub p99_latency_us: f64,
}

enum NetworkScenario {
    AttackC2 { spec: NetworkAlertSpec, expected_protected: Vec<&'static str> },
    BenignJitter { signal: Vec<f64> },
    TruncatedWindow { signal: Vec<f64>, min_cycles: f64, period_hint: Option<f64> },
}

/// Ejecuta el Benchmark 3: Auditoría de Telemetría y Ciberseguridad (14 escenarios).
pub fn run_bench3_network_telemetry() -> NetworkTelemetryMetrics {
    let make_alert = |period: usize, n: usize, bridge: &str, edges: Vec<(&str, &str)>, origins: Vec<&str>, targets: Vec<&str>| -> NetworkAlertSpec {
        let mut ips = HashMap::new();
        ips.insert("198.51.100.10".to_string(), 50);
        ips.insert("198.51.100.20".to_string(), 50);
        NetworkAlertSpec {
            syn_count: 500,
            ack_count: 10,
            destination_ip_counts: ips,
            packet_time_series: (0..n).map(|i| if i % period == 0 { 1.0 } else { 0.0 }).collect(),
            topology_edges: edges.into_iter().map(|(u, v)| (u.to_string(), v.to_string())).collect(),
            compromised_origins: origins.into_iter().map(|s| s.to_string()).collect(),
            critical_targets: targets.into_iter().map(|s| s.to_string()).collect(),
            bridge_node_to_isolate: bridge.to_string(),
            min_cycles: 3.0,
        }
    };

    let scenarios: Vec<NetworkScenario> = vec![
        NetworkScenario::AttackC2 {
            spec: make_alert(4, 16, "GW_DMZ", vec![("Bot_IoT_1", "GW_DMZ"), ("GW_DMZ", "DB_Core"), ("GW_DMZ", "Vault_Keys")], vec!["Bot_IoT_1"], vec!["DB_Core", "Vault_Keys"]),
            expected_protected: vec!["DB_Core", "Vault_Keys"],
        },
        NetworkScenario::AttackC2 {
            spec: make_alert(2, 16, "Switch_Piso2", vec![("Camara_IP_4", "Switch_Piso2"), ("Switch_Piso2", "PLC_Scada"), ("Camara_IP_4", "Log_Collector")], vec!["Camara_IP_4"], vec!["PLC_Scada", "Log_Collector"]),
            expected_protected: vec!["PLC_Scada"],
        },
        NetworkScenario::AttackC2 {
            spec: make_alert(5, 20, "Router_Interno", vec![("Host_Infectado", "Router_Interno"), ("Router_Interno", "Subred_Pagos"), ("Router_Interno", "Subred_Nomina")], vec!["Host_Infectado"], vec!["Subred_Nomina", "Subred_Pagos"]),
            expected_protected: vec!["Subred_Nomina", "Subred_Pagos"],
        },
        NetworkScenario::AttackC2 {
            spec: make_alert(3, 18, "Proxy_Saliente", vec![("Nodo_Mirai_A", "Proxy_Saliente"), ("Nodo_Mirai_B", "Proxy_Saliente"), ("Proxy_Saliente", "Servidor_AD")], vec!["Nodo_Mirai_A", "Nodo_Mirai_B"], vec!["Servidor_AD"]),
            expected_protected: vec!["Servidor_AD"],
        },
        NetworkScenario::AttackC2 {
            spec: make_alert(6, 24, "Firewall_OT", vec![("Sensor_Comprometido", "Firewall_OT"), ("Firewall_OT", "Turbina_Control")], vec!["Sensor_Comprometido"], vec!["Turbina_Control"]),
            expected_protected: vec!["Turbina_Control"],
        },
        NetworkScenario::BenignJitter { signal: vec![1.0, 0.2, 0.0, 0.8, 0.1, 0.9, 0.0, 0.0, 0.7, 0.3, 0.5, 0.0, 0.1, 0.6, 0.4, 0.2] },
        NetworkScenario::BenignJitter { signal: vec![0.4, 0.9, 0.3, 0.1, 0.8, 0.6, 0.2, 0.7, 0.5, 0.1, 0.9, 0.4, 0.3, 0.8, 0.2, 0.6] },
        NetworkScenario::BenignJitter { signal: vec![0.1, 0.0, 0.9, 0.8, 0.2, 0.1, 0.4, 0.7, 0.3, 0.6, 0.5, 0.2, 0.8, 0.1, 0.3, 0.9] },
        NetworkScenario::BenignJitter { signal: vec![0.9, 0.7, 0.5, 0.3, 0.8, 0.2, 0.6, 0.4, 0.1, 0.5, 0.7, 0.9, 0.2, 0.4, 0.6, 0.8] },
        NetworkScenario::BenignJitter { signal: vec![0.2, 0.5, 0.8, 0.1, 0.4, 0.9, 0.3, 0.6, 0.7, 0.2, 0.5, 0.8, 0.3, 0.1, 0.7, 0.4] },
        NetworkScenario::TruncatedWindow { signal: vec![1.0, 0.0, 1.0], min_cycles: 3.0, period_hint: None },
        NetworkScenario::TruncatedWindow { signal: (0..16).map(|i| if i % 8 == 0 { 1.0 } else { 0.0 }).collect(), min_cycles: 3.0, period_hint: Some(8.0) },
        NetworkScenario::TruncatedWindow { signal: (0..12).map(|i| if i % 6 == 0 { 1.0 } else { 0.0 }).collect(), min_cycles: 3.0, period_hint: Some(6.0) },
        NetworkScenario::TruncatedWindow { signal: vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], min_cycles: 3.0, period_hint: None },
    ];

    let total_traces = scenarios.len();
    let (mut attack_traces, mut attacks_detected, mut topo_exact_matches) = (0usize, 0usize, 0usize);
    let (mut benign_jitter_traces, mut false_positives) = (0usize, 0usize);
    let (mut truncated_window_traces, mut correct_uncertainty_abstentions) = (0usize, 0usize);
    let mut latencies_us = Vec::with_capacity(total_traces);

    for sc in &scenarios {
        match sc {
            NetworkScenario::AttackC2 { spec, expected_protected } => {
                attack_traces += 1;
                let t0 = Instant::now();
                let rep = verify_network_alert(spec);
                latencies_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);
                if rep.is_c2_beacon_confirmed { attacks_detected += 1; }
                let exp_vec: Vec<String> = expected_protected.iter().map(|s| s.to_string()).collect();
                if rep.protected_targets_after_isolation == exp_vec { topo_exact_matches += 1; }
            }
            NetworkScenario::BenignJitter { signal } => {
                benign_jitter_traces += 1;
                let t0 = Instant::now();
                let out = analyze_spectrum_donoho_stark(signal, 0.05, 3.0, None);
                latencies_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);
                if matches!(out, SpectralOutcome::PeriodicBeaconDetected { .. }) { false_positives += 1; }
            }
            NetworkScenario::TruncatedWindow { signal, min_cycles, period_hint } => {
                truncated_window_traces += 1;
                let t0 = Instant::now();
                let out = analyze_spectrum_donoho_stark(signal, 0.05, *min_cycles, *period_hint);
                latencies_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);
                if matches!(out, SpectralOutcome::InsufficientWindow { .. }) { correct_uncertainty_abstentions += 1; }
            }
        }
    }

    let (mean_latency_us, p99_latency_us) = compute_mean_and_p99(&latencies_us);
    NetworkTelemetryMetrics {
        total_traces,
        attack_traces,
        benign_jitter_traces,
        truncated_window_traces,
        beacon_detection_recall_pct: (attacks_detected as f64 / attack_traces as f64) * 100.0,
        false_positive_rate_pct: (false_positives as f64 / benign_jitter_traces as f64) * 100.0,
        uncertainty_abstention_accuracy_pct: (correct_uncertainty_abstentions as f64 / truncated_window_traces as f64) * 100.0,
        topological_isolation_exact_match_pct: (topo_exact_matches as f64 / attack_traces as f64) * 100.0,
        mean_latency_us,
        p99_latency_us,
    }
}

