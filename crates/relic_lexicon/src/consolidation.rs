//! Motor de Consolidación ("Sueño") y poda de episodios efímeros de bajo peso.
//! Preserva la inmutabilidad del log original emitiendo un `ConsolidationCheckpoint`.
//! Menos de 180 líneas.

use serde::{Deserialize, Serialize};
use crate::memory::{EngramStore, LexiconView};

/// Reporte determinista de consolidación en Sandbox antes de validar fidelidad en el Blackwall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsolidationCandidate {
    pub min_weight_threshold: f64,
    pub pruned_edges: Vec<(String, String)>,
    pub retained_edges_count: usize,
}

/// Identifica las aristas efímeras cuyo peso es estrictamente menor a `min_weight_threshold`
/// y proyecta el estado consolidado sin alterar la bitácora original.
pub fn compute_consolidation_candidate(
    store: &EngramStore,
    min_weight_threshold: f64,
) -> (Vec<(String, String)>, LexiconView) {
    let current = store.active_view();
    let mut pruned_edges: Vec<(String, String)> = current
        .active_facts
        .iter()
        .filter(|(_, (_, weight, _))| *weight < min_weight_threshold)
        .map(|((from, to), _)| (from.clone(), to.clone()))
        .collect();
    pruned_edges.sort();

    let mut sandbox = store.clone();
    sandbox.detach_backing_file();
    let pruned_view = if pruned_edges.is_empty() {
        current.clone()
    } else {
        let mut next_entries = sandbox.entries().to_vec();
        let next_ver = sandbox.current_version() + 1;
        let checkpoint = crate::log::LogEntry::new_sealed(
            next_ver,
            0,
            sandbox.head_hash(),
            crate::log::EntryStatus::ConsolidatedCheckpoint,
            crate::log::LexiconPayload::ConsolidationCheckpoint {
                min_weight_threshold,
                pruned_edges: pruned_edges.clone(),
                fidelity_score: 1.0,
            },
        );
        next_entries.push(checkpoint);
        EngramStore::rebuild_active_view(&next_entries).unwrap_or_else(|_| current.clone())
    };

    (pruned_edges, pruned_view)
}
