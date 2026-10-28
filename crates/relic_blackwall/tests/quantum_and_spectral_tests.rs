//! Suite Científica de Interferencia Cuántica y Verificador Espectral (`quantum_and_spectral_tests.rs`). Menos de 180 líneas.

use std::collections::HashMap;
use std::f64::consts::PI;
use relic_blackwall::{BlackwallGate, TransactionOutcome};
use relic_core::{
    analyze_spectrum_donoho_stark, compute_asymmetric_threshold, pearson_correlation_welford,
    verify_network_alert, ArithmeticOp, CollapseDecision, Complex, NetworkAlertSpec,
    QuantumSuperposition, SpectralOutcome,
};
use relic_lexicon::{ClassicalPolysemyResult, LexiconPayload};

#[test]
fn test_1_destructive_phase_interference_vs_classical_baseline_in_polysemy() {
    let mut gate = BlackwallGate::with_genesis();
    let payloads = [
        LexiconPayload::quantum_polysemous_alias("tumbar", ArithmeticOp::Subtract, 1.0, 0.0, None),
        LexiconPayload::quantum_polysemous_alias("tumbar", ArithmeticOp::Divide, 1.0, 0.0, None),
        LexiconPayload::quantum_polysemous_alias("tumbar", ArithmeticOp::Subtract, 1.0, 0.0, Some("descuento".to_string())),
        LexiconPayload::quantum_polysemous_alias("tumbar", ArithmeticOp::Divide, 1.0, PI, Some("descuento".to_string())),
    ];
    for (idx, payload) in payloads.into_iter().enumerate() {
        assert!(matches!(gate.propose_mutation(payload, 5000 + idx as u64), TransactionOutcome::Committed { .. }));
    }

    let classical_res = gate.resolve_polysemous_classical("tumbar", &["descuento"]);
    assert!(matches!(classical_res, ClassicalPolysemyResult::Ambiguous { .. }));

    let quantum_res = gate.resolve_polysemous_quantum("tumbar", &["descuento"], 3.0, 0.0).expect("Entrada polisémica");
    match quantum_res {
        CollapseDecision::Collapsed {
            target, born_probability, required_threshold, von_neumann_shannon_entropy, trace,
        } => {
            assert_eq!(target, ArithmeticOp::Subtract);
            assert!((born_probability - 1.0).abs() < 1e-9);
            assert!((required_threshold - 0.75).abs() < 1e-9);
            assert!(von_neumann_shannon_entropy < 1e-9);
            assert_eq!(trace.len(), 2);
            assert!(trace.iter().any(|t| t.interference_kind == "destructiva" && t.cos_delta < -0.99));
            assert!(trace.iter().any(|t| t.interference_kind == "constructiva" && t.cos_delta > 0.99));
        }
        other => panic!("Se esperaba colapso en Subtract, obtenido: {other:?}"),
    }

    let (_, answer) = gate.evaluate_query("Tengo 100 monedas y por descuento tumbar 25");
    assert_eq!(answer.status, "VALID");
    assert!(answer.text.contains("75"));
}

#[test]
fn test_2_dft_beaconing_detection_and_donoho_stark_uncertainty_theorem() {
    let dirac_comb: Vec<f64> = (0..16).map(|n| if n % 4 == 0 { 1.0 } else { 0.0 }).collect();
    match analyze_spectrum_donoho_stark(&dirac_comb, 0.05, 3.0, None) {
        SpectralOutcome::PeriodicBeaconDetected {
            n_samples, dominant_bin, estimated_period, temporal_support, spectral_support,
            donoho_stark_product, donoho_stark_holds, donoho_stark_distance, ..
        } => {
            assert_eq!((n_samples, dominant_bin, temporal_support, spectral_support, donoho_stark_product), (16, 4, 4, 4, 16));
            assert!((estimated_period - 4.0).abs() < 1e-9 && donoho_stark_holds && donoho_stark_distance.abs() < 1e-9);
        }
        other => panic!("Se esperaba PeriodicBeaconDetected, obtenido: {other:?}"),
    }

    let jittered_signal = vec![1.0, 0.2, 0.0, 0.8, 0.1, 0.9, 0.0, 0.0, 0.7, 0.3, 0.5, 0.0, 0.1, 0.6, 0.4, 0.2];
    match analyze_spectrum_donoho_stark(&jittered_signal, 0.05, 3.0, None) {
        SpectralOutcome::DispersedOrAperiodic { donoho_stark_holds, donoho_stark_distance, spectral_support, .. } => {
            assert!(donoho_stark_holds && spectral_support > 4 && donoho_stark_distance > 1.0);
        }
        other => panic!("Se esperaba DispersedOrAperiodic, obtenido: {other:?}"),
    }
}

#[test]
fn test_3_abstention_by_insufficient_window_and_asymmetric_cost() {
    let short_window_signal: Vec<f64> = (0..16).map(|n| if n % 8 == 0 { 1.0 } else { 0.0 }).collect();
    match analyze_spectrum_donoho_stark(&short_window_signal, 0.05, 3.0, Some(8.0)) {
        SpectralOutcome::InsufficientWindow { n_samples, candidate_period, min_cycles_required, min_samples_required, reason, .. } => {
            assert_eq!((n_samples, min_samples_required), (16, 24));
            assert!((candidate_period - 8.0).abs() < 1e-9 && (min_cycles_required - 3.0).abs() < 1e-9);
            assert!(reason.contains("Abstención por ventana insuficiente"));
        }
        other => panic!("Se esperaba InsufficientWindow, obtenido: {other:?}"),
    }

    let p_star = compute_asymmetric_threshold(9.0, 0.0);
    assert!((p_star - 0.90).abs() < 1e-9);
    let qs = QuantumSuperposition::new(vec![
        (ArithmeticOp::Subtract, Complex::from_polar(0.80_f64.sqrt(), 0.0)),
        (ArithmeticOp::Divide, Complex::from_polar(0.20_f64.sqrt(), 0.0)),
    ]);
    match qs.collapse_with_asymmetric_cost(9.0, 0.0) {
        CollapseDecision::Abstained { reason, max_prob, required_threshold, .. } => {
            assert!((max_prob - 0.80).abs() < 1e-9 && (required_threshold - 0.90).abs() < 1e-9);
            assert!(reason.contains("Incertidumbre bajo costo asimétrico"));
        }
        other => panic!("Se esperaba Abstained, obtenido: {other:?}"),
    }
}

#[test]
fn test_4_defensive_network_alert_verification_c2_bfs_and_spectral() {
    let r = pearson_correlation_welford(&[10.0, 20.0, 30.0, 40.0, 50.0], &[15.0, 30.0, 45.0, 60.0, 75.0]).unwrap();
    assert!((r - 1.0).abs() < 1e-12);

    let mut dest_ips = HashMap::new();
    dest_ips.insert("198.51.100.42".to_string(), 64);
    dest_ips.insert("198.51.100.99".to_string(), 64);

    let alert_spec = NetworkAlertSpec {
        syn_count: 480,
        ack_count: 12,
        destination_ip_counts: dest_ips,
        packet_time_series: (0..16).map(|n| if n % 4 == 0 { 1.0 } else { 0.0 }).collect(),
        topology_edges: vec![
            ("Host_Comprometido_A".to_string(), "Firewall_Puente_DMZ".to_string()),
            ("Firewall_Puente_DMZ".to_string(), "DB_Produccion".to_string()),
            ("Firewall_Puente_DMZ".to_string(), "Boveda_Claves".to_string()),
            ("Host_Comprometido_A".to_string(), "Collector_SIEM".to_string()),
        ],
        compromised_origins: vec!["Host_Comprometido_A".to_string()],
        critical_targets: vec!["DB_Produccion".to_string(), "Boveda_Claves".to_string(), "Collector_SIEM".to_string()],
        bridge_node_to_isolate: "Firewall_Puente_DMZ".to_string(),
        min_cycles: 3.0,
    };

    let report = verify_network_alert(&alert_spec);
    assert!((report.syn_ack_ratio - 40.0).abs() < 1e-9 && (report.destination_entropy_bits - 1.0).abs() < 1e-9);
    assert!(report.is_c2_beacon_confirmed);
    assert_eq!(report.reachable_before_isolation, vec!["Boveda_Claves".to_string(), "Collector_SIEM".to_string(), "DB_Produccion".to_string()]);
    assert_eq!(report.protected_targets_after_isolation, vec!["Boveda_Claves".to_string(), "DB_Produccion".to_string()]);
    assert_eq!(report.still_exposed_targets, vec!["Collector_SIEM".to_string()]);
    assert!(!report.trace.is_empty());
}
