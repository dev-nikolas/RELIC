//! # Topological Coupling, Quiescent Calibration & Bounded-Scale SPRT
//!
//! Módulo de acoplamiento espacial, calibración en línea Quiescence-Gated NLMS
//! y martingala SPRT normalizada con estimador de Welford acotado.

pub mod calibration;
pub mod martingale;
pub mod types;

pub use calibration::QuiescentCalibrator;
pub use martingale::{ResidualSprtMartingale, WALD_THRESHOLD_ALPHA_1E6};
pub use types::{
    NodeSpatialNeighborhood, TopologicalResidualEngine, DEFAULT_TOLERANCE_RATIO,
    MAX_BOUNDED_NEIGHBORS, MAX_SPATIAL_NEIGHBORS,
};
