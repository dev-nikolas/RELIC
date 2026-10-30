//! Inicializador de hechos del mundo en la memoria episódica a partir de la semilla génesis
//! verificada de `relic_lexicon` (cero aristas imperativas hardcodeadas en CLI).
use relic_core::EpisodicMemory;
use relic_lexicon::EngramStore;

pub fn load_default_knowledge(memory: &mut EpisodicMemory) {
    let engram = EngramStore::with_genesis();
    *memory = engram.active_view().episodic.clone();
}
