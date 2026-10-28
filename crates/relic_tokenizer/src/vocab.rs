//! Gestion del vocabulario y reglas de fusion (merges) para BPE.
//! Compatible con tokenizer.json estandar. Menos de 180 lineas.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VocabError {
    #[error("Error de entrada/salida: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error al deserializar JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Formato de tokenizador no valido: falta seccion '{0}'")]
    InvalidFormat(String),
}

#[derive(Deserialize)]
struct TokenizerModelJson {
    vocab: HashMap<String, u32>,
    merges: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct TokenizerRootJson {
    model: Option<TokenizerModelJson>,
}

#[derive(Clone, Debug)]
pub struct Vocab {
    pub token_to_id: HashMap<String, u32>,
    pub id_to_token: HashMap<u32, String>,
    pub bpe_ranks: HashMap<(String, String), u32>,
    pub bos_id: Option<u32>,
    pub eos_id: Option<u32>,
    pub pad_id: Option<u32>,
}

impl Vocab {
    /// Carga el vocabulario y fusiones directamente desde un archivo tokenizer.json.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, VocabError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let root: TokenizerRootJson = serde_json::from_reader(reader)?;

        let model = root
            .model
            .ok_or_else(|| VocabError::InvalidFormat("model".into()))?;

        let token_to_id = model.vocab;
        let mut id_to_token = HashMap::with_capacity(token_to_id.len());
        for (k, &v) in &token_to_id {
            id_to_token.insert(v, k.clone());
        }

        let mut bpe_ranks = HashMap::new();
        if let Some(merges_raw) = model.merges {
            for (rank, item) in merges_raw.iter().enumerate() {
                let pair_str = match item {
                    serde_json::Value::String(s) => s.as_str(),
                    serde_json::Value::Array(arr) if arr.len() == 2 => {
                        let a = arr[0].as_str().unwrap_or("");
                        let b = arr[1].as_str().unwrap_or("");
                        bpe_ranks.insert((a.to_string(), b.to_string()), rank as u32);
                        continue;
                    }
                    _ => continue,
                };
                let parts: Vec<&str> = pair_str.split_whitespace().collect();
                if parts.len() == 2 {
                    bpe_ranks.insert((parts[0].to_string(), parts[1].to_string()), rank as u32);
                }
            }
        }

        let bos_id = token_to_id.get("<|bos|>").copied();
        let eos_id = token_to_id.get("<|eos|>").copied()
            .or_else(|| token_to_id.get("endoftext").copied())
            .or_else(|| token_to_id.get("<|endoftext|>").copied());
        let pad_id = token_to_id.get("<|pad|>").copied();

        Ok(Self {
            token_to_id,
            id_to_token,
            bpe_ranks,
            bos_id,
            eos_id,
            pad_id,
        })
    }

    /// Obtiene el tamano total del vocabulario.
    pub fn len(&self) -> usize {
        self.token_to_id.len()
    }

    /// Verifica si el vocabulario esta vacio.
    pub fn is_empty(&self) -> bool {
        self.token_to_id.is_empty()
    }

    /// Comprueba si un token corresponde al delimitador de fin de secuencia (EOS).
    pub fn is_eos(&self, token_id: u32) -> bool {
        if self.eos_id == Some(token_id) {
            return true;
        }
        if let Some(tok) = self.id_to_token.get(&token_id) {
            tok == "<|eos|>" || tok == "<|endoftext|>" || tok == "endoftext"
        } else {
            false
        }
    }
}
