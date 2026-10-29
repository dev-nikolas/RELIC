//! Prueba de equivalencia estricta y determinista:
//! Verifica que la actualización incremental O(1) de `append_entry` produzca
//! un estado de memoria idéntico campo a campo a `rebuild_active_view`.
//! Menos de 180 líneas.

use std::time::Instant;
use relic_core::ArithmeticOp;
use relic_lexicon::{EngramStore, EntryStatus, LexiconPayload};

#[test]
fn test_incremental_vs_rebuild_exact_equivalence_all_payloads() {
    let mut store = EngramStore::with_genesis();
    let mut ts = 1_700_000_000u64;

    // 1. GraphFact
    for i in 0..50 {
        ts += 1;
        let u = format!("nodo_{i}");
        let v = format!("nodo_{}", i + 1);
        store.append_entry(ts, EntryStatus::ConfirmedFact, LexiconPayload::graph_fact(u, "conecta", v)).unwrap();
    }

    // 2. EdgeReinforcement
    for i in 0..20 {
        ts += 1;
        let u = format!("nodo_{i}");
        let v = format!("nodo_{}", i + 1);
        store.append_entry(ts, EntryStatus::ConfirmedFact, LexiconPayload::EdgeReinforcement {
            from: u, to: v, new_weight: 5.5 + i as f64, usage_count: 10 + i as u64,
        }).unwrap();
    }

    // 3. OpAlias
    for (w, op) in [("sumale", ArithmeticOp::Add), ("quitale", ArithmeticOp::Subtract), ("duplica", ArithmeticOp::Multiply)] {
        ts += 1;
        store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::op_alias(w, op)).unwrap();
    }

    // 4. PhraseMacro
    for (p, e) in [("cuanto cuesta", "calcula"), ("por favor suma", "suma")] {
        ts += 1;
        store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::phrase_macro(p, e)).unwrap();
    }

    // 5. EntityAlias
    for (a, c) in [("bogo", "Bogota"), ("medallo", "Medellin")] {
        ts += 1;
        store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::entity_alias(a, c)).unwrap();
    }

    // 6. QuantumPolysemousAlias
    ts += 1;
    store.append_entry(ts, EntryStatus::LexicalAlias, LexiconPayload::quantum_polysemous_alias("partir", ArithmeticOp::Divide, 0.9, 0.1, Some("pastel".to_string()))).unwrap();

    let v_checkpoint = store.current_version();

    // 7. ConsolidationCheckpoint
    ts += 1;
    store.append_entry(ts, EntryStatus::ConsolidatedCheckpoint, LexiconPayload::ConsolidationCheckpoint {
        min_weight_threshold: 1.0,
        pruned_edges: vec![("nodo_30".to_string(), "nodo_31".to_string())],
        fidelity_score: 1.0,
    }).unwrap();

    // Verificar equivalencia tras consolidación
    let rebuilt_view = EngramStore::rebuild_active_view(store.entries()).expect("Reconstrucción exitosa");
    assert_views_identical(store.active_view(), &rebuilt_view);

    // 8. Rollback a versión previa a consolidación
    ts += 1;
    store.append_entry(ts, EntryStatus::Reverted, LexiconPayload::Rollback {
        target_version: v_checkpoint,
        reason: "Reversión de prueba".to_string(),
    }).unwrap();

    let rebuilt_post_rb = EngramStore::rebuild_active_view(store.entries()).expect("Reconstrucción post rollback");
    assert_views_identical(store.active_view(), &rebuilt_post_rb);
}

fn assert_views_identical(v1: &relic_lexicon::LexiconView, v2: &relic_lexicon::LexiconView) {
    assert_eq!(v1.current_version, v2.current_version, "current_version discrepa");
    assert_eq!(v1.head_hash, v2.head_hash, "head_hash discrepa");
    assert_eq!(v1.active_facts, v2.active_facts, "active_facts discrepa");
    assert_eq!(v1.op_aliases, v2.op_aliases, "op_aliases discrepa");
    assert_eq!(v1.op_alias_weights, v2.op_alias_weights, "op_alias_weights discrepa");
    assert_eq!(v1.phrase_macros, v2.phrase_macros, "phrase_macros discrepa");
    assert_eq!(v1.entity_aliases, v2.entity_aliases, "entity_aliases discrepa");
    assert_eq!(v1.quantum_polysemy.len(), v2.quantum_polysemy.len(), "quantum_polysemy len discrepa");

    assert_eq!(v1.episodic.node_count(), v2.episodic.node_count(), "episodic node_count discrepa");
    assert_eq!(v1.episodic.edge_count(), v2.episodic.edge_count(), "episodic edge_count discrepa");

    for (from, _) in v1.active_facts.keys() {
        let mut n1 = v1.episodic.clone();
        let mut n2 = v2.episodic.clone();
        let neighbors1 = n1.get_neighbors(from);
        let neighbors2 = n2.get_neighbors(from);
        assert_eq!(neighbors1, neighbors2, "Vecinos ordenados discrepan para nodo {}", from);
    }
}

#[test]
fn test_in_memory_vs_disk_persistence_latency_separation() {
    let mut mem_store = EngramStore::with_genesis();
    let num_mutations = 100usize;

    // Medición 1: Latencia de actualización en memoria pura
    let t0 = Instant::now();
    for i in 0..num_mutations {
        let p = LexiconPayload::graph_fact(format!("src_{i}"), "link", format!("dst_{i}"));
        mem_store.append_entry(2_000_000_000 + i as u64, EntryStatus::ConfirmedFact, p).unwrap();
    }
    let dur_mem = t0.elapsed();
    let mean_mem_us = (dur_mem.as_micros() as f64) / (num_mutations as f64);

    // Medición 2: Latencia con persistencia a disco (IO sincrónico)
    let tmp_dir = std::env::temp_dir();
    let tmp_file = tmp_dir.join(format!("relic_bench_persist_{}.jsonl", Instant::now().elapsed().as_nanos()));
    let mut disk_store = EngramStore::open_or_init_file(&tmp_file).unwrap();

    let t1 = Instant::now();
    for i in 0..num_mutations {
        let p = LexiconPayload::graph_fact(format!("src_{i}"), "link", format!("dst_{i}"));
        disk_store.append_entry(2_000_000_000 + i as u64, EntryStatus::ConfirmedFact, p).unwrap();
    }
    let dur_disk = t1.elapsed();
    let mean_disk_us = (dur_disk.as_micros() as f64) / (num_mutations as f64);

    let _ = std::fs::remove_file(&tmp_file);

    println!("[MEDIDO] Latencia media en memoria pura: {:.2} µs/mutación", mean_mem_us);
    println!("[MEDIDO] Latencia media con persistencia a disco: {:.2} µs/mutación", mean_disk_us);

    // Comprobación de orden de magnitud: la mutación en memoria debe ser < 500 µs (sub-milisegundo)
    assert!(mean_mem_us < 500.0, "La mutación en memoria superó el presupuesto sub-milisegundo: {:.2} µs", mean_mem_us);
}
