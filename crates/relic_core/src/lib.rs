//! # relic_core
//!
//! Motor neuronal e inferencia para RELIC-10M en Rust nativo.
//! Diseno modular, cero dependencias C++, multiplataforma.
//! Menos de 180 lineas.

pub mod arithmetic;
pub mod attention;
pub mod block;
pub mod config;
pub mod episodic_memory;
pub mod hash;
pub mod kv_cache;
pub mod loader;
pub mod mlp;
pub mod model;
pub mod network_audit;
pub mod norm;
pub mod ops;
pub mod quantum;
pub mod reasoning_engine;
pub mod rir;
pub mod rotary;
pub mod sampler;
pub mod spectral;
pub mod spreading_activation;
pub mod temporal_graph;

pub use arithmetic::{ArithmeticError, ArithmeticOp, ArithmeticPlan, ArithmeticReduction};
pub use quantum::{
    compute_asymmetric_threshold, CollapseDecision, Complex, InterferenceStepTrace,
    QuantumSuperposition,
};
pub use spectral::{
    analyze_spectrum_donoho_stark, compute_destination_entropy, compute_dft_exact,
    compute_syn_ack_ratio, pearson_correlation_welford, verify_network_alert, NetworkAlertSpec,
    SpectralOutcome, VerifiedAlertReport,
};
pub use spreading_activation::{
    compute_shannon_entropy, propagate_activation, ActivationStatus, ActivationStepTrace,
    SpreadingActivationResult,
};
pub use temporal_graph::{TemporalEdge, TemporalGraph};
pub use rir::{
    ArithmeticStep, DomainKind as RirDomainKind, IntentKind as RirIntentKind,
    RirArithmeticOp, RirDocumentV1, RirValidationError, UncertaintyReport as RirUncertaintyReport,
    ValidityStatus as RirValidityStatus,
};
pub use attention::RelicAttention;
pub use block::RelicBlock;
pub use config::RelicConfig;
pub use episodic_memory::{EdgeStats, EpisodicMemory, NodeResolution};
pub use hash::{sha256_hex, Sha256};
pub use kv_cache::{KvCache, LayerKvCache};
pub use loader::SafetensorsLoader;
pub use mlp::SwiGluMlp;
pub use model::RelicModel;
pub use norm::RMSNorm;
pub use reasoning_engine::{ReachabilityResult, ReasoningEngine};
pub use rotary::RotaryEmbedding;
pub use sampler::Sampler;
