//! # relic_server::state
//!
//! Estado de inferencia compartido en memoria (modelo y grafo episódico). Sin estado conversacional.
//! Menos de 180 líneas.

use std::path::Path;
use std::sync::{Arc, RwLock};
use relic_cli::knowledge::load_default_knowledge;
use relic_core::{EpisodicMemory, RelicConfig, RelicModel, SafetensorsLoader};
use relic_tokenizer::{BpeTokenizer, Vocab};

#[derive(Clone)]
pub struct ServerState {
    pub config: RelicConfig,
    pub model: Option<Arc<RelicModel>>,
    pub tokenizer: Option<Arc<BpeTokenizer>>,
    pub memory: Arc<RwLock<EpisodicMemory>>,
    pub weights_loaded: bool,
}

impl ServerState {
    pub fn new() -> Self {
        let config = RelicConfig::default();
        let mut memory = EpisodicMemory::new();
        load_default_knowledge(&mut memory);

        let weights_path = Path::new("artifacts/weights/relic_10m_fp16.safetensors");
        let (model, weights_loaded) = if weights_path.exists() {
            let mut m = RelicModel::new(config.clone());
            if let Ok(loader) = SafetensorsLoader::open(weights_path) {
                let _ = loader.load_into_model(&mut m);
                println!("[relic_server] Pesos Safetensors cargados exitosamente.");
                (Some(Arc::new(m)), true)
            } else {
                eprintln!("[relic_server] Error al mapear archivo safetensors.");
                (None, false)
            }
        } else {
            println!("[relic_server] Modo degradado: weights safetensors ausentes.");
            (None, false)
        };

        let tok_path = Path::new("artifacts/tokenizer/tokenizer.json");
        let tokenizer = if tok_path.exists() {
            Vocab::from_file(tok_path).ok().map(|v| Arc::new(BpeTokenizer::new(v)))
        } else {
            None
        };

        Self {
            config,
            model,
            tokenizer,
            memory: Arc::new(RwLock::new(memory)),
            weights_loaded,
        }
    }
}
