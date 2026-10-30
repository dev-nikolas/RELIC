//! # Topological Regime Falsification Test Suite

use relic_core::anomaly::channel::SixEyesChannel;
use relic_core::anomaly::topology_coupling::{
    NodeSpatialNeighborhood, TopologicalResidualEngine,
};
use relic_core::anomaly::{load_weights_from_bytes, WEIGHTS_SEED42_BYTES};
use std::time::Instant;

struct TestRng { state: u64 }
impl TestRng {
    fn new(seed: u64) -> Self { Self { state: seed.max(1) } }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.state = x;
        x
    }
    fn next_f64(&mut self) -> f64 { (self.next_u64() as f64) / (u64::MAX as f64) }
    fn next_gaussian(&mut self, mean: f64, std: f64) -> f64 {
        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        mean + z0 * std
    }
}

#[test]
fn test_topological_regime_immunity_and_causal_fault_detection() {
    const NUM_NODES: usize = 4;
    let mut rng = TestRng::new(0xCAFE_BABE_1337_0001);
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).unwrap();

    let mut topo_engine = TopologicalResidualEngine::<NUM_NODES>::new();
    let w_third = 1.0f32 / 3.0f32;
    for i in 0..NUM_NODES {
        let mut neighbors = Vec::with_capacity(3);
        for j in 0..NUM_NODES {
            if i != j { neighbors.push((j as u16, w_third)); }
        }
        topo_engine.set_neighborhood(i, NodeSpatialNeighborhood::new(&neighbors).unwrap()).unwrap();
    }

    let channels: Vec<SixEyesChannel> = (0..NUM_NODES).map(|_| SixEyesChannel::new()).collect();

    // 1. Fase Estacionaria
    for _ in 0..200 {
        let base_signal = rng.next_gaussian(100.0, 1.0);
        let mut snapshot = [0.0f32; NUM_NODES];
        for i in 0..NUM_NODES {
            snapshot[i] = (base_signal + rng.next_gaussian(0.0, 0.2)) as f32;
        }

        for i in 0..NUM_NODES {
            let assessment = channels[i].evaluate_with_topology(
                snapshot[i] as f64, &topo_engine.neighborhoods[i], &snapshot, &win, &wres,
            );
            assert!(!assessment.martingale_alarm);
        }
    }

    // 2. Salto Global de Régimen (+400% idéntico en todos los nodos)
    for _ in 0..100 {
        let base_regime = rng.next_gaussian(500.0, 2.0);
        let mut snapshot = [0.0f32; NUM_NODES];
        for i in 0..NUM_NODES {
            snapshot[i] = (base_regime + rng.next_gaussian(0.0, 0.2)) as f32;
        }

        for i in 0..NUM_NODES {
            let assessment = channels[i].evaluate_with_topology(
                snapshot[i] as f64, &topo_engine.neighborhoods[i], &snapshot, &win, &wres,
            );
            assert!(!assessment.martingale_alarm, "Salto global no debe disparar falsos positivos");
        }
    }

    // 3. Falla Causal Localizada (Sensor 0 cae a 250 MW mientras vecinos permanecen en 500 MW)
    let mut fault_detected_sample = None;
    for sample in 1..=50 {
        let mut snapshot = [0.0f32; NUM_NODES];
        snapshot[0] = (250.0 + rng.next_gaussian(0.0, 0.2)) as f32;
        for i in 1..NUM_NODES {
            snapshot[i] = (500.0 + rng.next_gaussian(0.0, 0.2)) as f32;
        }

        let assessment0 = channels[0].evaluate_with_topology(
            snapshot[0] as f64, &topo_engine.neighborhoods[0], &snapshot, &win, &wres,
        );
        if assessment0.martingale_alarm && fault_detected_sample.is_none() {
            fault_detected_sample = Some(sample);
        }
    }

    let detect_sample = fault_detected_sample.expect("Falla causal en nodo 0 debe detectarse");
    assert!(detect_sample <= 5, "Detección rápida en <= 5 muestras (observada en {})", detect_sample);

    // 4. Benchmark de Latencia (< 25 µs)
    const BENCH_TICKS: usize = 10_000;
    let bench_snapshot = [500.0f32, 500.0, 500.0, 500.0];
    let start = Instant::now();

    for _ in 0..BENCH_TICKS {
        for i in 0..NUM_NODES {
            let res = channels[i].evaluate_with_topology(
                bench_snapshot[i] as f64, &topo_engine.neighborhoods[i], &bench_snapshot, &win, &wres,
            );
            std::hint::black_box(res);
        }
    }

    let avg_micros = (start.elapsed().as_nanos() as f64 / BENCH_TICKS as f64) / 1000.0;
    assert!(avg_micros < 25.0, "Latencia excede 25 µs: {:.3} µs", avg_micros);
}
