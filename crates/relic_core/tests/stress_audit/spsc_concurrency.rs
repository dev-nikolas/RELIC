//! # SPSC Concurrency Stress Test
//!
//! Prueba de contención multi-hilo real: 1 hilo productor (Plano Cognitivo) y
//! 1 hilo consumidor (Plano Reflejo) intercambiando 100,000 directivas en paralelo.

use std::sync::Arc;
use std::thread;
use relic_core::anomaly::topology_coupling::{NodeSpatialNeighborhood, ResidualSprtMartingale};
use relic_core::blackwall_scars::{NeighborTopologyToken, StructuralDiscriminationEngine, WL1Signature};
use relic_core::contingency_bfs::{BitAdjacencyGraph, PreCertifiedContingencyLibrary};
use relic_core::repair::{EdgeOperatingBounds, StagedEnvelope};
use relic_core::tactical_dispatch::{
    ReflexTickInput, StagedOrderRingBuffer, TacticalDecision, TacticalKernelContext, TacticalReflexKernel,
};

pub fn run_spsc_concurrency_stress_test() {
    const TOTAL_MESSAGES: u64 = 100_000;
    let ring_buffer = Arc::new(StagedOrderRingBuffer::new());

    let bounds_prod = [
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        EdgeOperatingBounds::new(0.0, 500.0, 100.0),
    ];

    let ring_prod = Arc::clone(&ring_buffer);
    let producer_handle = thread::spawn(move || {
        for i in 1..=TOTAL_MESSAGES {
            let mut env = StagedEnvelope::new(i);
            env.add_op(1, 300.0, 200.0, 0).unwrap();

            loop {
                match ring_prod.try_stage_from_cognitive(env, 2_000_000, (0, 1, 1), &bounds_prod, 1000.0) {
                    Ok(_) => break,
                    Err(_) => std::hint::spin_loop(),
                }
            }
        }
    });

    let ring_cons = Arc::clone(&ring_buffer);
    let consumer_handle = thread::spawn(move || {
        let sources = [0u8, 10, 20, 30];
        let sinks_mask = [1u64 << 1, 1u64 << 11, 1u64 << 21, 1u64 << 31];
        let mut graph = BitAdjacencyGraph::new();
        graph.add_bidirectional_edge(0, 1);
        let mut bounds = [
            EdgeOperatingBounds::new(0.0, 500.0, 100.0),
            EdgeOperatingBounds::new(0.0, 500.0, 100.0),
        ];
        let contingency_lib = PreCertifiedContingencyLibrary::new();
        let mut scar_engine = StructuralDiscriminationEngine::new();
        let nh = NodeSpatialNeighborhood::new(&[(1, 0.5), (2, 0.5)]).unwrap();
        let mut martingale = ResidualSprtMartingale::new(1.0, 0.5, 13.82);
        let token = NeighborTopologyToken::new(1, 1, false).raw();
        let wl1_sig = WL1Signature::compute_canonical(1, 1, &[token], &[token]);
        let clean_snapshot = [250.0f32, 250.0, 250.0, 0.0];

        let mut expected_id = 1u64;

        while expected_id <= TOTAL_MESSAGES {
            let mut ctx = TacticalKernelContext {
                martingale: &mut martingale,
                scar_engine: &mut scar_engine,
                contingency_lib: &contingency_lib,
                graph: &mut graph,
                bounds: &mut bounds,
                ring_buffer: &ring_cons,
            };
            let input = ReflexTickInput {
                current_tick: expected_id,
                target_edge: (0, 1, 1),
                monitored_node: 0,
                sensor_snapshot: &clean_snapshot,
                neighborhood: &nh,
                wl1_sig: &wl1_sig,
                sources: &sources,
                sinks_mask: &sinks_mask,
            };

            let decision = TacticalReflexKernel::execute_tick(&mut ctx, &input);
            match decision {
                TacticalDecision::TransmitDMA { envelope, .. } => {
                    assert_eq!(envelope.envelope_id, expected_id, "Corrupción o violación FIFO en SPSC");
                    expected_id += 1;
                }
                TacticalDecision::IdleNoStagedOrder { .. } => {
                    std::hint::spin_loop();
                }
                other => panic!("Decisión inesperada durante prueba de contención SPSC: {:?}", other),
            }
        }
    });

    producer_handle.join().expect("Productor finalizó exitosamente");
    consumer_handle.join().expect("Consumidor finalizó exitosamente");
    assert!(ring_buffer.is_empty(), "El búfer SPSC debe quedar vacío al final");
}

