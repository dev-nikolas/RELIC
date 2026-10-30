//! Rotary Position Embeddings (RoPE) precomputado en Rust puro.
#[derive(Debug, Clone)]
pub struct RotaryEmbedding {
    pub head_dim: usize,
    pub max_seq_len: usize,
    pub cos_table: Vec<f32>, // Shape: [max_seq_len, head_dim]
    pub sin_table: Vec<f32>, // Shape: [max_seq_len, head_dim]
}

impl RotaryEmbedding {
    pub fn new(head_dim: usize, max_seq_len: usize, theta: f32) -> Self {
        assert!(head_dim.is_multiple_of(2), "head_dim debe ser par");
        let half = head_dim / 2;

        let mut inv_freq = Vec::with_capacity(half);
        for i in 0..half {
            let exponent = (2 * i) as f32 / (head_dim as f32);
            inv_freq.push(1.0 / theta.powf(exponent));
        }

        let mut cos_table = Vec::with_capacity(max_seq_len * head_dim);
        let mut sin_table = Vec::with_capacity(max_seq_len * head_dim);

        for pos in 0..max_seq_len {
            // Primera mitad [0..half]
            for &freq in &inv_freq {
                let angle = (pos as f32) * freq;
                cos_table.push(angle.cos());
                sin_table.push(angle.sin());
            }
            // Segunda mitad [half..head_dim] duplica las frecuencias
            for &freq in &inv_freq {
                let angle = (pos as f32) * freq;
                cos_table.push(angle.cos());
                sin_table.push(angle.sin());
            }
        }

        Self {
            head_dim,
            max_seq_len,
            cos_table,
            sin_table,
        }
    }

    /// Aplica RoPE in-place sobre un vector de dimension head_dim correspondiente a la posicion pos.
    pub fn apply(&self, x: &mut [f32], pos: usize) {
        debug_assert_eq!(x.len(), self.head_dim);
        debug_assert!(pos < self.max_seq_len);

        let half = self.head_dim / 2;
        let offset = pos * self.head_dim;
        let cos = &self.cos_table[offset..offset + self.head_dim];
        let sin = &self.sin_table[offset..offset + self.head_dim];

        let mut rot = vec![0.0f32; self.head_dim];
        for i in 0..half {
            rot[i] = -x[i + half];
            rot[i + half] = x[i];
        }

        for i in 0..self.head_dim {
            x[i] = x[i] * cos[i] + rot[i] * sin[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rope_orthogonality() {
        let rope = RotaryEmbedding::new(4, 32, 10000.0);
        let mut vec_a = vec![1.0, 2.0, 3.0, 4.0];
        let norm_before: f32 = vec_a.iter().map(|v| v * v).sum();
        rope.apply(&mut vec_a, 5);
        let norm_after: f32 = vec_a.iter().map(|v| v * v).sum();
        // La rotacion debe preservar exactamente la norma Euclidea (isometria)
        assert!((norm_before - norm_after).abs() < 1e-4);
    }
}
