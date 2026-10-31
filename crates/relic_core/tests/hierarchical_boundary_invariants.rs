//! Verificación formal de cotas topológicas e invariantes de frontera.

use relic_core::contingency_bfs::{
    BridgeError, HierarchicalBitGraph, HierarchicalNode, MAX_BRIDGES,
};

fn node(c: u8, l: u8) -> HierarchicalNode { HierarchicalNode::new(c, l).unwrap() }

// Cadena lineal larga multi-clúster (1 024 nodos)
#[test]
fn long_chain_multi_cluster_reachability() {
    const K: usize = 16;
    let mut g = HierarchicalBitGraph::<K>::new();
    for c in 0..K as u8 {
        g.add_intra_cluster_edge(c, 0, 63).unwrap();
    }
    for c in 0..(K as u8 - 1) {
        g.add_inter_cluster_bridge(c as u32, node(c, 63), node(c+1, 0)).unwrap();
    }
    assert!(g.is_reachable(node(0, 0), node(15, 63)));
    g.quarantine_bridge(node(7, 63), node(8, 0)).unwrap();
    assert!(!g.is_reachable(node(0, 0), node(15, 63)));
    g.restore_bridge(node(7, 63), node(8, 0)).unwrap();
    assert!(g.is_reachable(node(0, 0), node(15, 63)));
}

// Prohibición de satisfacción vacua en máscaras nulas
#[test]
fn empty_sink_mask_never_satisfied() {
    let mut g = HierarchicalBitGraph::<2>::new();
    for c in 0..2u8 { for u in 0..63u8 { g.add_intra_cluster_edge(c, u, u+1).unwrap(); } }
    g.add_inter_cluster_bridge(0, node(0, 63), node(1, 0)).unwrap();

    let srcs = [node(0,0), node(0,1), node(0,2), node(0,3)];
    let sc = [0u8; 4];
    let mask = g.verify_hierarchical_pairs(&srcs, &sc, &[0u64; 4]);
    assert_eq!(mask, 0);
    let sm = [0u64, 1u64 << 1, 0u64, 1u64 << 3];
    let mask2 = g.verify_hierarchical_pairs(&srcs, &sc, &sm);
    assert_eq!(mask2 & 0b0101, 0);
}

// Verificación estricta de cotas sin aliasing modular
#[test]
fn strict_bounds_verification_no_modular_aliasing() {
    assert!(HierarchicalNode::new(64, 0).is_err());
    assert!(HierarchicalNode::new(0, 64).is_err());
    assert!(HierarchicalNode::from_global(4096).is_err());
    let g = HierarchicalBitGraph::<2>::new();
    let bad = HierarchicalNode { cluster: 2, local: 0 };
    assert!(!g.is_reachable(bad, node(0, 0)));
    let mut graph_two = HierarchicalBitGraph::<2>::new();
    let bad_n = HierarchicalNode { cluster: 3, local: 0 };
    assert!(graph_two.add_inter_cluster_bridge(0, bad_n, node(1, 0)).is_err());
}

// Rechazo formal de puentes intra-clúster en el registro de puentes
#[test]
fn intra_cluster_bridge_rejected() {
    let mut g = HierarchicalBitGraph::<2>::new();
    let a = node(0, 0); let b = node(0, 5);
    let res = g.add_inter_cluster_bridge(0, a, b);
    assert_eq!(res, Err(BridgeError::IntraClusterEdgeNotAllowed));
    assert_eq!(g.bridges.bridge_count(a, b), 0);
    assert!(!g.backbone.is_connected(0, 0));
}

// Límite de capacidad y saturación de la tabla de puentes
#[test]
fn bridge_table_capacity_saturation() {
    let mut g = HierarchicalBitGraph::<2>::new();
    let a = node(0, 0); let b = node(1, 0);
    for i in 0..MAX_BRIDGES {
        assert!(g.add_inter_cluster_bridge(i as u32, a, b).is_ok());
    }
    let res = g.add_inter_cluster_bridge(MAX_BRIDGES as u32, a, b);
    assert_eq!(res, Err(BridgeError::BridgeTableFull));
    assert_eq!(g.bridges.count as usize, MAX_BRIDGES);
}
