//! # Contingency Reachability & Bit-Parallel BFS
//!
//! Desambiguación física de topologías, filtrado causal MS-BFS en AVX2 y
//! conmutación pre-certificada de contingencias N-1 y N-2.

pub mod graph;
pub mod bridge;
pub mod hierarchy;
pub mod hierarchical_library;
pub mod reachability;
pub mod library;
pub mod sensitivity;

pub use graph::{BitAdjacencyGraph, MAX_GRAPH_NODES};
pub use bridge::{BridgeError, BridgeRegistry, HierarchicalNode, InterClusterBridge, MAX_BRIDGES, MAX_CLUSTERS};
pub use hierarchy::HierarchicalBitGraph;
pub use hierarchical_library::{
    HierarchicalContingencyLibrary, HierarchicalContingencyPlan, HierarchicalSwitchAction,
};
pub use reachability::{AlignedSimd, CausalReachabilityFilter};
pub use library::{
    ContingencyOutcome, PreCertifiedContingencyLibrary, PreCertifiedTopology, SwitchAction,
    SwitchActionType, MAX_CONTINGENCIES,
};
pub use sensitivity::{LinearizedSensitivityMatrix, MAX_SENSITIVITY_EDGES};
