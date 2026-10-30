//! # Local Constraint Repair & UnsatCore Diagnostic
//!
//! Verificación formal en tiempo de ejecución, extracción de contraejemplos mínimos
//! y proyección local determinista a la frontera convexa admisible en < 15 µs.

pub mod types;
pub mod envelope;
pub mod engine;

pub use types::{
    ConstraintViolation, EdgeOperatingBounds, UnsatCore, ViolationType, MAX_VIOLATIONS,
};
pub use envelope::{
    EnvelopeStatus, RepairOutcome, StagedEnvelope, StagedOperation, MAX_STAGED_OPS,
};
pub use engine::LocalConstraintRepairer;

