//! Verificación formal Zero-Alloc para grafos jerárquicos.
//! Verifica que is_reachable, verify_hierarchical_pairs, quarantine y restore
//! no realizan asignaciones en heap durante su ejecución.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use relic_core::contingency_bfs::{HierarchicalBitGraph, HierarchicalNode};

static TRACKING: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

struct Tracker;

unsafe impl GlobalAlloc for Tracker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: Tracker = Tracker;

fn node(c: u8, l: u8) -> HierarchicalNode {
    HierarchicalNode::new(c, l).unwrap()
}

#[test]
fn hierarchical_graph_hot_path_zero_alloc() {
    let mut g = HierarchicalBitGraph::<4>::new();
    for c in 0..3u8 {
        for u in 0..10u8 {
            let _ = g.add_intra_cluster_edge(c, u, u + 1);
        }
        let _ = g.add_inter_cluster_bridge(c as u32, node(c, 10), node(c + 1, 0));
    }

    // Warm-up fuera de tracking
    let _ = g.is_reachable(node(0, 0), node(3, 5));
    let a = node(0, 10);
    let b = node(1, 0);
    let _ = g.quarantine_bridge(a, b);
    let _ = g.restore_bridge(a, b);

    // Medición exacta en ventana aislada
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    TRACKING.store(true, Ordering::SeqCst);

    for _ in 0..100_000 {
        let _ = g.is_reachable(node(0, 0), node(3, 5));
        let _ = g.quarantine_bridge(a, b);
        let _ = g.restore_bridge(a, b);
    }

    let sources = [node(0, 0), node(1, 0), node(2, 0), node(3, 0)];
    let sinks_cluster = [3u8; 4];
    let sinks_mask = [1u64 << 5, 1u64 << 5, 1u64 << 5, 1u64 << 5];
    for _ in 0..10_000 {
        let _ = g.verify_hierarchical_pairs(&sources, &sinks_cluster, &sinks_mask);
    }

    TRACKING.store(false, Ordering::SeqCst);
    let allocs = ALLOC_COUNT.load(Ordering::SeqCst);

    assert_eq!(allocs, 0, "Zero-Alloc violado: {allocs} llamadas a alloc");
}

