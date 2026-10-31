//! # Causal Syndrome Decoding & Epistemic Uncertainty Falsification Suite

use relic_core::syndrome::{
    CausalParityMatrix, CausalSyndromeDecoder, EpistemicScratchpad, FaultClass,
    SyndromeVerdict,
};

#[test]
fn test_causal_syndrome_unambiguous_detection() {
    let mut parity = CausalParityMatrix::empty();
    parity.signature_count = 4;
    // Falla en arista 1 excita nodos 0 y 1: máscara 0b0011 (3)
    parity.edge_fault_signatures[1] = 0b0011;
    // Falla en sensor 2 solo excita nodo 2: máscara 0b0100 (4)
    parity.sensor_fault_signatures[2] = 0b0100;

    let mut scratchpad = EpistemicScratchpad::new();

    // 1. Observación exacta de arista 1
    let verdict = CausalSyndromeDecoder::decode(0b0011, 100, &parity, &mut scratchpad);
    match verdict {
        SyndromeVerdict::UnambiguousIdentification { primary, hamming_distance } => {
            assert_eq!(primary.target_id, 1);
            assert_eq!(primary.class, FaultClass::EdgeDisruption);
            assert_eq!(hamming_distance, 0);
        }
        _ => panic!("Esperado veredicto inequívoco"),
    }
}

#[test]
fn test_equifinality_triggers_epistemic_uncertainty_and_scratchpad() {
    let mut parity = CausalParityMatrix::empty();
    parity.signature_count = 4;
    // Ambigüedad idéntica: dos eventos producen exactamente la misma máscara de excitación
    parity.edge_fault_signatures[0] = 0b0110;
    parity.sensor_fault_signatures[1] = 0b0110;

    let mut scratchpad = EpistemicScratchpad::new();

    // Observación 0b0110
    let verdict = CausalSyndromeDecoder::decode(0b0110, 500, &parity, &mut scratchpad);
    match verdict {
        SyndromeVerdict::EpistemicUncertainty { candidate_a, candidate_b, ambiguity_margin } => {
            assert_eq!(ambiguity_margin, 0);
            assert_ne!(candidate_a.class, candidate_b.class);
            assert_eq!(scratchpad.active_count, 2);
        }
        _ => panic!("Debe declarar EpistemicUncertainty ante empate de Hamming"),
    }

    // Comprobación de TTL en el scratchpad
    scratchpad.prune_expired(560);
    assert_eq!(scratchpad.active_count, 0, "Las hipótesis debieron expirar tras 50 ticks");
}

#[test]
fn test_syndrome_latency_sub_50ns() {
    let mut parity = CausalParityMatrix::empty();
    parity.signature_count = 16;
    for i in 0..16 {
        parity.edge_fault_signatures[i] = (1u64 << i) | (1u64 << ((i + 1) % 16));
        parity.sensor_fault_signatures[i] = 1u64 << i;
    }
    let mut scratchpad = EpistemicScratchpad::new();

    // Warm-up de 5,000 ciclos para cebar la caché L1 y estabilizar el gobernador de frecuencia
    for k in 0..5_000 {
        let mask = (1u64 << (k % 16)) | (1u64 << ((k + 1) % 16));
        let v = CausalSyndromeDecoder::decode(mask, k as u64, &parity, &mut scratchpad);
        std::hint::black_box(v);
    }

    let start = std::time::Instant::now();
    let iters = 100_000;
    for k in 0..iters {
        let mask = (1u64 << (k % 16)) | (1u64 << ((k + 1) % 16));
        let v = CausalSyndromeDecoder::decode(mask, k as u64, &parity, &mut scratchpad);
        std::hint::black_box(v);
    }
    let elapsed = start.elapsed();
    let ns_per_op = (elapsed.as_nanos() as f64) / (iters as f64);
    println!("Latencia por decodificación de síndrome: {:.2} ns", ns_per_op);
    assert!(ns_per_op < 50.0, "La decodificación debe tomar menos de 50 ns");
}

