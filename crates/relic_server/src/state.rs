//! # relic_server::state
//!
//! Estado compartido en memoria del servidor, modelo neuronal y grafo episódico.
//! Menos de 180 líneas.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use relic_cli::knowledge::load_default_knowledge;
use relic_core::{EpisodicMemory, RelicConfig, RelicModel, SafetensorsLoader};
use relic_tokenizer::{BpeTokenizer, Vocab};
use crate::types::Conversation;

#[derive(Clone)]
pub struct ServerState {
    pub config: RelicConfig,
    pub model: Option<Arc<RelicModel>>,
    pub tokenizer: Option<Arc<BpeTokenizer>>,
    pub memory: Arc<RwLock<EpisodicMemory>>,
    pub conversations: Arc<RwLock<HashMap<String, Conversation>>>,
    pub weights_loaded: bool,
    pub static_dir: Option<PathBuf>,
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

        let static_candidates = [
            PathBuf::from("../Simple_Domain/frontend/dist"),
            PathBuf::from("../../Simple_Domain/frontend/dist"),
            PathBuf::from("c:/Users/Nicolas Pachon/Desktop/Seis Ojos/Simple_Domain/frontend/dist"),
            PathBuf::from("dist"),
        ];
        let static_dir = static_candidates.into_iter().find(|p| p.is_dir());
        if let Some(ref dir) = static_dir {
            println!("[relic_server] Frontend compilado detectado en: {:?}", dir);
        }

        Self {
            config,
            model,
            tokenizer,
            memory: Arc::new(RwLock::new(memory)),
            conversations: Arc::new(RwLock::new(HashMap::new())),
            weights_loaded,
            static_dir,
        }
    }
}
