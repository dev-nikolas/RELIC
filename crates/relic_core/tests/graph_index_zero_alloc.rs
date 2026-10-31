//! Gate de Fase 6: construir y consultar el índice no asigna memoria.
//!
//! Binario propio: un `#[global_allocator]` solo puede declararse una vez por
//! binario, así que se replica el patrón de `solver_zero_alloc.rs`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use relic_core::graph_index::{ReachHierarchy, ReachVerdict};
use relic_core::EpisodicMemory;

static TRACKING: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

struct Tracker;

unsafe impl GlobalAlloc for Tracker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
            ALLOC_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: Tracker = Tracker;

fn measured() -> (usize, usize) {
    (ALLOC_COUNT.load(Ordering::SeqCst), ALLOC_BYTES.load(Ordering::SeqCst))
}

#[test]
fn build_and_query_never_allocate() {
    let n = 64u32;
    let mut mem = EpisodicMemory::new();
    for i in 0..n {
        mem.insert_edge(format!("n{i}"), format!("n{}", (i + 1) % n));
        mem.insert_edge(format!("n{i}"), format!("n{}", (i * 7 + 3) % n));
    }

    // Calentamiento fuera de la ventana medida.
    let warm = ReachHierarchy::build(&mem).unwrap();
    let _ = warm.reaches(0, n - 1);
    let _ = ReachHierarchy::build(&mem).unwrap();

    TRACKING.store(true, Ordering::SeqCst);
    let index = ReachHierarchy::build(&mem).unwrap();
    TRACKING.store(false, Ordering::SeqCst);
    let (build_count, build_bytes) = measured();
    assert_eq!(
        build_count, 0,
        "la construcción asignó {build_count} veces ({build_bytes} B)"
    );

    ALLOC_COUNT.store(0, Ordering::SeqCst);
    ALLOC_BYTES.store(0, Ordering::SeqCst);
    TRACKING.store(true, Ordering::SeqCst);
    let mut reachable = 0usize;
    for i in 0..200_000u32 {
        let u = i % n;
        let v = (i * 13 + 5) % n;
        if index.reaches(u, v) == ReachVerdict::Reachable {
            reachable += 1;
        }
    }
    TRACKING.store(false, Ordering::SeqCst);
    let (query_count, query_bytes) = measured();
    assert_eq!(
        query_count, 0,
        "200 000 consultas asignaron {query_count} veces ({query_bytes} B)"
    );
    assert!(reachable > 0);
    assert_eq!(index.node_count(), n as usize);
}
