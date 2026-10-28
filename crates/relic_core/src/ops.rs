//! Operadores tensoriales fundamentales y funciones de activacion optimizadas.
//! Menos de 180 lineas.

/// Suma vectorial elemento a elemento: out[i] = a[i] + b[i].
pub fn vec_add(out: &mut [f32], a: &[f32], b: &[f32]) {
    for (o, (&x, &y)) in out.iter_mut().zip(a.iter().zip(b.iter())) {
        *o = x + y;
    }
}

/// Suma acumulativa sobre el primer operando: a[i] += b[i].
pub fn vec_add_assign(a: &mut [f32], b: &[f32]) {
    for (x, &y) in a.iter_mut().zip(b.iter()) {
        *x += y;
    }
}

/// Producto interno optimizado con desenrollado de 4 vias para instruccion FMA/AVX2.
pub fn vec_dot(a: &[f32], b: &[f32]) -> f32 {
    let mut sum0 = 0.0f32;
    let mut sum1 = 0.0f32;
    let mut sum2 = 0.0f32;
    let mut sum3 = 0.0f32;

    let (chunks_a, rem_a) = a.as_chunks::<4>();
    let (chunks_b, rem_b) = b.as_chunks::<4>();

    for (ca, cb) in chunks_a.iter().zip(chunks_b.iter()) {
        sum0 += ca[0] * cb[0];
        sum1 += ca[1] * cb[1];
        sum2 += ca[2] * cb[2];
        sum3 += ca[3] * cb[3];
    }

    let mut total = (sum0 + sum1) + (sum2 + sum3);
    for (&x, &y) in rem_a.iter().zip(rem_b.iter()) {
        total += x * y;
    }
    total
}

/// Proyeccion lineal estandar estilo PyTorch: y = x @ W^T.
/// x: [in_dim], weight: [out_dim, in_dim], out: [out_dim].
pub fn linear_forward(out: &mut [f32], x: &[f32], weight: &[f32], in_dim: usize, out_dim: usize) {
    debug_assert_eq!(x.len(), in_dim);
    debug_assert_eq!(out.len(), out_dim);
    debug_assert_eq!(weight.len(), out_dim * in_dim);

    for (o, item) in out.iter_mut().enumerate().take(out_dim) {
        let row_start = o * in_dim;
        let row = &weight[row_start..row_start + in_dim];
        *item = vec_dot(x, row);
    }
}

/// Activacion SiLU (Swish-1): f(x) = x / (1.0 + exp(-x)).
#[inline(always)]
pub fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

/// Fusion elemento a elemento para SwiGLU: out[i] = silu(gate[i]) * up[i].
pub fn swiglu_forward(out: &mut [f32], gate: &[f32], up: &[f32]) {
    for (o, (&g, &u)) in out.iter_mut().zip(gate.iter().zip(up.iter())) {
        *o = silu(g) * u;
    }
}

/// Softmax numericamente estable sobre un slice continuo.
pub fn softmax(slice: &mut [f32]) {
    if slice.is_empty() {
        return;
    }
    let mut max_val = f32::NEG_INFINITY;
    for &val in slice.iter() {
        if val > max_val {
            max_val = val;
        }
    }

    let mut sum = 0.0f32;
    for val in slice.iter_mut() {
        *val = (*val - max_val).exp();
        sum += *val;
    }

    let inv_sum = 1.0 / sum.max(1e-12);
    for val in slice.iter_mut() {
        *val *= inv_sum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silu_and_softmax() {
        assert!((silu(0.0) - 0.0).abs() < 1e-6);
        let mut probs = vec![1.0, 2.0, 3.0];
        softmax(&mut probs);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!(probs[0] < probs[1] && probs[1] < probs[2]);
    }

    #[test]
    fn test_linear_forward() {
        let x = vec![1.0, 2.0];
        let w = vec![1.0, 0.0, 0.0, 1.0]; // Matriz identidad 2x2
        let mut out = vec![0.0, 0.0];
        linear_forward(&mut out, &x, &w, 2, 2);
        assert_eq!(out, vec![1.0, 2.0]);
    }
}
