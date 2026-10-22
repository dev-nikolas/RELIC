//! Inicializador de hechos del mundo en la memoria episódica.
//! Menos de 180 líneas.

use relic_core::EpisodicMemory;

pub fn load_default_knowledge(memory: &mut EpisodicMemory) {
    // 1. Dominio Espacial
    memory.insert_edge("Madrid", "Barcelona");
    memory.insert_edge("Barcelona", "París");
    memory.insert_edge("Madrid", "Toledo");
    memory.insert_edge("Bogotá", "Lima");
    memory.insert_edge("Lima", "Buenos Aires");
    memory.insert_edge("Marta", "estación");
    memory.insert_edge("estación", "biblioteca");
    memory.insert_edge("nodo_A", "nodo_B");

    // 2. Dominio Dependencias de Software (RELIC Crates)
    memory.insert_edge("relic_cli", "relic_core");
    memory.insert_edge("relic_core", "memmap2");
    memory.insert_edge("relic_cli", "relic_tokenizer");
    memory.insert_edge("relic_core", "serde");
    memory.insert_edge("relic_core", "thiserror");
}
