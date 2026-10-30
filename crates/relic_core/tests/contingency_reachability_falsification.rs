//! # Causal Reachability MS-BFS & N-1 Contingency Falsification Test Suite

use relic_core::contingency_bfs::{
    BitAdjacencyGraph, CausalReachabilityFilter, ContingencyOutcome, PreCertifiedContingencyLibrary,
    PreCertifiedTopology, SwitchAction, SwitchActionType,
};
use relic_core::repair::EdgeOperatingBounds;
use std::time::Instant;

#[test]
fn test_contingency_reachability_and_ms_bfs_avx2_pipeline() {
    let mut base_graph = BitAdjacencyGraph::new();
    base_graph.add_bidirectional_edge(0, 1);
    base_graph.add_bidirectional_edge(10, 11);
    base_graph.add_bidirectional_edge(20, 21);
    base_graph.add_bidirectional_edge(30, 31);
    base_graph.add_bidirectional_edge(0, 2);
    base_graph.add_bidirectional_edge(0, 3);

    let sources = [0u8, 10, 20, 30];
    let sinks_mask = [1u64 << 1, 1u64 << 11, 1u64 << 21, 1u64 << 31];
    assert_eq!(CausalReachabilityFilter::verify_critical_pairs(&base_graph, &sources, &sinks_mask), 0b1111);

    let bounds = [
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 450.0, 80.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 380.0, 60.0),
        EdgeOperatingBounds::new(0.0, 350.0, 50.0),
    ];

    let mut library = PreCertifiedContingencyLibrary::new();
    let topo_disconnected = PreCertifiedTopology {
        contingency_id: 101, failed_edge_id: 1,
        actions: [SwitchAction { u: 40, v: 41, edge_id: 99, action: SwitchActionType::CloseBackup, new_max_capacity: 400.0 }, SwitchAction::empty(), SwitchAction::empty(), SwitchAction::empty()],
        action_count: 1, thermal_safety_margin: 0.95, _padding: [0u8; 1],
    };
    library.register_contingency(topo_disconnected).unwrap();

    let topo_b = PreCertifiedTopology {
        contingency_id: 102, failed_edge_id: 1,
        actions: [SwitchAction { u: 2, v: 1, edge_id: 5, action: SwitchActionType::CloseBackup, new_max_capacity: 380.0 }, SwitchAction::empty(), SwitchAction::empty(), SwitchAction::empty()],
        action_count: 1, thermal_safety_margin: 0.85, _padding: [0u8; 1],
    };
    library.register_contingency(topo_b).unwrap();

    let topo_c = PreCertifiedTopology {
        contingency_id: 103, failed_edge_id: 1,
        actions: [SwitchAction { u: 3, v: 1, edge_id: 6, action: SwitchActionType::CloseBackup, new_max_capacity: 350.0 }, SwitchAction::empty(), SwitchAction::empty(), SwitchAction::empty()],
        action_count: 1, thermal_safety_margin: 0.70, _padding: [0u8; 1],
    };
    library.register_contingency(topo_c).unwrap();

    // Caso 1: Poda de contingencia desconectada y selección óptima
    {
        let mut sim_graph = base_graph;
        let mut sim_bounds = bounds;
        let outcome = library.select_and_apply_contingency(&mut sim_graph, &mut sim_bounds, (0, 1, 1), &sources, &sinks_mask);
        match outcome {
            ContingencyOutcome::Applied { contingency_id, thermal_safety_margin, backup_edge_id } => {
                assert_eq!(contingency_id, 102);
                assert_eq!(backup_edge_id, 5);
                assert_eq!(thermal_safety_margin, 0.85);
                assert!(!sim_graph.is_connected(0, 1));
                assert!(sim_graph.is_connected(2, 1));
                assert!(sim_bounds[1].quarantined);
                assert!(!sim_bounds[5].quarantined);
                assert_eq!(CausalReachabilityFilter::verify_critical_pairs(&sim_graph, &sources, &sinks_mask), 0b1111);
            }
            ContingencyOutcome::NoViableTopologyFound => panic!("Debió seleccionar contingencia 102"),
        }
    }

    // Caso 2: Rechazo por Cicatriz Activa
    {
        let mut sim_graph = base_graph;
        let mut sim_bounds = bounds;
        sim_bounds[5].active_scar_mask = 0b0000_0001;
        let outcome = library.select_and_apply_contingency(&mut sim_graph, &mut sim_bounds, (0, 1, 1), &sources, &sinks_mask);
        match outcome {
            ContingencyOutcome::Applied { contingency_id, backup_edge_id, .. } => {
                assert_eq!(contingency_id, 103);
                assert_eq!(backup_edge_id, 6);
                assert!(sim_bounds[5].active_scar_mask != 0);
                assert!(!sim_bounds[6].quarantined);
            }
            ContingencyOutcome::NoViableTopologyFound => panic!("Debió conmutar a topo_c"),
        }
    }

    // Caso 3: Agotamiento de contingencias y Safe-Halt
    {
        let mut sim_graph = base_graph;
        let mut sim_bounds = bounds;
        sim_bounds[5].active_scar_mask = 0b0000_0001;
        sim_bounds[6].active_scar_mask = 0b0000_0001;
        let outcome = library.select_and_apply_contingency(&mut sim_graph, &mut sim_bounds, (0, 1, 1), &sources, &sinks_mask);
        assert_eq!(outcome, ContingencyOutcome::NoViableTopologyFound);
    }

    // Caso 4: Benchmark de Latencia (<= 310 µs)
    {
        const BENCH_ROUNDS: usize = 100_000;
        let start = Instant::now();
        for _ in 0..BENCH_ROUNDS {
            let res = CausalReachabilityFilter::verify_critical_pairs(&base_graph, &sources, &sinks_mask);
            std::hint::black_box(res);
        }
        let avg_micros = (start.elapsed().as_nanos() as f64 / BENCH_ROUNDS as f64) / 1000.0;
        assert!(avg_micros <= 310.0, "Latencia MS-BFS excede presupuesto de 310 µs: {:.4} µs", avg_micros);
    }
}
