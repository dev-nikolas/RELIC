//! # relic_tokenizer
//!
//! Tokenizador BPE nativo en Rust puro para RELIC-10M.
//! Soporta mapeo biyectivo ByteLevel y lectura directa de tokenizer.json.
pub mod bpe;
pub mod byte_level;
pub mod vocab;

pub use bpe::BpeTokenizer;
pub use byte_level::{bytelevel_to_text, bytes_to_unicode, text_to_bytelevel};
pub use vocab::{Vocab, VocabError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input_encode() {
        let vocab = Vocab {
            token_to_id: std::collections::HashMap::new(),
            id_to_token: std::collections::HashMap::new(),
            bpe_ranks: std::collections::HashMap::new(),
            bos_id: None,
            eos_id: None,
            pad_id: None,
        };
        let tok = BpeTokenizer::new(vocab);
        let ids = tok.encode("");
        assert!(ids.is_empty());
    }

    #[test]
    fn test_spanish_roundtrip_with_artifacts() {
        let tok_path = std::path::Path::new("../../artifacts/tokenizer/tokenizer.json");
        if !tok_path.exists() {
            return;
        }
        let vocab = Vocab::from_file(tok_path).expect("Cargar vocabulario");
        let tok = BpeTokenizer::new(vocab);
        let sample = "El pingüino encontró una señal en Bogotá";
        let ids = tok.encode(sample);
        let decoded = tok.decode(&ids).expect("Decodificar");
        assert_eq!(sample, decoded);
    }
}
