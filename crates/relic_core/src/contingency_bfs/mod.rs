//! # Contingency Reachability & Bit-Parallel BFS
//!
//! Desambiguación física de topologías, filtrado causal MS-BFS en AVX2 y
//! conmutación pre-certificada de contingencias N-1 y N-2.

pub mod graph;
pub mod reachability;
pub mod library;

pub use graph::{BitAdjacencyGraph, MAX_GRAPH_NODES};
pub use reachability::{AlignedSimd, CausalReachabilityFilter};
pub use library::{
    ContingencyOutcome, PreCertifiedContingencyLibrary, PreCertifiedTopology, SwitchAction,
    SwitchActionType, MAX_CONTINGENCIES,
};

