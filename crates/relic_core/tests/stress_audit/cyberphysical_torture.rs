//! # Cyberphysical Multi-Fault Torture Scenario
//!
//! Falsificación empírica ante saltos de régimen, fallas en cascada N-1 -> N-2 -> N-3,
//! resistencia a permutación canónica WL-1 y Safe-Halt sin fugas a DMA.

use relic_core::anomaly::topology_coupling::{NodeSpatialNeighborhood, ResidualSprtMartingale};
use relic_core::blackwall_scars::{NeighborTopologyToken, StructuralDiscriminationEngine, WL1Signature};
use relic_core::contingency_bfs::{
    BitAdjacencyGraph, ContingencyOutcome, PreCertifiedContingencyLibrary, PreCertifiedTopology,
    SwitchAction, SwitchActionType,
};
use relic_core::repair::{
    EdgeOperatingBounds, LocalConstraintRepairer, RepairOutcome, StagedEnvelope,
};
use relic_core::tactical_dispatch::{
    ReflexTickInput, StagedOrderRingBuffer, TacticalDecision, TacticalKernelContext, TacticalReflexKernel,
    VetoReason,
};

pub fn run_cyberphysical_multi_fault_torture() {
    let sources = [0u8, 10, 20, 30];
    let sinks_mask = [1u64 << 1, 1u64 << 11, 1u64 << 21, 1u64 << 31];

    let mut graph = BitAdjacencyGraph::new();
    graph.add_bidirectional_edge(0, 1);
    graph.add_bidirectional_edge(0, 2);
    graph.add_bidirectional_edge(0, 3);
    graph.add_bidirectional_edge(10, 11);
    graph.add_bidirectional_edge(20, 21);
    graph.add_bidirectional_edge(30, 31);

    let mut bounds = [
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 380.0, 80.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 400.0, 70.0),
        EdgeOperatingBounds::new(0.0, 400.0, 70.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
    ];

    let mut contingency_lib = PreCertifiedContingencyLibrary::new();
    let topo_n1 = PreCertifiedTopology {
        contingency_id: 201, failed_edge_id: 1,
        actions: [SwitchAction { u: 2, v: 1, edge_id: 5, action: SwitchActionType::CloseBackup, new_max_capacity: 400.0 }, SwitchAction::empty(), SwitchAction::empty(), SwitchAction::empty()],
        action_count: 1, thermal_safety_margin: 0.90, _padding: [0u8; 1],
    };
    contingency_lib.register_contingency(topo_n1).unwrap();

    let topo_n2 = PreCertifiedTopology {
        contingency_id: 202, failed_edge_id: 5,
        actions: [SwitchAction { u: 3, v: 1, edge_id: 6, action: SwitchActionType::CloseBackup, new_max_capacity: 400.0 }, SwitchAction::empty(), SwitchAction::empty(), SwitchAction::empty()],
        action_count: 1, thermal_safety_margin: 0.85, _padding: [0u8; 1],
    };
    contingency_lib.register_contingency(topo_n2).unwrap();

    let mut scar_engine = StructuralDiscriminationEngine::new();
    let ring_buffer = StagedOrderRingBuffer::new();
    let nh_node0 = NodeSpatialNeighborhood::new(&[(1, 0.5), (2, 0.5)]).unwrap();
    let mut martingale = ResidualSprtMartingale::new(1.0, 0.5, 13.82);

    let token_gen = NeighborTopologyToken::new(1, 3, false).raw();
    let token_sub = NeighborTopologyToken::new(2, 4, false).raw();
    let token_tx = NeighborTopologyToken::new(3, 2, false).raw();
    let u_neighbors = [token_gen, token_sub];
    let v_neighbors = [token_sub, token_tx];
    let wl1_sig_edge1 = WL1Signature::compute_canonical(1, 1, &u_neighbors, &v_neighbors);

    // 1. Régimen global y reparación in-place
    let regime_levels = [100.0f32, 400.0, 900.0, 200.0];
    for i in 0..1000u64 {
        let base_val = regime_levels[((i / 250) % 4) as usize];
        let snapshot = [base_val, base_val, base_val, base_val];
        let mut env = StagedEnvelope::new(1000 + i);
        env.add_op(1, 380.0 + ((i % 50) as f64) + 10.0, 300.0, 0).unwrap();

        let repair_res = ring_buffer.try_stage_from_cognitive(env, i + 100, (0, 1, 1), &bounds, 1000.0).unwrap();
        assert!(matches!(repair_res, RepairOutcome::RepairedInPlace { .. }));

        let mut ctx = TacticalKernelContext { martingale: &mut martingale, scar_engine: &mut scar_engine, contingency_lib: &contingency_lib, graph: &mut graph, bounds: &mut bounds, ring_buffer: &ring_buffer };
        let input = ReflexTickInput { current_tick: i, target_edge: (0, 1, 1), monitored_node: 0, sensor_snapshot: &snapshot, neighborhood: &nh_node0, wl1_sig: &wl1_sig_edge1, sources: &sources, sinks_mask: &sinks_mask };
        let decision = TacticalReflexKernel::execute_tick(&mut ctx, &input);

        match decision {
            TacticalDecision::TransmitDMA { envelope, .. } => {
                assert_eq!(envelope.ops_slice()[0].requested_val, 380.0);
            }
            other => panic!("Fallo en transmisión DMA: {:?}", other),
        }
    }
    assert_eq!(martingale.log_martingale, 0.0);

    // 2. Falla N-1 y conmutación
    let mut env2 = StagedEnvelope::new(2001);
    env2.add_op(1, 350.0, 300.0, 0).unwrap();
    ring_buffer.try_stage_from_cognitive(env2, 2000, (0, 1, 1), &bounds, 1000.0).unwrap();
    let fault_snapshot = [120.0f32, 300.0, 300.0, 300.0];

    let mut ctx = TacticalKernelContext { martingale: &mut martingale, scar_engine: &mut scar_engine, contingency_lib: &contingency_lib, graph: &mut graph, bounds: &mut bounds, ring_buffer: &ring_buffer };
    let input = ReflexTickInput { current_tick: 1001, target_edge: (0, 1, 1), monitored_node: 0, sensor_snapshot: &fault_snapshot, neighborhood: &nh_node0, wl1_sig: &wl1_sig_edge1, sources: &sources, sinks_mask: &sinks_mask };
    let decision = TacticalReflexKernel::execute_tick(&mut ctx, &input);

    match decision {
        TacticalDecision::VetoedByCircuitBreaker { reason, contingency, scar_assimilated } => {
            assert_eq!(reason, VetoReason::MartingaleAnomalyTriggered);
            assert!(scar_assimilated);
            assert!(bounds[1].quarantined);
            assert_eq!(contingency, ContingencyOutcome::Applied { contingency_id: 201, backup_edge_id: 5, thermal_safety_margin: 0.90 });
        }
        other => panic!("Se esperaba veto de circuit breaker: {:?}", other),
    }

    // 3. Invarianza WL-1 ante permutación
    let permuted_u = [token_sub, token_gen];
    let permuted_v = [token_tx, token_sub];
    let wl1_sig_permuted = WL1Signature::compute_canonical(1, 1, &permuted_u, &permuted_v);
    assert_eq!(wl1_sig_edge1, wl1_sig_permuted);

    let mut adversarial_env = StagedEnvelope::new(3001);
    adversarial_env.add_op(1, 250.0, 0.0, 0).unwrap();
    let outcome = LocalConstraintRepairer::repair_or_veto(&mut adversarial_env, &bounds, 1000.0);
    assert!(matches!(outcome, RepairOutcome::VetoedIrreparable { .. }));

    // 4. Falla N-2 y conmutación a arista 6
    let nh_node2 = NodeSpatialNeighborhood::new(&[(0, 0.5), (1, 0.5)]).unwrap();
    let wl1_sig_edge5 = WL1Signature::compute_canonical(1, 1, &[token_gen], &[token_sub]);
    let mut martingale_node2 = ResidualSprtMartingale::new(1.0, 0.5, 13.82);
    let mut env4 = StagedEnvelope::new(4001);
    env4.add_op(5, 360.0, 300.0, 0).unwrap();
    ring_buffer.try_stage_from_cognitive(env4, 3000, (2, 1, 5), &bounds, 1000.0).unwrap();

    let fault2 = [300.0f32, 300.0, 100.0, 300.0];
    let mut ctx4 = TacticalKernelContext { martingale: &mut martingale_node2, scar_engine: &mut scar_engine, contingency_lib: &contingency_lib, graph: &mut graph, bounds: &mut bounds, ring_buffer: &ring_buffer };
    let input4 = ReflexTickInput { current_tick: 2001, target_edge: (2, 1, 5), monitored_node: 2, sensor_snapshot: &fault2, neighborhood: &nh_node2, wl1_sig: &wl1_sig_edge5, sources: &sources, sinks_mask: &sinks_mask };
    let dec4 = TacticalReflexKernel::execute_tick(&mut ctx4, &input4);
    assert!(matches!(dec4, TacticalDecision::VetoedByCircuitBreaker { contingency: ContingencyOutcome::Applied { contingency_id: 202, backup_edge_id: 6, .. }, .. }));

    // 5. Colapso N-3 y Safe-Halt sin fugas DMA
    let nh_node3 = NodeSpatialNeighborhood::new(&[(0, 0.5), (1, 0.5)]).unwrap();
    let wl1_sig_edge6 = WL1Signature::compute_canonical(1, 1, &[token_gen], &[token_sub]);
    let mut martingale_node3 = ResidualSprtMartingale::new(1.0, 0.5, 13.82);
    let mut env5 = StagedEnvelope::new(5001);
    env5.add_op(6, 320.0, 300.0, 0).unwrap();
    ring_buffer.try_stage_from_cognitive(env5, 4000, (3, 1, 6), &bounds, 1000.0).unwrap();

    let fault3 = [300.0f32, 300.0, 300.0, 80.0];
    let mut ctx5 = TacticalKernelContext { martingale: &mut martingale_node3, scar_engine: &mut scar_engine, contingency_lib: &contingency_lib, graph: &mut graph, bounds: &mut bounds, ring_buffer: &ring_buffer };
    let input5 = ReflexTickInput { current_tick: 3001, target_edge: (3, 1, 6), monitored_node: 3, sensor_snapshot: &fault3, neighborhood: &nh_node3, wl1_sig: &wl1_sig_edge6, sources: &sources, sinks_mask: &sinks_mask };
    let dec5 = TacticalReflexKernel::execute_tick(&mut ctx5, &input5);
    assert_eq!(dec5, TacticalDecision::VetoedByCircuitBreaker { reason: VetoReason::MartingaleAnomalyTriggered, contingency: ContingencyOutcome::NoViableTopologyFound, scar_assimilated: true });
    assert!(ring_buffer.is_empty());
}

