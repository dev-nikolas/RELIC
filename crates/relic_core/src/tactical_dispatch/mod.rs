//! # Tactical Dispatch & Double-Loop Architecture
//!
//! Despacho táctico, búfer SPSC libre de bloqueos, disyuntor lógico y orquestación
//! del bucle reflejo de microsegundos.

pub mod ring_buffer;
pub mod circuit_breaker;
pub mod context;
pub mod kernel;

pub use ring_buffer::{
    CachePaddedAtomicUsize, StagedOrderRingBuffer, StagedPolicySlot, RING_CAPACITY, RING_MASK,
};
pub use circuit_breaker::{CircuitBreaker, TacticalDecision, VetoReason};
pub use context::{ReflexTickInput, TacticalKernelContext};
pub use kernel::{TacticalReflexKernel, TacticalReflexOrchestrator};

