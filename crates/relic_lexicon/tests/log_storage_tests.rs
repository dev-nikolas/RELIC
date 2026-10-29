//! # log_storage_tests
//!
//! Pruebas de almacenamiento de log fuera de RAM, reconstrucción de offsets,
//! reversión y diff de versiones inmediatos.
//! Menos de 180 líneas.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use relic_lexicon::{EngramStore, EntryStatus, LexiconPayload};

fn temp_log_path() -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("relic_log_test_{}.jsonl", nanos))
}

#[test]
fn test_append_followed_by_immediate_rollback_and_diff() {
    let path = temp_log_path();
    let mut store = EngramStore::open_or_init_file(&path).expect("abrir store");
    let base_v = store.active_view().current_version;
    let base_facts = store.active_view().active_facts.len();

    // 1. Append de hecho inicial (versión base_v + 1)
    let p1 = LexiconPayload::graph_fact("Alice", "knows", "Bob");
    store.append_entry(1001, EntryStatus::ConfirmedFact, p1).expect("append fact 1");
    assert_eq!(store.active_view().current_version, base_v + 1);

    // 2. Append de segundo hecho (versión base_v + 2)
    let p2 = LexiconPayload::graph_fact("Bob", "knows", "Charlie");
    store.append_entry(1002, EntryStatus::ConfirmedFact, p2).expect("append fact 2");
    assert_eq!(store.active_view().current_version, base_v + 2);
    assert_eq!(store.active_view().active_facts.len(), base_facts + 2);

    // 3. Rollback inmediato a versión base_v + 1
    let rollback_entry = store.rollback_to(base_v + 1, 1003, "test_rollback").expect("rollback exitoso");
    assert_eq!(rollback_entry.version, base_v + 3);
    assert_eq!(store.active_view().current_version, base_v + 3);
    assert_eq!(store.active_view().active_facts.len(), base_facts + 1);
    assert!(store.active_view().active_facts.contains_key(&("Alice".into(), "Bob".into())));

    // 4. diff_versions inmediato entre v_base+1 y v_base+2, y v_base+2 y v_base+3
    let diff_1_2 = store.diff_versions(base_v + 1, base_v + 2).expect("diff v+1 vs v+2");
    assert_eq!(diff_1_2.added_facts.len(), 1);
    assert_eq!(diff_1_2.added_facts[0].2, "Charlie");

    let diff_2_3 = store.diff_versions(base_v + 2, base_v + 3).expect("diff v+2 vs v+3");
    assert_eq!(diff_2_3.removed_facts.len(), 1);
    assert_eq!(diff_2_3.removed_facts[0].2, "Charlie");

    let _ = fs::remove_file(path);
}

#[test]
fn test_reopen_existing_log_reconstructs_entry_offsets() {
    let path = temp_log_path();
    let initial_count;

    // Sesión 1: escribir 5 entradas
    {
        let mut store = EngramStore::open_or_init_file(&path).expect("abrir store inicial");
        initial_count = store.entry_count();
        for i in 1..=5 {
            let p = LexiconPayload::entity_alias(format!("alias_{i}"), format!("canon_{i}"));
            store.append_entry(2000 + i, EntryStatus::LexicalAlias, p).expect("append alias");
        }
        assert_eq!(store.entry_count(), initial_count + 5);
    }

    // Sesión 2: reabrir desde archivo existente y verificar offsets
    {
        let store = EngramStore::open_or_init_file(&path).expect("reabrir store");
        assert_eq!(store.entry_count(), initial_count + 5);
        assert_eq!(store.entry_offsets().len(), initial_count + 5);

        // Cada offset debe ser estrictamente monótono creciente
        for i in 1..store.entry_offsets().len() {
            assert!(store.entry_offsets()[i] > store.entry_offsets()[i - 1]);
        }

        // Leer cada entrada por offset de disco
        for idx in 0..store.entry_count() {
            let entry = store.read_entry_at(idx).expect("leer entrada por offset");
            assert_eq!(entry.version, (idx + 1) as u64);
        }
    }

    let _ = fs::remove_file(path);
}

#[test]
fn test_read_last_entry_immediately_after_bufwriter_append() {
    let path = temp_log_path();
    let mut store = EngramStore::open_or_init_file(&path).expect("abrir store");
    let base_v = store.active_view().current_version;

    // Escribir 10 hechos consecutivamente
    for i in 0..10 {
        let p = LexiconPayload::graph_fact(format!("src_{i}"), "rel", format!("dst_{i}"));
        store.append_entry(3000 + i, EntryStatus::ConfirmedFact, p).expect("append");

        // Leer inmediatamente la última entrada por offset en disco para verificar flush
        let last_idx = store.entry_count() - 1;
        let last_entry = store.read_entry_at(last_idx).expect("lectura inmediata tras flush");
        assert_eq!(last_entry.version, base_v + 1 + i);
        if let LexiconPayload::GraphFact { from, .. } = last_entry.payload {
            assert_eq!(from, format!("src_{i}"));
        } else {
            panic!("Payload no coincide");
        }
    }

    let _ = fs::remove_file(path);
}
