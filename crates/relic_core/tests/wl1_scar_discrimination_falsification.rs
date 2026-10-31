//! # WL-1 Canonical Signature & AVX2 Scar Discrimination Falsification Test Suite

use relic_core::blackwall_scars::{
    Avx2PredicateBlock, CriticalEdgeKey, EmpiricalFailureToken, NeighborTopologyToken,
    StructuralDiscriminationEngine, WL1Signature,
};
use relic_core::hash::sha256_hex;
use relic_core::repair::{
    EdgeOperatingBounds, EnvelopeStatus, LocalConstraintRepairer, RepairOutcome, StagedEnvelope,
    ViolationType,
};
use std::time::Instant;

#[test]
fn test_wl1_scar_discrimination_and_avx2_rete_engine() {
    let mut engine = StructuralDiscriminationEngine::new();
    let edge_key = CriticalEdgeKey::from_nodes(10, 20);

    let token_gen = NeighborTopologyToken::new(1, 3, false).raw();
    let token_sub = NeighborTopologyToken::new(2, 4, false).raw();
    let token_tx = NeighborTopologyToken::new(3, 2, false).raw();

    let u_orig = [token_gen, token_sub, token_tx];
    let v_orig = [token_sub, token_gen];
    let sig_orig = WL1Signature::compute_canonical(1, 2, &u_orig, &v_orig);

    // Caso 1: Inmunidad al Efecto Avalancha y Permutación de Nodos
    {
        let mut thresh = [f32::INFINITY; 8];
        thresh[0] = 400.0;
        let pred = Avx2PredicateBlock::new(thresh, 0b0000_0001);
        let token = EmpiricalFailureToken::test_token(1, edge_key);
        assert!(!engine.assimilate_failure(&token, edge_key, 1, sig_orig, pred).unwrap());

        let u_perm = [token_tx, token_gen, token_sub];
        let v_perm = [token_gen, token_sub];
        let sig_perm = WL1Signature::compute_canonical(1, 2, &u_perm, &v_perm);
        assert!(sig_orig.matches_simd(&sig_perm));
        assert_eq!(sig_orig.0, sig_perm.0);

        let h_before = sha256_hex(b"graph_with_1000_nodes");
        let h_after = sha256_hex(b"graph_with_500_nodes");
        assert_ne!(h_before, h_after);

        let sig_remote = WL1Signature::compute_canonical(1, 2, &u_perm, &v_perm);
        assert!(sig_orig.matches_simd(&sig_remote));

        let telem_violating = [420.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mask = engine.evaluate_edge_scars(edge_key, &sig_remote, &telem_violating);
        assert_ne!(mask & 0b0000_0010, 0);

        let telem_normal = [350.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(engine.evaluate_edge_scars(edge_key, &sig_remote, &telem_normal), 0);
    }

    // Caso 2: Integración con LocalConstraintRepairer
    {
        let mut bounds = [
            EdgeOperatingBounds::new(0.0, 500.0, 100.0),
            EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        ];
        let telem_violating = [450.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        engine.sync_operating_bounds(edge_key, &sig_orig, &telem_violating, &mut bounds[1]);
        assert_ne!(bounds[1].active_scar_mask, 0);

        let mut env = StagedEnvelope::new(777);
        env.add_op(1, 350.0, 300.0, 0).unwrap();
        let outcome = LocalConstraintRepairer::repair_or_veto(&mut env, &bounds, 1000.0);
        match outcome {
            RepairOutcome::VetoedIrreparable { core } => {
                assert_eq!(env.status, EnvelopeStatus::Vetoed);
                assert!(core.violations.iter().any(|v| v.violation_type == ViolationType::ScarInvariantViolated));
            }
            _ => panic!("Se esperaba VetoedIrreparable"),
        }
    }

    // Caso 3: Subsunción contra Engram Bloat
    {
        let edge_b = CriticalEdgeKey::from_nodes(50, 60);
        let u_n = [NeighborTopologyToken::new(1, 2, false).raw()];
        let v_n = [NeighborTopologyToken::new(2, 2, false).raw()];
        let sig = WL1Signature::compute_canonical(1, 2, &u_n, &v_n);

        let thresholds = [450.0, 440.0, 430.0, 420.0, 410.0, 400.0, 390.0, 385.0, 380.0, 375.0];
        let token_b = EmpiricalFailureToken::test_token(2, edge_b);
        for (i, &t) in thresholds.iter().enumerate() {
            let mut pred = Avx2PredicateBlock::empty();
            pred.thresholds[0] = t;
            pred.active_lane_mask = 0b0000_0001;
            let subsumed = engine.assimilate_failure(&token_b, edge_b, 5, sig, pred).unwrap();
            assert_eq!(subsumed, i > 0);
        }

        let slot = engine.slots.iter().find(|s| s.occupied && s.edge_key == edge_b).unwrap();
        assert_eq!(slot.scar_count, 1);
        assert_eq!(slot.scars[0].predicates.thresholds[0], 375.0);
    }

    // Caso 4: Benchmark de Latencia (< 1.5 µs)
    {
        const BENCH_ROUNDS: usize = 100_000;
        let u_n = [101u16, 102, 103];
        let v_n = [201u16, 202];
        let telem = [410.0f32, 20.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut test_bounds = EdgeOperatingBounds::new(0.0, 500.0, 100.0);

        let start = Instant::now();
        for i in 0..BENCH_ROUNDS {
            let sig = WL1Signature::compute_canonical((i % 4) as u8, ((i + 1) % 4) as u8, &u_n, &v_n);
            let mask = engine.evaluate_edge_scars(edge_key, &sig, &telem);
            test_bounds.active_scar_mask = mask;
            std::hint::black_box(test_bounds.active_scar_mask);
        }
        let avg_micros = (start.elapsed().as_nanos() as f64 / BENCH_ROUNDS as f64) / 1000.0;
        assert!(avg_micros < 1.5, "Latencia excede 1.5 µs: {:.4} µs", avg_micros);
    }
}
