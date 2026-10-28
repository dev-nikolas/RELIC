//! Semilla Génesis auditable del engrama RELIC.
//! Migra las 13 aristas base a entradas formales `LogEntry` selladas con SHA-256.
//! Menos de 180 líneas.

use crate::log::{EntryStatus, LexiconPayload, LogEntry, GENESIS_PREV_HASH};

/// Las 13 relaciones base (espaciales y dependencias de crates) que conforman el bloque génesis.
pub const GENESIS_FACTS: &[(&str, &str, &str)] = &[
    // 1. Dominio Espacial
    ("Madrid", "conectado_a", "Barcelona"),
    ("Barcelona", "conectado_a", "París"),
    ("Madrid", "conectado_a", "Toledo"),
    ("Bogotá", "conectado_a", "Lima"),
    ("Lima", "conectado_a", "Buenos Aires"),
    ("Marta", "conectado_a", "estación"),
    ("estación", "conectado_a", "biblioteca"),
    ("nodo_A", "conectado_a", "nodo_B"),
    // 2. Dominio Dependencias de Software (RELIC Crates)
    ("relic_cli", "depende_de", "relic_core"),
    ("relic_core", "depende_de", "memmap2"),
    ("relic_cli", "depende_de", "relic_tokenizer"),
    ("relic_core", "depende_de", "serde"),
    ("relic_core", "depende_de", "thiserror"),
];

/// Construye la cadena génesis sellada con SHA-256 desde `GENESIS_PREV_HASH`.
pub fn build_genesis_entries() -> Vec<LogEntry> {
    let mut entries = Vec::with_capacity(GENESIS_FACTS.len());
    let mut prev_hash = GENESIS_PREV_HASH.to_string();

    for (idx, &(from, relation, to)) in GENESIS_FACTS.iter().enumerate() {
        let version = (idx + 1) as u64;
        let payload = LexiconPayload::graph_fact(from, relation, to);
        let entry = LogEntry::new_sealed(
            version,
            0,
            prev_hash,
            EntryStatus::ConfirmedFact,
            payload,
        );
        prev_hash = entry.entry_hash.clone();
        entries.push(entry);
    }

    entries
}

/// Exporta las entradas génesis en formato JSONL canónico (una entrada por línea).
pub fn genesis_jsonl_seed() -> String {
    let entries = build_genesis_entries();
    let mut lines = Vec::with_capacity(entries.len());
    for entry in entries {
        lines.push(serde_json::to_string(&entry).expect("Serialización génesis infalible"));
    }
    lines.join("\n") + "\n"
}

