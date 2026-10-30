//! # Zero-Alloc Guard & Latency Percentiles Audit
//!
//! Verificación estricta de 0 asignaciones en heap durante 250,000 ciclos continuos.
//! Medición estadística de percentiles con pre-calentamiento (warm-up) de caché L1/L2.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;
use relic_core::anomaly::topology_coupling::{NodeSpatialNeighborhood, ResidualSprtMartingale};
use relic_core::blackwall_scars::{NeighborTopologyToken, StructuralDiscriminationEngine, WL1Signature};
use relic_core::contingency_bfs::{BitAdjacencyGraph, PreCertifiedContingencyLibrary};
use relic_core::repair::{EdgeOperatingBounds, StagedEnvelope};
use relic_core::tactical_dispatch::{
    ReflexTickInput, StagedOrderRingBuffer, TacticalKernelContext, TacticalReflexKernel,
};

pub static TRACKING_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

pub fn run_zero_alloc_and_latency_audit() {
    const BENCH_CYCLES: usize = 250_000;
    const WARMUP_CYCLES: usize = 5_000;

    let mut latencies_ns = vec![0u32; BENCH_CYCLES];

    let sources = [0u8, 10, 20, 30];
    let sinks_mask = [1u64 << 1, 1u64 << 11, 1u64 << 21, 1u64 << 31];
    let mut graph = BitAdjacencyGraph::new();
    graph.add_bidirectional_edge(0, 1);
    graph.add_bidirectional_edge(10, 11);
    graph.add_bidirectional_edge(20, 21);
    graph.add_bidirectional_edge(30, 31);

    let mut bounds = [
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
    ];

    let contingency_lib = PreCertifiedContingencyLibrary::new();
    let mut scar_engine = StructuralDiscriminationEngine::new();
    let ring_buffer = StagedOrderRingBuffer::new();
    let nh = NodeSpatialNeighborhood::new(&[(1, 0.5), (2, 0.5)]).unwrap();
    let mut martingale = ResidualSprtMartingale::new(1.0, 0.5, 13.82);
    let token = NeighborTopologyToken::new(1, 1, false).raw();
    let wl1_sig = WL1Signature::compute_canonical(1, 1, &[token], &[token]);
    let clean_snapshot = [250.0f32, 250.0, 250.0, 0.0];

    // Fase de pre-calentamiento (warm-up) para eliminar fallos de página y cebar caché
    for i in 0..WARMUP_CYCLES as u64 {
        let mut env = StagedEnvelope::new(i + 1);
        env.add_op(1, 300.0, 200.0, 0).unwrap();
        let _ = ring_buffer.try_stage_from_cognitive(env, i + 500, (0, 1, 1), &bounds, 1000.0);
        let mut ctx = TacticalKernelContext {
            martingale: &mut martingale, scar_engine: &mut scar_engine, contingency_lib: &contingency_lib,
            graph: &mut graph, bounds: &mut bounds, ring_buffer: &ring_buffer,
        };
        let input = ReflexTickInput {
            current_tick: i, target_edge: (0, 1, 1), monitored_node: 0, sensor_snapshot: &clean_snapshot,
            neighborhood: &nh, wl1_sig: &wl1_sig, sources: &sources, sinks_mask: &sinks_mask,
        };
        let decision = TacticalReflexKernel::execute_tick(&mut ctx, &input);
        std::hint::black_box(decision);
    }

    // Activación formal del guardián de cero asignaciones en el hot path
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    ALLOC_BYTES.store(0, Ordering::SeqCst);
    TRACKING_ACTIVE.store(true, Ordering::SeqCst);

    for i in 0..BENCH_CYCLES as u64 {
        let mut env = StagedEnvelope::new(i + 1);
        env.add_op(1, 300.0, 200.0, 0).unwrap();
        let _ = ring_buffer.try_stage_from_cognitive(env, i + 500, (0, 1, 1), &bounds, 1000.0);

        let mut ctx = TacticalKernelContext {
            martingale: &mut martingale, scar_engine: &mut scar_engine, contingency_lib: &contingency_lib,
            graph: &mut graph, bounds: &mut bounds, ring_buffer: &ring_buffer,
        };
        let input = ReflexTickInput {
            current_tick: i, target_edge: (0, 1, 1), monitored_node: 0, sensor_snapshot: &clean_snapshot,
            neighborhood: &nh, wl1_sig: &wl1_sig, sources: &sources, sinks_mask: &sinks_mask,
        };

        let t0 = Instant::now();
        let decision = TacticalReflexKernel::execute_tick(&mut ctx, &input);
        let elapsed = t0.elapsed();
        latencies_ns[i as usize] = elapsed.as_nanos().min(u32::MAX as u128) as u32;

        std::hint::black_box(decision);
    }

    TRACKING_ACTIVE.store(false, Ordering::SeqCst);

    let recorded_allocs = ALLOC_COUNT.load(Ordering::SeqCst);
    let recorded_bytes = ALLOC_BYTES.load(Ordering::SeqCst);
    assert_eq!(recorded_allocs, 0, "Violación formal: llamadas a alloc detectadas");
    assert_eq!(recorded_bytes, 0, "Violación formal: bytes en heap detectados");

    latencies_ns.sort_unstable();
    let p50 = latencies_ns[BENCH_CYCLES * 50 / 100];
    let p90 = latencies_ns[BENCH_CYCLES * 90 / 100];
    let p99 = latencies_ns[BENCH_CYCLES * 99 / 100];
    let p99_9 = latencies_ns[BENCH_CYCLES * 999 / 1000];
    let min = latencies_ns[0];
    let max = latencies_ns[BENCH_CYCLES - 1];
    let sum: u64 = latencies_ns.iter().map(|&x| x as u64).sum();
    let mean = sum as f64 / BENCH_CYCLES as f64;

    println!("\n=== AUDITORÍA ESTADÍSTICA DE LATENCIAS (250,000 CICLOS POST-WARMUP) ===");
    println!("Ciclos Evaluados:   {}", BENCH_CYCLES);
    println!("Latencia Mínima:    {:.3} µs ({} ns)", min as f64 / 1000.0, min);
    println!("Latencia Media:     {:.3} µs ({:.1} ns)", mean / 1000.0, mean);
    println!("Percentil P50:      {:.3} µs ({} ns)", p50 as f64 / 1000.0, p50);
    println!("Percentil P90:      {:.3} µs ({} ns)", p90 as f64 / 1000.0, p90);
    println!("Percentil P99:      {:.3} µs ({} ns)", p99 as f64 / 1000.0, p99);
    println!("Percentil P99.9:    {:.3} µs ({} ns)", p99_9 as f64 / 1000.0, p99_9);
    println!("Latencia Máxima:    {:.3} µs ({} ns)", max as f64 / 1000.0, max);

    assert!((p99 as f64 / 1000.0) <= 351.5, "P99 excede el presupuesto de tiempo real");
}

