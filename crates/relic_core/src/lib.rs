//! # relic_core
//!
//! Motor neuronal e inferencia para RELIC-10M en Rust nativo.
//! Diseno modular, cero dependencias C++, multiplataforma.
pub mod arithmetic;
pub mod anomaly;
pub mod attention;
pub mod blackwall_scars;
pub mod block;
pub mod certificate;
pub mod compact_adj;
pub mod config;
pub mod contingency_bfs;
pub mod episodic_memory;
pub mod hash;
pub mod interner;
pub mod kuramoto;
pub mod kv_cache;
pub mod loader;
pub mod mlp;
pub mod model;
pub mod model_mmap;
pub mod mask256;
pub mod ms_bfs;
pub mod network_audit;
pub mod node_resolution;
pub mod norm;
pub mod ops;
pub mod quantum;
pub mod reasoning_engine;
pub mod repair;
pub mod rir;
pub mod rotary;
pub mod semantic_proposer;
pub mod sampler;
pub mod search_buffer;
pub mod spectral;
pub mod spectral_sprt;
pub mod spreading_activation;
pub mod tactical_dispatch;
pub mod temporal_graph;

pub use arithmetic::{ArithmeticError, ArithmeticOp, ArithmeticPlan, ArithmeticReduction};
pub use blackwall_scars::{
    Avx2PredicateBlock, CriticalEdgeKey, EdgeScarSlot, LocalScarRule, NeighborTopologyToken,
    StructuralDiscriminationEngine, WL1Signature, MAX_SCARS_PER_EDGE, MAX_TRACKED_EDGES,
};
pub use compact_adj::CompactAdj;
pub use contingency_bfs::{
    BitAdjacencyGraph, CausalReachabilityFilter, ContingencyOutcome, PreCertifiedContingencyLibrary,
    PreCertifiedTopology, SwitchAction, SwitchActionType, MAX_CONTINGENCIES, MAX_GRAPH_NODES,
};
pub use interner::NodeInterner;
pub use kuramoto::{
    compute_kuramoto_order_parameter, simulate_kuramoto_synchronization, wrap_angle,
    KuramotoNetworkSpec, KuramotoOscillator, KuramotoTrace, SynchronizationOutcome,
};
pub use quantum::{
    compute_asymmetric_threshold, CollapseDecision, Complex, InterferenceStepTrace,
    QuantumSuperposition,
};
pub use repair::{
    ConstraintViolation, EdgeOperatingBounds, EnvelopeStatus, LocalConstraintRepairer,
    RepairOutcome, StagedEnvelope, StagedOperation, UnsatCore, ViolationType,
    MAX_STAGED_OPS, MAX_VIOLATIONS,
};
pub use spectral::{
    analyze_spectrum_donoho_stark, compute_destination_entropy, compute_dft_exact,
    compute_syn_ack_ratio, pearson_correlation_welford, verify_network_alert, NetworkAlertSpec,
    SpectralOutcome, VerifiedAlertReport,
};
pub use spectral_sprt::{SprtBeaconDetector, SprtConfig, SprtOutcome};
pub use spreading_activation::{
    compute_shannon_entropy, propagate_activation, ActivationStatus, ActivationStepTrace,
    SpreadingActivationResult,
};
pub use tactical_dispatch::{
    CachePaddedAtomicUsize, CircuitBreaker, ReflexTickInput, StagedOrderRingBuffer, StagedPolicySlot,
    TacticalDecision, TacticalKernelContext, TacticalReflexKernel, TacticalReflexOrchestrator,
    VetoReason, RING_CAPACITY, RING_MASK,
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
pub use model_mmap::{RelicLayerMmap, RelicModelMmap};
pub use norm::RMSNorm;
pub use certificate::{
    verify_negative, verify_negative_backward, verify_positive, verify_unreachable,
    ReachabilityResult, SearchOutcome,
};
pub use ms_bfs::{solve_ms_bfs_batch, solve_ms_bfs_counterfactual, Mask256, MsBfsResult};
pub use reasoning_engine::ReasoningEngine;
pub use rotary::RotaryEmbedding;
pub use sampler::Sampler;
pub use semantic_proposer::propose_rir_from_text;
