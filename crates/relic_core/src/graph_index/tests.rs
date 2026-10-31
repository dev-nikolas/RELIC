//! Contratos del índice: exactitud, componentes, capacidad y determinismo.

use super::{IndexFault, ReachHierarchy, ReachVerdict, MAX_COMPONENTS, MAX_INDEX_NODES};
use crate::episodic_memory::EpisodicMemory;

fn memory(edges: &[(&str, &str)]) -> EpisodicMemory {
    let mut m = EpisodicMemory::new();
    for &(u, v) in edges {
        m.insert_edge(u, v);
    }
    m
}

fn index(edges: &[(&str, &str)]) -> ReachHierarchy {
    ReachHierarchy::build(&memory(edges)).expect("el índice debe construirse")
}

fn verdict(index: &ReachHierarchy, u: u32, v: u32) -> bool {
    match index.reaches(u, v) {
        ReachVerdict::Reachable => true,
        ReachVerdict::Unreachable => false,
        ReachVerdict::OutOfRange => panic!("id fuera de rango: {u} -> {v}"),
    }
}

#[test]
fn layout_is_frozen() {
    assert_eq!(core::mem::size_of::<ReachHierarchy>(), 34_824);
    assert_eq!(MAX_INDEX_NODES, 1024);
    assert_eq!(MAX_COMPONENTS, 512);
}

#[test]
fn empty_graph_has_no_nodes() {
    let idx = ReachHierarchy::build(&EpisodicMemory::new()).unwrap();
    assert_eq!(idx.node_count(), 0);
    assert_eq!(idx.component_count(), 0);
    assert_eq!(idx.reaches(0, 0), ReachVerdict::OutOfRange);
}

#[test]
fn single_node_reaches_itself() {
    let idx = index(&[("a", "a")]);
    assert_eq!(idx.node_count(), 1);
    assert_eq!(idx.component_count(), 1);
    assert_eq!(idx.reaches(0, 0), ReachVerdict::Reachable);
    assert_eq!(idx.component_of(0), Some(0));
    assert_eq!(idx.component_of(1), None);
}

#[test]
fn chain_is_one_way() {
    let idx = index(&[("a", "b"), ("b", "c")]);
    assert!(verdict(&idx, 0, 1));
    assert!(verdict(&idx, 0, 2));
    assert!(!verdict(&idx, 1, 0));
    assert!(!verdict(&idx, 2, 0));
    assert_eq!(idx.component_count(), 3);
}

#[test]
fn cycle_members_share_component() {
    let idx = index(&[("a", "b"), ("b", "c"), ("c", "a")]);
    assert_eq!(idx.component_count(), 1);
    for u in 0..3 {
        for v in 0..3 {
            assert!(verdict(&idx, u, v), "{u} -> {v} dentro de la SCC");
        }
    }
}

#[test]
fn two_cycles_joined_one_way() {
    let idx = index(&[("a", "b"), ("b", "a"), ("c", "d"), ("d", "c"), ("b", "c")]);
    assert_eq!(idx.component_count(), 2);
    assert!(verdict(&idx, 0, 2) && verdict(&idx, 0, 3));
    assert!(!verdict(&idx, 2, 0) && !verdict(&idx, 3, 0));
}

#[test]
fn isolated_nodes_are_unreachable() {
    let mut m = EpisodicMemory::new();
    m.interner.intern("a");
    m.interner.intern("b");
    let idx = ReachHierarchy::build(&m).unwrap();
    assert_eq!(idx.node_count(), 2);
    assert_eq!(idx.component_count(), 2);
    assert!(!verdict(&idx, 0, 1) && !verdict(&idx, 1, 0));
}

#[test]
fn diamond_keeps_siblings_unreachable() {
    let idx = index(&[("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")]);
    assert!(verdict(&idx, 0, 3));
    assert!(!verdict(&idx, 1, 2) && !verdict(&idx, 2, 1));
    assert!(!verdict(&idx, 1, 0));
}

#[test]
fn cycle_with_exit_does_not_return() {
    let idx = index(&[("a", "b"), ("b", "a"), ("b", "c"), ("c", "d")]);
    assert!(verdict(&idx, 0, 3) && verdict(&idx, 3, 3));
    assert!(!verdict(&idx, 3, 0) && !verdict(&idx, 2, 0));
}

#[test]
fn out_of_range_ids_are_reported() {
    let idx = index(&[("a", "b")]);
    assert_eq!(idx.reaches(2, 0), ReachVerdict::OutOfRange);
    assert_eq!(idx.reaches(0, 2), ReachVerdict::OutOfRange);
}

#[test]
fn too_many_nodes_abstains() {
    let mut m = EpisodicMemory::new();
    for i in 0..MAX_INDEX_NODES {
        m.insert_edge(format!("n{i}"), format!("n{}", i + 1));
    }
    assert_eq!(
        ReachHierarchy::build(&m).err(),
        Some(IndexFault::TooManyNodes { nodes: MAX_INDEX_NODES + 1, max: MAX_INDEX_NODES })
    );
}

#[test]
fn too_many_components_abstains() {
    // Cadena de 601 nodos: 601 SCC triviales > 512.
    let mut m = EpisodicMemory::new();
    for i in 0..600 {
        m.insert_edge(format!("n{i}"), format!("n{}", i + 1));
    }
    assert_eq!(
        ReachHierarchy::build(&m).err(),
        Some(IndexFault::TooManyComponents { max: MAX_COMPONENTS })
    );
}

#[test]
fn component_ids_are_deterministic() {
    let m = memory(&[("a", "b"), ("b", "c"), ("c", "a"), ("c", "d"), ("d", "e")]);
    let (first, second) = (ReachHierarchy::build(&m).unwrap(), ReachHierarchy::build(&m).unwrap());
    assert_eq!(first.component_count(), second.component_count());
    for node in 0..5u32 {
        assert_eq!(first.component_of(node), second.component_of(node));
        for target in 0..5u32 {
            assert_eq!(first.reaches(node, target), second.reaches(node, target));
        }
    }
}
