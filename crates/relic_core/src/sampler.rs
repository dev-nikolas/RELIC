//! Muestreador determinista y probabilistico de tokens.
pub struct Sampler;

impl Sampler {
    /// Selecciona de forma determinista el token con el mayor logit (Greedy Search).
    pub fn greedy(logits: &[f32]) -> u32 {
        let mut best_id = 0u32;
        let mut best_logit = f32::NEG_INFINITY;
        for (id, &logit) in logits.iter().enumerate() {
            if logit > best_logit {
                best_logit = logit;
                best_id = id as u32;
            }
        }
        best_id
    }

    /// Muestra un token aplicando temperatura y muestreo probabilistico.
    pub fn sample_with_temperature(logits: &[f32], temperature: f32, mut rng_state: u64) -> (u32, u64) {
        if temperature <= 1e-4 {
            return (Self::greedy(logits), rng_state);
        }

        let mut scaled_logits: Vec<f32> = logits.iter().map(|&x| x / temperature).collect();
        crate::ops::softmax(&mut scaled_logits);

        // Generador de numeros pseudo-aleatorio lineal rapido (LCG)
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let random_fraction = (rng_state >> 32) as f32 / (u32::MAX as f32);

        let mut cumulative = 0.0f32;
        let mut selected = 0u32;
        for (i, &p) in scaled_logits.iter().enumerate() {
            cumulative += p;
            if random_fraction <= cumulative {
                selected = i as u32;
                break;
            }
        }

        (selected, rng_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greedy_selection() {
        let logits = [0.1, 0.5, 2.8, -1.0];
        let token = Sampler::greedy(&logits);
        assert_eq!(token, 2);
    }
}
