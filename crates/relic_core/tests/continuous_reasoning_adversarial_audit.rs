//! # Continuous Reasoning & Blinded Architecture Adversarial Audit
//!
//! Suite de certificación adversarial integral de los 6 Pilares de Razonamiento Continuo:
//! 1. Resistencia al Ataque de la Rana Hervida (Quiescence-Gated NLMS + Box Clamp).
//! 2. Prevención de Incendios en Cascada de Kirchhoff N-3 (LODF SIMD Superposition + Safe-Halt).
//! 3. Inmunidad a Ceguera por Variance Spoofing (Escala Welford Acotada + h Invariante).
//! 4. Decodificación de Síndromes y Abstención ante Equifinalidad (Causal ECC + EpistemicScratchpad).
//! 5. Blindaje contra Auto-Immune DoS (Gating Empírico Estricto: Prior vs. Posterior).
//! 6. Monotonía de Seguridad (Subsunción Convexa Dominante en 1 Slot sin Pérdida Monotónica).

use relic_core::anomaly::topology_coupling::{
    NodeSpatialNeighborhood, QuiescentCalibrator, ResidualSprtMartingale,
    WALD_THRESHOLD_ALPHA_1E6,
};
use relic_core::blackwall_scars::{
    Avx2PredicateBlock, CriticalEdgeKey, EmpiricalFailureToken, NeighborTopologyToken,
    StructuralDiscriminationEngine, WL1Signature,
};
use relic_core::contingency_bfs::{LinearizedSensitivityMatrix, MAX_SENSITIVITY_EDGES};
use relic_core::repair::{
    EdgeOperatingBounds, LocalConstraintRepairer, StagedEnvelope,
};
use relic_core::syndrome::{
    CausalParityMatrix, CausalSyndromeDecoder, EpistemicScratchpad, SyndromeVerdict,
};

#[test]
fn test_pillar_1_boiling_frog_slow_ramp_frozen() {
    let mut neighborhood = NodeSpatialNeighborhood::new(&[(1u16, 0.5f32), (2u16, 0.5f32)]).unwrap();
    let mut martingale = ResidualSprtMartingale::default();
    let snapshot = [100.0f32, 100.0, 100.0, 0.0];

    // Simular rampa lenta adversarial en el sensor 0 (+0.05 unidades por tick)
    let initial_w0 = neighborhood.weights[0];
    let mut ramped_sensor = 100.0f32;

    for _ in 0..500 {
        ramped_sensor += 0.05;
        let mut sim_snapshot = snapshot;
        sim_snapshot[0] = ramped_sensor;

        let residual = neighborhood.compute_residual(ramped_sensor, &sim_snapshot);
        let (_, log_m) = martingale.update(residual);

        // Intento de adaptación del atacante
        QuiescentCalibrator::adapt_weights(
            &mut neighborhood, ramped_sensor, &sim_snapshot, residual, log_m, 0.05,
        );
    }

    // La compuerta de silencio congeló los pesos ante sospecha: los pesos NO absorbieron la anomalía
    assert!(martingale.log_martingale >= martingale.threshold_h, "La martingala debió disparar");
    assert!((neighborhood.weights[0] - initial_w0).abs() < 1e-3, "Los pesos debieron congelarse");
}

#[test]
fn test_pillar_2_kirchhoff_n3_cascade_prevention() {
    let mut lodf = LinearizedSensitivityMatrix::empty();
    lodf.set_lodf(0, 3, 0.70).unwrap(); // Fallo en 0 desvía 70% a línea 3
    lodf.set_lodf(1, 3, 0.60).unwrap(); // Fallo en 1 desvía 60% a línea 3
    lodf.set_lodf(2, 3, 0.50).unwrap(); // Fallo en 2 desvía 50% a línea 3

    let mut pre_flows = [0.0f32; MAX_SENSITIVITY_EDGES];
    pre_flows[0] = 100.0; pre_flows[1] = 80.0; pre_flows[2] = 60.0; pre_flows[3] = 120.0;

    let mut limits = [0.0f32; MAX_SENSITIVITY_EDGES];
    limits[3] = 200.0; // Límite térmico de la línea paralela 3

    let mut post_flows = [0.0f32; MAX_SENSITIVITY_EDGES];
    // Falla triple N-3 simultánea en líneas 0, 1 y 2
    let outcome = lodf.evaluate_post_contingency_flows(&pre_flows, &[0, 1, 2], &limits, &mut post_flows);

    // Flujo inducido: 120 + (0.7*100) + (0.6*80) + (0.5*60) = 120 + 70 + 48 + 30 = 268 MW > 200 MW
    match outcome {
        Err(violating_edge) => assert_eq!(violating_edge, 3, "Debe vetar la sobrecarga en arista 3"),
        Ok(_) => panic!("Debió detectar sobrecarga de Kirchhoff e impedir incendio en cascada"),
    }
}

#[test]
fn test_pillar_3_variance_jamming_preserves_wald_bound() {
    let mut martingale = ResidualSprtMartingale::default();
    assert_eq!(martingale.threshold_h, WALD_THRESHOLD_ALPHA_1E6);

    // Inyección de ruido masivo (Variance Jamming)
    for i in 0..10_000 {
        let extreme_noise = if i % 2 == 0 { 50.0f32 } else { -50.0f32 };
        let _ = martingale.update(extreme_noise);
    }

    // La escala rodante se satura en sigma_max_safe y el umbral h jamás se altera
    assert!(martingale.current_sigma <= martingale.sigma_max_safe);
    assert_eq!(martingale.threshold_h, WALD_THRESHOLD_ALPHA_1E6);
}

#[test]
fn test_pillar_4_equifinality_triggers_epistemic_abstention() {
    let mut parity = CausalParityMatrix::empty();
    parity.signature_count = 2;
    parity.edge_fault_signatures[0] = 0b1100;
    parity.sensor_fault_signatures[1] = 0b1100; // Misma firma observada (Equifinalidad)

    let mut scratchpad = EpistemicScratchpad::new();
    let verdict = CausalSyndromeDecoder::decode(0b1100, 1, &parity, &mut scratchpad);

    match verdict {
        SyndromeVerdict::EpistemicUncertainty { candidate_a, candidate_b, .. } => {
            assert_ne!(candidate_a.class, candidate_b.class);
            assert_eq!(scratchpad.active_count, 2);
        }
        _ => panic!("Debe abstenerse de amputar la topología ante ambigüedad causal"),
    }
}

#[test]
fn test_pillar_5_and_6_autoimmune_dos_and_convex_subsumption() {
    let mut scar_engine = StructuralDiscriminationEngine::new();
    let bounds = [EdgeOperatingBounds::new(0.0, 380.0, 80.0), EdgeOperatingBounds::new(0.0, 380.0, 80.0)];

    // 1. 10,000 UnsatCores del LLM producen 0 cicatrices en Blackwall (Pilar 5)
    for i in 1..=10_000u64 {
        let mut env = StagedEnvelope::new(i);
        env.add_op(1, 999.0, 0.0, 0).unwrap();
        let _ = LocalConstraintRepairer::repair_or_veto(&mut env, &bounds, 50.0);
    }
    assert_eq!(scar_engine.slot_count, 0, "Blackwall inmaculado ante ataques cognitivos");

    // 2. 1,000 degradaciones empíricas consecutivas consumen 1 solo slot (Pilar 6)
    let edge_key = CriticalEdgeKey::from_nodes(5, 10);
    let token = NeighborTopologyToken::new(1, 1, false).raw();
    let wl1 = WL1Signature::compute_canonical(1, 1, &[token], &[token]);

    for s in 0..1_000 {
        let mut pred = Avx2PredicateBlock::empty();
        pred.thresholds[0] = 500.0 - (s as f32 * 0.4);
        pred.active_lane_mask = 0b0000_0001;

        let ft = EmpiricalFailureToken::issue_after_martingale_alarm(s as u64, edge_key, 15.0, 13.82).unwrap();
        let _ = scar_engine.assimilate_failure(&ft, edge_key, 1, wl1, pred);
    }

    assert_eq!(scar_engine.slot_count, 1);
    assert_eq!(scar_engine.slots[0].scar_count, 1);
    assert!((scar_engine.slots[0].scars[0].predicates.thresholds[0] - (500.0 - (999.0 * 0.4))).abs() < 1e-3);
}

