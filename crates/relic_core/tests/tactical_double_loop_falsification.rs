//! # Tactical Double-Loop, Lock-Free SPSC & Circuit Breaker Falsification Suite

use relic_core::anomaly::topology_coupling::{NodeSpatialNeighborhood, ResidualSprtMartingale};
use relic_core::blackwall_scars::{NeighborTopologyToken, StructuralDiscriminationEngine, WL1Signature};
use relic_core::contingency_bfs::{
    BitAdjacencyGraph, ContingencyOutcome, PreCertifiedContingencyLibrary, PreCertifiedTopology,
    SwitchAction, SwitchActionType,
};
use relic_core::repair::{
    EdgeOperatingBounds, LocalConstraintRepairer, RepairOutcome, StagedEnvelope, ViolationType,
};
use relic_core::tactical_dispatch::{
    CircuitBreaker, StagedOrderRingBuffer, StagedPolicySlot, TacticalDecision,
    TacticalReflexOrchestrator, VetoReason,
};
use std::time::Instant;

#[test]
fn test_tactical_double_loop_orchestrator_and_circuit_breaker() {
    let target_edge = (0u8, 1u8, 1u32);
    let sources = [0u8, 10, 20, 30];
    let sinks_mask = [1u64 << 1, 1u64 << 11, 1u64 << 21, 1u64 << 31];

    let mut graph = BitAdjacencyGraph::new();
    graph.add_bidirectional_edge(0, 1);
    graph.add_bidirectional_edge(10, 11);
    graph.add_bidirectional_edge(20, 21);
    graph.add_bidirectional_edge(30, 31);
    graph.add_bidirectional_edge(0, 2);

    let mut bounds = [
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 380.0, 80.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 400.0, 70.0),
    ];

    let mut contingency_lib = PreCertifiedContingencyLibrary::new();
    let topo_backup = PreCertifiedTopology {
        contingency_id: 202, failed_edge_id: 1,
        actions: [SwitchAction { u: 2, v: 1, edge_id: 5, action: SwitchActionType::CloseBackup, new_max_capacity: 400.0 }, SwitchAction::empty(), SwitchAction::empty(), SwitchAction::empty()],
        action_count: 1, thermal_safety_margin: 0.85, _padding: [0u8; 1],
    };
    contingency_lib.register_contingency(topo_backup).unwrap();

    let mut scar_engine = StructuralDiscriminationEngine::new();
    let nh_node0 = NodeSpatialNeighborhood::new(&[(1, 0.5), (2, 0.5)]).unwrap();
    let mut martingale = ResidualSprtMartingale::new(1.0, 0.5, 13.82);

    let token_sub = NeighborTopologyToken::new(2, 3, false).raw();
    let wl1_sig = WL1Signature::compute_canonical(1, 1, &[token_sub], &[token_sub]);
    let ring_buffer = StagedOrderRingBuffer::new();

    // Caso 1: Flujo Nominal con Reparación Cognitiva Previa
    {
        let mut env_draft = StagedEnvelope::new(1001);
        env_draft.add_op(1, 420.0, 300.0, 0).unwrap();
        let outcome = ring_buffer.try_stage_from_cognitive(env_draft, 1000, target_edge, &bounds, 1000.0).unwrap();
        assert!(matches!(outcome, RepairOutcome::RepairedInPlace { .. }));
        assert_eq!(ring_buffer.len(), 1);

        let sensor_snapshot_regime = [300.0f32, 300.0, 300.0, 0.0];
        let decision = TacticalReflexOrchestrator::execute_reflex_tick(
            50, target_edge, 0, &sensor_snapshot_regime, &nh_node0, &mut martingale, &mut scar_engine,
            &wl1_sig, &contingency_lib, &mut graph, &mut bounds, &sources, &sinks_mask, &ring_buffer,
        );
        match decision {
            TacticalDecision::TransmitDMA { envelope, .. } => {
                assert_eq!(envelope.envelope_id, 1001);
                assert_eq!(envelope.ops_slice()[0].requested_val, 380.0);
            }
            _ => panic!("Se esperaba TransmitDMA"),
        }
        assert_eq!(ring_buffer.len(), 0);
    }

    // Caso 2: Intercepción TOCTOU (< 1 µs) + Conmutación
    {
        let mut env_staged = StagedEnvelope::new(2002);
        env_staged.add_op(1, 350.0, 300.0, 0).unwrap();
        ring_buffer.try_stage_from_cognitive(env_staged, 2000, target_edge, &bounds, 1000.0).unwrap();

        let sensor_snapshot_fault = [120.0f32, 300.0, 300.0, 0.0];
        let decision = TacticalReflexOrchestrator::execute_reflex_tick(
            60, target_edge, 0, &sensor_snapshot_fault, &nh_node0, &mut martingale, &mut scar_engine,
            &wl1_sig, &contingency_lib, &mut graph, &mut bounds, &sources, &sinks_mask, &ring_buffer,
        );
        match decision {
            TacticalDecision::VetoedByCircuitBreaker { reason, contingency, scar_assimilated } => {
                assert_eq!(reason, VetoReason::MartingaleAnomalyTriggered);
                assert!(scar_assimilated);
                assert!(bounds[1].quarantined);
                assert_eq!(contingency, ContingencyOutcome::Applied { contingency_id: 202, backup_edge_id: 5, thermal_safety_margin: 0.85 });
            }
            _ => panic!("Se esperaba VetoedByCircuitBreaker"),
        }
    }

    // Caso 3: Memoria Auto-Inmune Permanente
    {
        let mut stubborn_env = StagedEnvelope::new(3003);
        stubborn_env.add_op(1, 300.0, 0.0, 0).unwrap();
        let outcome = LocalConstraintRepairer::repair_or_veto(&mut stubborn_env, &bounds, 1000.0);
        match outcome {
            RepairOutcome::VetoedIrreparable { core } => {
                assert!(core.violations.iter().any(|v| v.violation_type == ViolationType::EdgeQuarantined || v.violation_type == ViolationType::ScarInvariantViolated));
            }
            _ => panic!("Debió ser vetada en la puerta cognitiva"),
        }
    }

    // Caso 4: Benchmark de Latencia (< 1 µs Disyuntor / <= 351.5 µs Pipeline)
    {
        const BENCH_ROUNDS: usize = 100_000;
        let test_slot = StagedPolicySlot { envelope: StagedEnvelope::new(777), valid_until_tick: 500_000, target_critical_edge: (0, 1, 1), _padding: [0u8; 2] };

        let start_cb = Instant::now();
        for tick in 0..BENCH_ROUNDS as u64 {
            let res = CircuitBreaker::evaluate_last_mile(&test_slot, tick, &bounds);
            let _ = std::hint::black_box(res);
        }
        let avg_cb_micros = (start_cb.elapsed().as_nanos() as f64 / BENCH_ROUNDS as f64) / 1000.0;
        assert!(avg_cb_micros < 1.0, "Disyuntor excede 1.0 µs: {:.4} µs", avg_cb_micros);

        let clean_snapshot = [300.0f32, 300.0, 300.0, 0.0];
        let mut clean_martingale = ResidualSprtMartingale::new(1.0, 0.5, 13.82);
        let mut clean_bounds = [
            EdgeOperatingBounds::new(0.0, 500.0, 100.0), EdgeOperatingBounds::new(0.0, 500.0, 100.0),
            EdgeOperatingBounds::new(0.0, 500.0, 100.0), EdgeOperatingBounds::new(0.0, 500.0, 100.0),
            EdgeOperatingBounds::new(0.0, 500.0, 100.0), EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        ];

        let start_pipeline = Instant::now();
        for tick in 0..BENCH_ROUNDS as u64 {
            let decision = TacticalReflexOrchestrator::execute_reflex_tick(
                tick, target_edge, 0, &clean_snapshot, &nh_node0, &mut clean_martingale, &mut scar_engine,
                &wl1_sig, &contingency_lib, &mut graph, &mut clean_bounds, &sources, &sinks_mask, &ring_buffer,
            );
            std::hint::black_box(decision);
        }
        let avg_pipe_micros = (start_pipeline.elapsed().as_nanos() as f64 / BENCH_ROUNDS as f64) / 1000.0;
        assert!(avg_pipe_micros <= 351.5, "Pipeline excede 351.5 µs: {:.4} µs", avg_pipe_micros);
    }
}
