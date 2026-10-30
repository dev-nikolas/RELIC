//! # Topological Coupling and Spatial Residual Engine
//!
//! Acoplamiento topológico causal y flujo de residuos espaciales.
//! Elimina el flapping y los falsos positivos ante cambios de régimen globales.

pub mod types;
pub mod martingale;

pub use types::{NodeSpatialNeighborhood, TopologicalResidualEngine, MAX_SPATIAL_NEIGHBORS};
pub use martingale::ResidualSprtMartingale;

