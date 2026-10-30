//! Pruebas de consistencia de mutación y reemplazo de hechos (MQuAKE-Style).
//! Criterio: 100 reemplazos consecutivos en cadena transitiva sin aristas fantasma.
use relic_lexicon::{EngramStore, EntryStatus, LexiconPayload};

#[test]
fn test_100_consecutive_fact_replacements_zero_ghost_edges() {
    let mut mem = EngramStore::new_empty();

    // 1. Construir una cadena transitiva inicial de 100 hechos: A_i -> A_{i+1}
    for i in 0..100 {
        let from = format!("Node_{i}");
        let to = format!("Node_{}", i + 1);
        let payload = LexiconPayload::graph_fact(from, "connects", to);
        mem.append_entry(1000 + i as u64, EntryStatus::ConfirmedFact, payload)
            .expect("append inicial");
    }

    assert_eq!(mem.active_view().active_facts.len(), 100);
    assert_eq!(mem.active_view().episodic.edge_count(), 100);

    // 2. Ejecutar 100 reemplazos consecutivos: Node_i -> Node_{i+1} se reemplaza por Node_i -> Repl_{i+1}
    for i in 0..100 {
        let from = format!("Node_{i}");
        let old_to = format!("Node_{}", i + 1);
        let new_to = format!("Repl_{}", i + 1);
        let payload = LexiconPayload::graph_fact_replacing(
            &from,
            "connects",
            &new_to,
            (from.clone(), "connects".to_string(), old_to.clone()),
        );

        mem.append_entry(2000 + i as u64, EntryStatus::ConfirmedFact, payload)
            .expect("append reemplazo");

        // Verificación inmediata de ausencia de aristas fantasma:
        // La arista vieja NO debe existir ni en active_facts ni en episodic
        assert!(
            !mem.active_view().active_facts.contains_key(&(from.clone(), old_to.clone())),
            "Arista obsoleta {from} -> {old_to} sigue viva en active_facts en iteración {i}"
        );
        let nbrs = mem.active_view().episodic.get_neighbors_u32(
            mem.active_view().episodic.interner.get_id(&from).unwrap(),
        );
        let old_to_id = mem.active_view().episodic.interner.get_id(&old_to);
        if let Some(oid) = old_to_id {
            assert!(
                !nbrs.contains(&oid),
                "Arista fantasma {from} -> {old_to} persiste físicamente en EpisodicMemory tras reemplazo"
            );
        }

        // La nueva arista DEBE existir
        assert!(mem.active_view().active_facts.contains_key(&(from.clone(), new_to.clone())));
    }

    // 3. Verificación final de consistencia
    assert_eq!(mem.active_view().active_facts.len(), 100);
    assert_eq!(mem.active_view().episodic.edge_count(), 100);
}

#[test]
fn test_fact_replacement_rollback_preserves_prior_edge() {
    let mut mem = EngramStore::with_genesis();
    let base_v = mem.active_view().current_version;

    // Hecho base: Alpha -> Beta
    mem.append_entry(
        1000,
        EntryStatus::ConfirmedFact,
        LexiconPayload::graph_fact("Alpha", "leads_to", "Beta"),
    ).expect("append base");

    // Reemplazo: Alpha -> Gamma (reemplaza Alpha -> Beta)
    mem.append_entry(
        1001,
        EntryStatus::ConfirmedFact,
        LexiconPayload::graph_fact_replacing("Alpha", "leads_to", "Gamma", ("Alpha".into(), "leads_to".into(), "Beta".into())),
    ).expect("append reemplazo");

    assert!(mem.active_view().active_facts.contains_key(&("Alpha".to_string(), "Gamma".to_string())));
    assert!(!mem.active_view().active_facts.contains_key(&("Alpha".to_string(), "Beta".to_string())));

    // Reversión mediante rollback a la versión del hecho base (base_v + 1)
    mem.rollback_to(base_v + 1, 1002, "deshacer reemplazo").expect("rollback exitoso");

    // Ahora Alpha -> Beta DEBE estar restaurada y Alpha -> Gamma eliminada
    assert!(mem.active_view().active_facts.contains_key(&("Alpha".to_string(), "Beta".to_string())));
    assert!(!mem.active_view().active_facts.contains_key(&("Alpha".to_string(), "Gamma".to_string())));
}
