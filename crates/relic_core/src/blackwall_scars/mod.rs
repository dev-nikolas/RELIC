//! # Structural Discrimination Engine & Invariant Scars (Blackwall)
//!
//! Indexación O(1), firmas Weisfeiler-Lehman 1-hop invariantes y predicados vectoriales AVX2.

pub mod signature;
pub mod predicates;
pub mod engine;

pub use signature::{CriticalEdgeKey, NeighborTopologyToken, WL1Signature};
pub use predicates::{
    Avx2PredicateBlock, EdgeScarSlot, LocalScarRule, MAX_SCARS_PER_EDGE, MAX_TRACKED_EDGES,
};
pub use engine::StructuralDiscriminationEngine;

