//! Ventana ECDF Causal Exacta y Compacta para Seis Ojos en Rust.
//!
//! Reemplaza la doble asignación de historial (256 KB) por un buffer circular
//! de valores `f64` (128 KB) y un vector de punteros ordenados `u16` (32 KB).
//! Total: 160 KB exactos (-37.5% de RAM), reduciendo 4x el movimiento de memoria.

pub const ECDF_WINDOW: usize = 16000;
pub const MIN_HISTORIAL: usize = 288;

const _: () = assert!(ECDF_WINDOW <= u16::MAX as usize);

#[derive(Clone)]
pub struct EcdfWindow {
    ring: Box<[f64]>,
    sorted: Vec<u16>,
    head: usize,
    capacity: usize,
    min_historial: usize,
}

impl EcdfWindow {
    pub fn new() -> Self {
        Self::with_capacity(ECDF_WINDOW, MIN_HISTORIAL)
    }

    pub fn with_capacity(capacity: usize, min_historial: usize) -> Self {
        assert!(capacity <= u16::MAX as usize, "Capacidad excede u16::MAX");
        Self {
            ring: vec![0.0; capacity].into_boxed_slice(),
            sorted: Vec::with_capacity(capacity),
            head: 0,
            capacity,
            min_historial,
        }
    }

    pub fn reset(&mut self) {
        self.sorted.clear();
        self.head = 0;
    }

    #[inline]
    fn lower(&self, v: f64) -> usize {
        self.sorted.partition_point(|&i| self.ring[i as usize] < v)
    }

    #[inline]
    fn upper(&self, v: f64) -> usize {
        self.sorted.partition_point(|&i| self.ring[i as usize] <= v)
    }

    /// Rango empírico: retorna `(menores, iguales, n)` equivalente a `bisect_left` y `bisect_right`.
    #[inline]
    pub fn rank(&self, v: f64) -> (usize, usize, usize) {
        let lo = self.lower(v);
        (lo, self.upper(v) - lo, self.sorted.len())
    }

    /// Transforma el error residual en un percentil empírico calibrado en `[0.0, 1.0]`.
    #[inline]
    pub fn score(&self, v: f64) -> f64 {
        if !v.is_finite() || v == 0.0 || self.sorted.len() < self.min_historial {
            return 0.0;
        }
        let (menores, iguales, n) = self.rank(v);
        ((menores as f64) + 0.5 * (iguales as f64) + 0.5) / (n as f64 + 1.0)
    }

    /// Ingesta causal de la observación residual tras la evaluación pura.
    pub fn push(&mut self, v: f64) {
        if !v.is_finite() || v == 0.0 {
            return;
        }

        // Expulsar el elemento más antiguo si se alcanzó la capacidad máxima
        if self.sorted.len() == self.capacity {
            let old_val = self.ring[self.head];
            let mut pos = self.lower(old_val);
            while pos < self.sorted.len() && self.sorted[pos] as usize != self.head {
                pos += 1;
            }
            if pos < self.sorted.len() {
                self.sorted.remove(pos);
            }
        }

        // Insertar en el anillo y en la lista ordenada
        self.ring[self.head] = v;
        let pos = self.upper(v);
        self.sorted.insert(pos, self.head as u16);
        self.head = (self.head + 1) % self.capacity;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.sorted.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sorted.is_empty()
    }

    /// Valida que el orden en sorted apunte estrictamente a elementos no decrecientes en el anillo.
    pub fn is_sorted(&self) -> bool {
        self.sorted.windows(2).all(|w| self.ring[w[0] as usize] <= self.ring[w[1] as usize])
    }
}

