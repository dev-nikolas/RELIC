//! Algoritmo de tokenizacion Byte-Pair Encoding (BPE) determinista.
//! Menos de 180 lineas.

use std::collections::HashMap;
use crate::byte_level::{bytelevel_to_text, bytes_to_unicode, text_to_bytelevel};
use crate::vocab::Vocab;

pub struct BpeTokenizer {
    vocab: Vocab,
    byte_to_char: HashMap<u8, char>,
    char_to_byte: HashMap<char, u8>,
}

impl BpeTokenizer {
    pub fn new(vocab: Vocab) -> Self {
        let (byte_to_char, char_to_byte) = bytes_to_unicode();
        Self {
            vocab,
            byte_to_char,
            char_to_byte,
        }
    }

    /// Aplica fusiones BPE a una palabra ya mapeada a nivel de bytes.
    fn bpe_merge(&self, piece: &str) -> Vec<String> {
        let mut parts: Vec<String> = piece.chars().map(|c| c.to_string()).collect();
        if parts.len() <= 1 {
            return parts;
        }

        loop {
            let mut min_rank = u32::MAX;
            let mut best_pair_idx = None;

            for i in 0..parts.len() - 1 {
                let pair = (parts[i].clone(), parts[i + 1].clone());
                if let Some(&rank) = self.vocab.bpe_ranks.get(&pair) {
                    if rank < min_rank {
                        min_rank = rank;
                        best_pair_idx = Some(i);
                    }
                }
            }

            match best_pair_idx {
                Some(idx) => {
                    let merged = format!("{}{}", parts[idx], parts[idx + 1]);
                    parts[idx] = merged;
                    parts.remove(idx + 1);
                    if parts.len() <= 1 {
                        break;
                    }
                }
                None => break,
            }
        }

        parts
    }

    /// Codifica un texto en una lista de token IDs enteros.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        if text.is_empty() {
            return Vec::new();
        }

        let byte_repr = text_to_bytelevel(text, &self.byte_to_char);
        let pieces = self.bpe_merge(&byte_repr);

        let mut ids = Vec::with_capacity(pieces.len());
        for p in pieces {
            if let Some(&id) = self.vocab.token_to_id.get(&p) {
                ids.push(id);
            } else {
                // Fallback a nivel de caracteres de bytes individuales
                for c in p.chars() {
                    let cs = c.to_string();
                    if let Some(&cid) = self.vocab.token_to_id.get(&cs) {
                        ids.push(cid);
                    }
                }
            }
        }
        ids
    }

    /// Decodifica una secuencia de token IDs a la cadena de texto UTF-8 original.
    pub fn decode(&self, ids: &[u32]) -> Result<String, String> {
        let mut buffer = String::new();
        for &id in ids {
            if let Some(token_str) = self.vocab.id_to_token.get(&id) {
                // Ignorar tokens especiales en decodificacion estandar
                if token_str.starts_with("<|") && token_str.ends_with("|>") {
                    continue;
                }
                buffer.push_str(token_str);
            }
        }
        bytelevel_to_text(&buffer, &self.char_to_byte)
            .map_err(|e| format!("Error decodificando UTF-8: {e}"))
    }

    pub fn vocab(&self) -> &Vocab {
        &self.vocab
    }
}
