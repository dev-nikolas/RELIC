//! KV-Cache dinamico preasignado para inferencia autoregresiva eficiente.
//! Memoria acotada (< 2 MB) para contexto de hasta 256 tokens. Menos de 180 lineas.

#[derive(Debug, Clone)]
pub struct LayerKvCache {
    pub k_cache: Vec<f32>, // [max_seq_len, num_kv_heads * head_dim]
    pub v_cache: Vec<f32>, // [max_seq_len, num_kv_heads * head_dim]
    pub kv_dim: usize,
}

impl LayerKvCache {
    pub fn new(max_seq_len: usize, kv_dim: usize) -> Self {
        Self {
            k_cache: vec![0.0f32; max_seq_len * kv_dim],
            v_cache: vec![0.0f32; max_seq_len * kv_dim],
            kv_dim,
        }
    }

    /// Guarda el nuevo vector K y V en la posicion temporal `pos`.
    pub fn store(&mut self, pos: usize, k: &[f32], v: &[f32]) {
        debug_assert_eq!(k.len(), self.kv_dim);
        debug_assert_eq!(v.len(), self.kv_dim);
        let start = pos * self.kv_dim;
        let end = start + self.kv_dim;
        self.k_cache[start..end].copy_from_slice(k);
        self.v_cache[start..end].copy_from_slice(v);
    }
}

#[derive(Debug, Clone)]
pub struct KvCache {
    pub layers: Vec<LayerKvCache>,
    pub max_seq_len: usize,
    pub current_len: usize,
}

impl KvCache {
    pub fn new(num_layers: usize, max_seq_len: usize, num_kv_heads: usize, head_dim: usize) -> Self {
        let kv_dim = num_kv_heads * head_dim;
        let layers = (0..num_layers)
            .map(|_| LayerKvCache::new(max_seq_len, kv_dim))
            .collect();

        Self {
            layers,
            max_seq_len,
            current_len: 0,
        }
    }

    /// Reinicia la longitud activa del cache para un nuevo prompt.
    pub fn reset(&mut self) {
        self.current_len = 0;
    }

    /// Incrementa la longitud del contexto almacenado en 1 token.
    pub fn advance(&mut self) {
        self.current_len = (self.current_len + 1).min(self.max_seq_len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_cache_storage_and_advance() {
        let mut cache = KvCache::new(2, 16, 2, 4);
        assert_eq!(cache.current_len, 0);

        let dummy_k = vec![1.0; 8];
        let dummy_v = vec![2.0; 8];
        cache.layers[0].store(0, &dummy_k, &dummy_v);
        cache.advance();

        assert_eq!(cache.current_len, 1);
        assert_eq!(&cache.layers[0].k_cache[0..8], &dummy_k[..]);
    }
}
