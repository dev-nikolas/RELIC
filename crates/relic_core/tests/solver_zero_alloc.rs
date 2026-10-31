//! Gate de Fase 4: la ruta `solve` no asigna memoria dinámica.
//!
//! Es un binario de test independiente: un `#[global_allocator]` solo puede
//! declararse una vez por binario, así que se replica el patrón de
//! `kernel_end_to_end_stress_audit.rs:26-43` en lugar de importarlo (los
//! `static` de allí no son visibles desde otro binario).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use relic_core::arithmetic::{compile, solve, SolveOutcome};

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

#[test]
fn solver_path_never_allocates() {
    let linear = compile("2*x + 1").unwrap();
    let quadratic = compile("x^2 - 100000000*x + 1").unwrap();
    let newton = compile("sin(x)").unwrap();

    // Calentamiento fuera de la ventana medida.
    for _ in 0..8 {
        let _ = solve(&linear, None);
        let _ = solve(&quadratic, None);
        let _ = solve(&newton, Some(3.0));
    }

    TRACKING.store(true, Ordering::SeqCst);
    let mut checksum = 0.0f64;
    for i in 0..20_000 {
        let seed = 3.0 + (i % 64) as f64 * 1e-6;
        if let SolveOutcome::Linear(c) = solve(&linear, None) {
            checksum += c.root;
        }
        if let SolveOutcome::Quadratic(l, h) = solve(&quadratic, None) {
            checksum += l.root + h.root;
        }
        if let SolveOutcome::Newton(n) = solve(&newton, Some(seed)) {
            checksum += n.root;
        }
    }
    TRACKING.store(false, Ordering::SeqCst);

    let count = ALLOC_COUNT.load(Ordering::SeqCst);
    let bytes = ALLOC_BYTES.load(Ordering::SeqCst);
    assert_eq!(count, 0, "la ruta de solve asignó {count} veces ({bytes} B)");
    assert!(checksum.is_finite());
}
