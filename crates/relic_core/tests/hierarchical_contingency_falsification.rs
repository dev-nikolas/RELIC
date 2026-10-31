//! Verificación formal de conmutación de contingencias jerárquicas con rollback diferencial O(1).

use relic_core::contingency_bfs::{
    ContingencyContext, ContingencyOutcome, HierarchicalBitGraph, HierarchicalContingencyLibrary,
    HierarchicalContingencyPlan, HierarchicalNode, HierarchicalSwitchAction, SwitchActionType,
};
use relic_core::repair::EdgeOperatingBounds;

fn node(c: u8, l: u8) -> HierarchicalNode {
    HierarchicalNode::new(c, l).unwrap()
}

// Conmutación N-1 inter-isla con discriminación de cicatrices
#[test]
fn inter_island_contingency_scar_discrimination() {
    let mut g = HierarchicalBitGraph::<3>::new();
    for u in 0..4u8 { g.add_intra_cluster_edge(0, u, 10).unwrap(); }
    for v in 4..8u8 { g.add_intra_cluster_edge(1, 0, v).unwrap(); }
    g.add_intra_cluster_edge(2, 20, 30).unwrap();

    g.add_inter_cluster_bridge(1, node(0, 10), node(1, 0)).unwrap();
    g.add_inter_cluster_bridge(2, node(0, 10), node(1, 0)).unwrap();
    g.add_inter_cluster_bridge(3, node(0, 10), node(2, 20)).unwrap();
    g.add_inter_cluster_bridge(4, node(2, 30), node(1, 0)).unwrap();

    let mut bounds = [EdgeOperatingBounds::new(0.0, 100.0, 10.0); 10];
    bounds[2].active_scar_mask = 0xDEADBEEF;

    let mut lib = HierarchicalContingencyLibrary::new();
    let mut p1 = HierarchicalContingencyPlan::empty();
    p1.contingency_id = 101; p1.failed_edge_id = 1; p1.action_count = 1; p1.thermal_safety_margin = 0.9;
    p1.actions[0] = HierarchicalSwitchAction { src: node(0, 10), dst: node(1, 0), action: SwitchActionType::CloseBackup, edge_id: 2, new_max_capacity: 80.0 };
    lib.register_plan(p1).unwrap();

    let mut p2 = HierarchicalContingencyPlan::empty();
    p2.contingency_id = 102; p2.failed_edge_id = 1; p2.action_count = 2; p2.thermal_safety_margin = 0.5;
    p2.actions[0] = HierarchicalSwitchAction { src: node(0, 10), dst: node(2, 20), action: SwitchActionType::CloseBackup, edge_id: 3, new_max_capacity: 50.0 };
    p2.actions[1] = HierarchicalSwitchAction { src: node(2, 30), dst: node(1, 0), action: SwitchActionType::CloseBackup, edge_id: 4, new_max_capacity: 50.0 };
    lib.register_plan(p2).unwrap();

    g.quarantine_bridge(node(0, 10), node(2, 20)).unwrap();
    g.quarantine_bridge(node(2, 30), node(1, 0)).unwrap();

    let srcs = [node(0, 0), node(0, 1), node(0, 2), node(0, 3)];
    let sc = [1u8; 4];
    let sm = [1u64 << 4, 1u64 << 5, 1u64 << 6, 1u64 << 7];

    let ctx = ContingencyContext {
        sources: &srcs,
        sinks_c: &sc,
        sinks_m: &sm,
        sens: None,
        flows: None,
        limits: None,
    };

    let outcome = lib.select_and_apply_contingency_hierarchical(
        &mut g, &mut bounds, (node(0, 10), node(1, 0), 1), ctx,
    );

    assert_eq!(outcome, ContingencyOutcome::Applied { contingency_id: 102, thermal_safety_margin: 0.5, backup_edge_id: 4 });
    assert!(bounds[1].quarantined);
    assert_eq!(g.verify_hierarchical_pairs(&srcs, &sc, &sm), 0b1111);
}

// Invariante de reversión diferencial exacta tras descarte de candidato
#[test]
fn exact_rollback_invariant_on_candidate_rejection() {
    let mut g = HierarchicalBitGraph::<2>::new();
    g.add_intra_cluster_edge(0, 0, 1).unwrap();
    g.add_intra_cluster_edge(1, 10, 11).unwrap();
    g.add_intra_cluster_edge(1, 20, 21).unwrap();

    g.add_inter_cluster_bridge(1, node(0, 1), node(1, 20)).unwrap();
    g.add_inter_cluster_bridge(2, node(0, 1), node(1, 10)).unwrap();
    g.quarantine_bridge(node(0, 1), node(1, 10)).unwrap();

    let mut bounds = [EdgeOperatingBounds::new(0.0, 100.0, 10.0); 5];
    let mut lib = HierarchicalContingencyLibrary::new();
    let mut plan = HierarchicalContingencyPlan::empty();
    plan.contingency_id = 201; plan.failed_edge_id = 1; plan.action_count = 1; plan.thermal_safety_margin = 0.8;
    plan.actions[0] = HierarchicalSwitchAction { src: node(0, 1), dst: node(1, 10), action: SwitchActionType::CloseBackup, edge_id: 2, new_max_capacity: 50.0 };
    lib.register_plan(plan).unwrap();

    let srcs = [node(0, 0); 4];
    let sc = [1u8; 4];
    let sm = [1u64 << 21; 4];

    let ctx = ContingencyContext {
        sources: &srcs,
        sinks_c: &sc,
        sinks_m: &sm,
        sens: None,
        flows: None,
        limits: None,
    };

    let outcome = lib.select_and_apply_contingency_hierarchical(
        &mut g, &mut bounds, (node(0, 1), node(1, 20), 1), ctx,
    );

    assert_eq!(outcome, ContingencyOutcome::NoViableTopologyFound);
    assert_eq!(g.bridges.bridge_count(node(0, 1), node(1, 10)), 0);
    assert_eq!(g.bridges.bridge_count(node(0, 1), node(1, 20)), 0);
    assert!(bounds[1].quarantined);
}

// Safe-Halt determinista al agotar opciones viables
#[test]
fn deterministic_safe_halt_on_exhaustion() {
    let mut g = HierarchicalBitGraph::<1>::new();
    g.add_intra_cluster_edge(0, 0, 1).unwrap();
    let mut bounds = [EdgeOperatingBounds::new(0.0, 50.0, 5.0); 4];
    let lib = HierarchicalContingencyLibrary::new();

    let srcs = [node(0, 0); 4];
    let sc = [0u8; 4];
    let sm = [1u64 << 1; 4];

    let ctx = ContingencyContext {
        sources: &srcs,
        sinks_c: &sc,
        sinks_m: &sm,
        sens: None,
        flows: None,
        limits: None,
    };

    let outcome = lib.select_and_apply_contingency_hierarchical(
        &mut g, &mut bounds, (node(0, 0), node(0, 1), 0), ctx,
    );

    assert_eq!(outcome, ContingencyOutcome::NoViableTopologyFound);
    assert!(bounds[0].quarantined);
    assert!(!g.islands[0].is_connected(0, 1));
}

// Conmutación zero-alloc continua a escala K=16
#[test]
fn zero_alloc_k16_contingency_churn_continuous() {
    let mut g = HierarchicalBitGraph::<16>::new();
    for c in 0..16u8 { g.add_intra_cluster_edge(c, 0, 1).unwrap(); }
    g.add_inter_cluster_bridge(1, node(0, 1), node(1, 0)).unwrap();
    g.add_inter_cluster_bridge(2, node(0, 1), node(1, 0)).unwrap();
    g.quarantine_bridge(node(0, 1), node(1, 0)).unwrap();

    let mut bounds = [EdgeOperatingBounds::new(0.0, 100.0, 10.0); 5];
    let mut lib = HierarchicalContingencyLibrary::new();
    let mut plan = HierarchicalContingencyPlan::empty();
    plan.contingency_id = 401; plan.failed_edge_id = 1; plan.action_count = 1; plan.thermal_safety_margin = 0.95;
    plan.actions[0] = HierarchicalSwitchAction { src: node(0, 1), dst: node(1, 0), action: SwitchActionType::CloseBackup, edge_id: 2, new_max_capacity: 90.0 };
    lib.register_plan(plan).unwrap();

    let srcs = [node(0, 0); 4];
    let sc = [1u8; 4];
    let sm = [1u64 << 1; 4];

    let ctx = ContingencyContext {
        sources: &srcs,
        sinks_c: &sc,
        sinks_m: &sm,
        sens: None,
        flows: None,
        limits: None,
    };

    for _ in 0..1_000 {
        let _ = g.restore_bridge(node(0, 1), node(1, 0));
        bounds[1].quarantined = false;
        let out = lib.select_and_apply_contingency_hierarchical(
            &mut g, &mut bounds, (node(0, 1), node(1, 0), 1), ctx,
        );
        assert!(matches!(out, ContingencyOutcome::Applied { .. }));
    }
}
