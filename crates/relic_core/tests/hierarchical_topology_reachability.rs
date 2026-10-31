//! Verificación formal de topología y alcanzabilidad jerárquica multi-clúster.
//! Oráculo: BFS plano sobre K×64 nodos en arrays de stack.

use relic_core::contingency_bfs::{
    CausalReachabilityFilter, HierarchicalBitGraph, HierarchicalNode, MAX_GRAPH_NODES,
};

/// Oráculo BFS plano de referencia para K=4 (256 nodos) con aristas intra e inter isla.
#[allow(dead_code)]
fn flat_oracle_reachable(
    intra: &[[u64; MAX_GRAPH_NODES]; 4], bridges: &[(usize, usize)], src: usize, dst: usize,
) -> bool {
    let n = 4 * MAX_GRAPH_NODES;
    if src >= n || dst >= n { return false; }
    if src == dst { return true; }
    let mut visited = [0u64; 4];
    let mut queue = [0usize; 256];
    let (mut head, mut tail) = (0, 0);
    visited[src / 64] |= 1u64 << (src % 64);
    queue[tail] = src; tail += 1;
    while head < tail {
        let cur = queue[head]; head += 1;
        let ci = cur / 64; let li = cur % 64;
        let mut nbrs = intra[ci][li];
        while nbrs != 0 {
            let bit = nbrs.trailing_zeros() as usize;
            let nb = ci * 64 + bit;
            if (visited[nb / 64] & (1u64 << (nb % 64))) == 0 {
                visited[nb / 64] |= 1u64 << (nb % 64);
                if nb == dst { return true; }
                if tail < 256 { queue[tail] = nb; tail += 1; }
            }
            nbrs &= nbrs - 1;
        }
        for &(u, v) in bridges {
            let nb = if cur == u { Some(v) } else if cur == v { Some(u) } else { None };
            if let Some(target) = nb {
                if (visited[target / 64] & (1u64 << (target % 64))) == 0 {
                    visited[target / 64] |= 1u64 << (target % 64);
                    if target == dst { return true; }
                    if tail < 256 { queue[tail] = target; tail += 1; }
                }
            }
        }
    }
    false
}

fn node(c: u8, l: u8) -> HierarchicalNode { HierarchicalNode::new(c, l).unwrap() }

// Conectividad intra-isla pura y detección de aislamiento
#[test]
fn intra_island_reachability_pure() {
    let mut g = HierarchicalBitGraph::<2>::new();
    g.add_intra_cluster_edge(0, 0, 1).unwrap();
    g.add_intra_cluster_edge(0, 1, 2).unwrap();
    assert!(g.is_reachable(node(0, 0), node(0, 2)));
    assert!(!g.is_reachable(node(0, 0), node(0, 10)));
    assert!(!g.is_reachable(node(0, 0), node(1, 0)));
}

// Salto simple inter-isla bidireccional
#[test]
fn single_inter_island_hop_bidirectional() {
    let mut g = HierarchicalBitGraph::<2>::new();
    g.add_intra_cluster_edge(0, 5, 6).unwrap();
    g.add_intra_cluster_edge(1, 0, 1).unwrap();
    g.add_inter_cluster_bridge(0, node(0, 6), node(1, 0)).unwrap();
    assert!(g.is_reachable(node(0, 5), node(1, 1)));
    assert!(g.is_reachable(node(1, 1), node(0, 5)));
    assert!(!g.is_reachable(node(0, 5), node(1, 10)));
}

// Invariante de conteo de referencias de puentes paralelos (cuarentena parcial)
#[test]
fn parallel_bridge_refcount_partial_quarantine() {
    let mut g = HierarchicalBitGraph::<2>::new();
    g.add_intra_cluster_edge(0, 0, 1).unwrap();
    g.add_intra_cluster_edge(1, 0, 1).unwrap();
    let a = node(0, 1); let b = node(1, 0);
    g.add_inter_cluster_bridge(10, a, b).unwrap();
    g.add_inter_cluster_bridge(11, a, b).unwrap();
    assert_eq!(g.bridges.bridge_count(a, b), 2);
    g.quarantine_bridge(a, b).unwrap();
    assert_eq!(g.bridges.bridge_count(a, b), 1);
    assert!(g.backbone.is_connected(0, 1));
    assert!(g.is_reachable(node(0, 0), node(1, 1)));
}

// Idempotencia estricta de cuarentena y restauración
#[test]
fn idempotence_quarantine_and_restore() {
    let mut g = HierarchicalBitGraph::<2>::new();
    let a = node(0, 0); let b = node(1, 0);
    g.add_inter_cluster_bridge(0, a, b).unwrap();
    g.quarantine_bridge(a, b).unwrap();
    g.quarantine_bridge(a, b).unwrap();
    assert_eq!(g.bridges.bridge_count(a, b), 0);
    g.restore_bridge(a, b).unwrap();
    assert_eq!(g.bridges.bridge_count(a, b), 1);
    g.restore_bridge(a, b).unwrap();
    assert_eq!(g.bridges.bridge_count(a, b), 1);
}

// Colapso total de puentes paralelos y reconexión exacta
#[test]
fn full_bridge_collapse_and_exact_reconnect() {
    let mut g = HierarchicalBitGraph::<2>::new();
    g.add_intra_cluster_edge(0, 0, 1).unwrap();
    g.add_intra_cluster_edge(1, 0, 1).unwrap();
    let a = node(0, 1); let b = node(1, 0);
    g.add_inter_cluster_bridge(0, a, b).unwrap();
    g.add_inter_cluster_bridge(1, a, b).unwrap();
    g.quarantine_bridge(a, b).unwrap();
    g.quarantine_bridge(a, b).unwrap();
    assert_eq!(g.bridges.bridge_count(a, b), 0);
    assert!(!g.backbone.is_connected(0, 1));
    assert!(!g.is_reachable(node(0, 0), node(1, 1)));
    g.restore_bridge(a, b).unwrap();
    assert_eq!(g.bridges.bridge_count(a, b), 1);
    assert!(g.backbone.is_connected(0, 1));
    assert!(g.is_reachable(node(0, 0), node(1, 1)));
}

// Equivalencia exacta de dominio único (K = 1)
#[test]
fn bitwise_single_domain_k1_equivalence() {
    {
        let g = HierarchicalBitGraph::<1>::new();
        let srcs = [node(0,0),node(0,1),node(0,2),node(0,3)];
        let sc = [0u8;4]; let sm = [1u64<<4,1u64<<5,1u64<<6,1u64<<7];
        let mh = g.verify_hierarchical_pairs(&srcs,&sc,&sm);
        let mf = CausalReachabilityFilter::verify_critical_pairs(&g.islands[0],&[0,1,2,3],&sm);
        let mf_expected: u8 = (0..4).map(|l| if sm[l]!=0&&(mf>>(l as u8))&1!=0{1<<l}else{0}).fold(0,|a,b|a|b);
        assert_eq!(mh, mf_expected);
    }
    {
        let mut g = HierarchicalBitGraph::<1>::new();
        for i in 0u8..3 { g.add_intra_cluster_edge(0, i, i+1).unwrap(); }
        let srcs = [node(0,0),node(0,1),node(0,2),node(0,3)];
        let sc = [0u8;4]; let sm = [1u64<<3,1u64<<3,1u64<<3,1u64<<0];
        let mh = g.verify_hierarchical_pairs(&srcs,&sc,&sm);
        let mf = CausalReachabilityFilter::verify_critical_pairs(&g.islands[0],&[0,1,2,3],&sm);
        let mf_expected: u8 = (0..4).map(|l| if sm[l]!=0&&(mf>>(l as u8))&1!=0{1<<l}else{0}).fold(0,|a,b|a|b);
        assert_eq!(mh, mf_expected);
    }
}

// Tránsito multi-hop con re-entrada a isla internamente particionada
#[test]
fn multi_hop_transit_split_island_reentry() {
    let mut g = HierarchicalBitGraph::<2>::new();
    g.add_intra_cluster_edge(0, 0, 1).unwrap();
    g.add_intra_cluster_edge(0, 50, 51).unwrap();
    g.add_intra_cluster_edge(1, 10, 20).unwrap();
    g.add_inter_cluster_bridge(0, node(0,1), node(1,10)).unwrap();
    g.add_inter_cluster_bridge(1, node(1,20), node(0,50)).unwrap();
    assert!(g.is_reachable(node(0,0), node(0,51)));
    g.islands[1] = relic_core::contingency_bfs::BitAdjacencyGraph::new();
    assert!(!g.is_reachable(node(0,0), node(0,51)));
}
