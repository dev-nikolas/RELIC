use relic_core::*;

#[test]
fn test_full_relic_model_forward_pass() {
    let cfg = RelicConfig {
        vocab_size: 64,
        hidden_size: 48,
        num_attention_heads: 2,
        num_key_value_heads: 1,
        head_dim: 24,
        intermediate_size: 96,
        num_hidden_layers: 2,
        ..Default::default()
    };

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
    let cfg = RelicConfig {
        vocab_size: 64,
        hidden_size: 48,
        num_attention_heads: 2,
        num_key_value_heads: 1,
        head_dim: 24,
        intermediate_size: 96,
        num_hidden_layers: 2,
        ..Default::default()
    };

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
    memory.insert_edge("N0", "N1");
    memory.insert_edge("N1", "N2");
    memory.insert_edge("N2", "N0");
    memory.insert_edge("N2", "N3");
    memory.insert_edge("N3", "N4");

    let res = ReasoningEngine::solve_reachability_bfs(&mut memory, "N0", "N4", 6);
    assert!(res.is_reachable);
    assert_eq!(res.path_length, 4);
    assert_eq!(res.path, vec!["N0", "N1", "N2", "N3", "N4"]);

    let res_no = ReasoningEngine::solve_reachability_bfs(&mut memory, "N4", "N0", 6);
    assert!(!res_no.is_reachable);
    assert_eq!(res_no.path_length, 0);

    let res_self = ReasoningEngine::solve_reachability_bfs(&mut memory, "N2", "N2", 6);
    assert!(res_self.is_reachable);
    assert_eq!(res_self.path_length, 0);
    assert_eq!(res_self.path, vec!["N2"]);
}


#[test]
fn test_alias_collision_is_ambiguous_not_silent_last_wins() {
    let mut memory = EpisodicMemory::new();
    memory.insert_edge("Bogotá", "Lima");
    memory.insert_edge("Bogota", "Medellín");

    assert!(!memory.contains_node("bogota"), "Nombre ambiguo debe abstenerse");
    match memory.resolve_node("bogota") {
        NodeResolution::Ambiguous(candidates) => {
            assert!(candidates.contains(&"Bogotá".to_string()));
            assert!(candidates.contains(&"Bogota".to_string()));
        }
        _ => panic!("Nodo canónicamente colisionado debe reportarse como Ambiguous"),
    }

    assert!(memory.contains_node("Bogotá"));
    memory.insert_edge("Toledo", "Madrid");
    assert!(memory.contains_node("toledo"));
}
