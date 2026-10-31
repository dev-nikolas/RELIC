//! # Causal Syndrome Decoding & Epistemic Scratchpad Module
//!
//! Decodificación de síndromes causales por paridad de bits (ECC/LDPC) y
//! memoria volátil con TTL (`EpistemicScratchpad`) para abstención ante equifinalidad.

pub mod decoder;
pub mod types;

pub use decoder::CausalSyndromeDecoder;
pub use types::{
    CausalParityMatrix, EpistemicScratchpad, FaultCandidate, FaultClass,
    ScratchpadEntry, SyndromeVerdict, MAX_SYNDROME_NODES, SCRATCHPAD_CAPACITY,
};

