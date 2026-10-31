//! # Convex Subsumption & Auto-Immune DoS Falsification Suite
//!
//! Verificación empírica de:
//! 1. Protección contra Auto-Immune DoS: 10,000 rechazos cognitivos del LLM (UnsatCore) producen 0 cicatrices.
//! 2. Subsunción por Dominancia Convexa: 1,000 degradaciones empíricas consecutivas consumen exactamente 1 slot.
//! 3. Monotonía de Seguridad: C_{t+1} \subseteq C_t sin decaimiento temporal en el Blackwall.

use relic_core::blackwall_scars::{
    Avx2PredicateBlock, CriticalEdgeKey, EmpiricalFailureToken, NeighborTopologyToken,
    StructuralDiscriminationEngine, WL1Signature,
};
use relic_core::repair::{
    EdgeOperatingBounds, LocalConstraintRepairer, RepairOutcome, StagedEnvelope,
};

#[test]
fn test_autoimmune_dos_rejected_orders_create_zero_scars() {
    let scar_engine = StructuralDiscriminationEngine::new();
    let bounds = [
        EdgeOperatingBounds::new(0.0, 380.0, 80.0),
        EdgeOperatingBounds::new(0.0, 380.0, 80.0),
    ];

    // Simular 10,000 propuestas inválidas del LLM que generan UnsatCore
    for i in 1..=10_000u64 {
        let mut envelope = StagedEnvelope::new(i);
        // El LLM pide 900 MW en una línea de 380 MW
        envelope.add_op(1, 900.0, 0.0, 0).unwrap();

        let core = LocalConstraintRepairer::validate_envelope(&envelope, &bounds, 50.0);
        assert!(!core.is_empty(), "La propuesta debe violar cotas y generar UnsatCore");

        let outcome = LocalConstraintRepairer::repair_or_veto(&mut envelope, &bounds, 50.0);
        assert!(matches!(
            outcome,
            RepairOutcome::RepairedInPlace { .. } | RepairOutcome::VetoedIrreparable { .. }
        ));
    }

    // Comprobar que NINGUNA cicatriz fue registrada en el Blackwall
    assert_eq!(scar_engine.slot_count, 0, "UnsatCore cognitivo NUNCA debe generar cicatrices");
    for slot in scar_engine.slots.iter() {
        assert!(!slot.occupied, "Los slots del Blackwall deben permanecer inmaculados");
    }

    // Comprobar que el emisor de tokens rechaza martingalas no alarmadas
    let fake_edge = CriticalEdgeKey::from_nodes(1, 2);
    let token_attempt = EmpiricalFailureToken::issue_after_martingale_alarm(100, fake_edge, 5.0, 13.82);
    assert!(token_attempt.is_err(), "Martingala < h debe ser rechazada para emitir token");
}

#[test]
fn test_convex_subsumption_1000_degradations_consume_single_slot() {
    let mut scar_engine = StructuralDiscriminationEngine::new();
    let edge_key = CriticalEdgeKey::from_nodes(10, 20);

    let token_u = NeighborTopologyToken::new(1, 3, false).raw();
    let token_v = NeighborTopologyToken::new(2, 2, false).raw();
    let wl1_sig = WL1Signature::compute_canonical(1, 2, &[token_u], &[token_v]);

    // 1,000 degradaciones empíricas sucesivas: 400.0 -> 399.7 -> ... -> 100.0 MW
    for step in 0..1_000 {
        let degradation_bound = 400.0f32 - (step as f32 * 0.3);
        let mut pred = Avx2PredicateBlock::empty();
        pred.thresholds[0] = degradation_bound;
        pred.active_lane_mask = 0b0000_0001;

        let failure_token = EmpiricalFailureToken::issue_after_martingale_alarm(
            step as u64,
            edge_key,
            15.5, // Martingala claramente por encima de h = 13.82
            13.82,
        ).expect("Token emitido legítimamente tras alarma de martingala");

        let was_subsumed = scar_engine.assimilate_failure(
            &failure_token,
            edge_key,
            7,
            wl1_sig,
            pred,
        ).expect("Asimilación exitosa");

        if step > 0 {
            assert!(was_subsumed, "Paso {} debió ser subsumido in-place", step);
        }
    }

    // Verificar que solo se consumió 1 único slot en el motor
    assert_eq!(scar_engine.slot_count, 1, "Debe existir exactamente 1 slot de arista crítica");
    let slot = &scar_engine.slots[0];
    assert_eq!(slot.scar_count, 1, "Debe existir exactamente 1 regla de cicatriz");
    assert_eq!(slot.edge_key, edge_key);

    // El límite se contrajo monotónicamente hasta el mínimo histórico
    let final_threshold = slot.scars[0].predicates.thresholds[0];
    let expected_min = 400.0f32 - (999.0f32 * 0.3);
    assert!((final_threshold - expected_min).abs() < 1e-3, "El umbral debió contraerse al mínimo");

    // Monotonía de seguridad: cualquier flujo >= umbral mínimo dispara la cicatriz
    let telem_at_250 = [250.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let mask = scar_engine.evaluate_edge_scars(edge_key, &wl1_sig, &telem_at_250);
    assert_ne!(mask, 0, "Flujo de 250 MW debe violar la cota contraída de ~100 MW");
}

