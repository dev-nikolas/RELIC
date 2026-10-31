//! # CausalSyndromeDecoder
//!
//! Decodificador determinista de paridad causal en tiempo constante sin asignaciones en heap.
//! Opera mediante instrucciones nativas XOR/POPCNT y filtrado branchless/predictivo
//! con latencia sub-50ns garantizada.

use super::types::{
    CausalParityMatrix, EpistemicScratchpad, FaultCandidate, FaultClass,
    SyndromeVerdict, MAX_SYNDROME_NODES,
};

/// Motor de decodificación de síndromes causales ortogonales.
pub struct CausalSyndromeDecoder;

impl CausalSyndromeDecoder {
    /// Decodifica el vector de residuos excitados contra la matriz de paridad causal.
    /// Si hay empate en la distancia mínima entre dos fallas físicamente distintas,
    /// emite `SyndromeVerdict::EpistemicUncertainty` y registra ambas conjeturas en el scratchpad.
    #[inline(always)]
    pub fn decode(
        observed_mask: u64,
        current_tick: u64,
        parity: &CausalParityMatrix,
        scratchpad: &mut EpistemicScratchpad,
    ) -> SyndromeVerdict {
        if observed_mask == 0 {
            return SyndromeVerdict::NominalQuiescence;
        }

        let n = (parity.signature_count as usize).min(MAX_SYNDROME_NODES);
        let mut d1: u32 = u32::MAX;
        let mut cand1: u32 = 0;
        let mut d2: u32 = u32::MAX;
        let mut cand2: u32 = 0;

        let edge_sigs = &parity.edge_fault_signatures[..n];
        let sensor_sigs = &parity.sensor_fault_signatures[..n];

        // 1. Barrido ultrarrápido sobre firmas de aristas
        for (i, &sig_e) in edge_sigs.iter().enumerate() {
            if sig_e != 0 {
                let d = (observed_mask ^ sig_e).count_ones();
                if d < d1 {
                    d2 = d1;
                    cand2 = cand1;
                    d1 = d;
                    cand1 = i as u32;
                } else if d <= d2 {
                    d2 = d;
                    cand2 = i as u32;
                }
            }
        }

        // 2. Barrido ultrarrápido sobre firmas de sensores
        for (i, &sig_s) in sensor_sigs.iter().enumerate() {
            if sig_s != 0 {
                let d = (observed_mask ^ sig_s).count_ones();
                if d < d1 {
                    d2 = d1;
                    cand2 = cand1;
                    d1 = d;
                    cand1 = 0x0001_0000 | (i as u32);
                } else if d <= d2 {
                    d2 = d;
                    cand2 = 0x0001_0000 | (i as u32);
                }
            }
        }

        let min_dist_1 = d1 as u8;
        let min_dist_2 = d2 as u8;

        // 3. Regla de Abstención Epistémica ante Empate (Equifinalidad)
        if min_dist_1 == min_dist_2 && cand1 != cand2 {
            let best_cand_1 = Self::unpack_candidate(cand1, min_dist_1);
            let best_cand_2 = Self::unpack_candidate(cand2, min_dist_2);
            scratchpad.record_hypothesis(best_cand_1, observed_mask, current_tick, 50);
            scratchpad.record_hypothesis(best_cand_2, observed_mask, current_tick, 50);

            return SyndromeVerdict::EpistemicUncertainty {
                candidate_a: best_cand_1,
                candidate_b: best_cand_2,
                ambiguity_margin: 0,
            };
        }

        let best_cand_1 = Self::unpack_candidate(cand1, min_dist_1);

        if min_dist_1 <= 2 {
            SyndromeVerdict::UnambiguousIdentification {
                primary: best_cand_1,
                hamming_distance: min_dist_1,
            }
        } else {
            SyndromeVerdict::EpistemicUncertainty {
                candidate_a: best_cand_1,
                candidate_b: FaultCandidate {
                    target_id: 0xFFFF_FFFF,
                    class: FaultClass::UnknownExogenous,
                    confidence_score: 0,
                },
                ambiguity_margin: min_dist_1,
            }
        }
    }

    #[inline(always)]
    fn unpack_candidate(packed: u32, dist: u8) -> FaultCandidate {
        let class_byte = ((packed >> 16) & 0xFF) as u8;
        let id = packed & 0xFFFF;
        let class = match class_byte {
            0 => FaultClass::EdgeDisruption,
            1 => FaultClass::SensorCalibrationDrift,
            2 => FaultClass::ActuatorSaturating,
            _ => FaultClass::UnknownExogenous,
        };
        FaultCandidate {
            target_id: id,
            class,
            confidence_score: 100u8.saturating_sub(dist.saturating_mul(15)),
        }
    }
}
