

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
pub mod kv_cache;
pub mod loader;
pub mod mlp;
pub mod model;
pub mod norm;
pub mod ops;
pub mod reasoning_engine;
pub mod rir;
pub mod rotary;
pub mod sampler;
pub mod temporal_graph;

pub use arithmetic::{ArithmeticError, ArithmeticOp, ArithmeticPlan};
pub use temporal_graph::{TemporalEdge, TemporalGraph};
pub use rir::{
    ArithmeticStep, DomainKind as RirDomainKind, IntentKind as RirIntentKind,
    RirArithmeticOp, RirDocumentV1, RirValidationError, UncertaintyReport as RirUncertaintyReport,
    ValidityStatus as RirValidityStatus,
};
pub use attention::RelicAttention;
pub use block::RelicBlock;
pub use config::RelicConfig;
pub use episodic_memory::EpisodicMemory;
pub use kv_cache::{KvCache, LayerKvCache};
pub use loader::SafetensorsLoader;
pub use mlp::SwiGluMlp;
pub use model::RelicModel;
pub use norm::RMSNorm;
pub use reasoning_engine::{AtomicOperator, ReachabilityResult, ReasoningEngine, RuleCatalog, RuleError};
pub use rotary::RotaryEmbedding;
pub use sampler::Sampler;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_relic_model_forward_pass() {
        let mut cfg = RelicConfig::default();
        cfg.vocab_size = 64;
        cfg.hidden_size = 48;
        cfg.num_attention_heads = 2;
        cfg.num_key_value_heads = 1;
        cfg.head_dim = 24;
        cfg.intermediate_size = 96;
        cfg.num_hidden_layers = 2;

        let model = RelicModel::new(cfg.clone());
        let input_tokens = [1u32, 5u32, 12u32];
        let mut logits = vec![0.0f32; input_tokens.len() * cfg.vocab_size];

        model.forward(&input_tokens, &mut logits);

        assert_eq!(logits.len(), 3 * 64);
        for &val in &logits {
            assert!(!val.is_nan(), "Logit no debe ser NaN");
            assert!(!val.is_infinite(), "Logit debe ser finito");
        }
    }

    #[test]
    fn test_kv_cache_decode_step() {
        let mut cfg = RelicConfig::default();
        cfg.vocab_size = 64;
        cfg.hidden_size = 48;
        cfg.num_attention_heads = 2;
        cfg.num_key_value_heads = 1;
        cfg.head_dim = 24;
        cfg.intermediate_size = 96;
        cfg.num_hidden_layers = 2;

        let model = RelicModel::new(cfg.clone());
        let mut cache = KvCache::new(cfg.num_hidden_layers, 32, cfg.num_key_value_heads, cfg.head_dim);
        let mut logits = vec![0.0f32; cfg.vocab_size];

        model.forward_decode_step(5, 0, &mut cache, &mut logits);
        assert_eq!(cache.current_len, 1);
        assert!(!logits[0].is_nan());
    }

    #[test]
    fn test_episodic_memory_insert_and_neighbors() {
        let mut memory = EpisodicMemory::new();
        assert!(memory.insert_edge("N0", "N1"));
        assert!(!memory.insert_edge("N0", "N1"), "Duplicado debe ser rechazado");
        assert!(memory.insert_edge("N0", "N2"));

        assert_eq!(memory.node_count(), 1);
        assert_eq!(memory.edge_count(), 2);

        let neighbors = memory.get_neighbors("N0");
        assert_eq!(neighbors, &["N1".to_string(), "N2".to_string()]);
        assert_eq!(memory.get_neighbors("N3").len(), 0);

        memory.clear();
        assert_eq!(memory.node_count(), 0);
        assert_eq!(memory.edge_count(), 0);
    }

    #[test]
    fn test_reasoning_bfs_path_and_cycles() {
        let mut memory = EpisodicMemory::new();
        // Ciclo: N0 -> N1 -> N2 -> N0, y bifurcación N2 -> N3 -> N4
        memory.insert_edge("N0", "N1");
        memory.insert_edge("N1", "N2");
        memory.insert_edge("N2", "N0");
        memory.insert_edge("N2", "N3");
        memory.insert_edge("N3", "N4");

        // Caso 1: Alcanzable con ciclo intermedio
        let res = ReasoningEngine::solve_reachability_bfs(&mut memory, "N0", "N4", 6);
        assert!(res.is_reachable);
        assert_eq!(res.path_length, 4);
        assert_eq!(res.path, vec!["N0", "N1", "N2", "N3", "N4"]);

        // Caso 2: Inalcanzable
        let res_no = ReasoningEngine::solve_reachability_bfs(&mut memory, "N4", "N0", 6);
        assert!(!res_no.is_reachable);
        assert_eq!(res_no.path_length, 0);

        // Caso 3: Mismo nodo (distancia 0)
        let res_self = ReasoningEngine::solve_reachability_bfs(&mut memory, "N2", "N2", 6);
        assert!(res_self.is_reachable);
        assert_eq!(res_self.path_length, 0);
        assert_eq!(res_self.path, vec!["N2"]);
    }

    #[test]
    fn test_rule_catalog_atomic_and_composition() {
        let catalog = RuleCatalog::default();

        // Operador 1: Reverse
        let rev = catalog.apply_operator("Relic", AtomicOperator::Reverse).unwrap();
        assert_eq!(rev, "cileR");

        // Operador 2: SwapCase
        let swap = catalog.apply_operator("aBc", AtomicOperator::SwapCase).unwrap();
        assert_eq!(swap, "AbC");

        // Operador 3: DuplicateVowels
        let dup = catalog.apply_operator("relic", AtomicOperator::DuplicateVowels).unwrap();
        assert_eq!(dup, "reeliic");

        // Composición de 2 pasos: Reverse -> SwapCase
        let comp2 = catalog.compose("Relic", &[AtomicOperator::Reverse, AtomicOperator::SwapCase]).unwrap();
        assert_eq!(comp2, "CILEr");

        // Validación de entrada vacía
        assert_eq!(catalog.apply_operator("", AtomicOperator::Reverse), Err(RuleError::EmptyInput));

        // Validación de profundidad máxima excedida
        let ops = vec![AtomicOperator::Reverse; 9];
        assert_eq!(catalog.compose("test", &ops), Err(RuleError::MaxDepthExceeded(9, 8)));
    }
}
