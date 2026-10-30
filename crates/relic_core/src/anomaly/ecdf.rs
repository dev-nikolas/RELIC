//! Calibrador de Percentiles Empíricos (ECDF - Empirical Cumulative Distribution Function).
//!
//! ¿Para qué sirve?
//! Convierte el error de predicción en un porcentaje comprensible de 0.0 a 1.0.
//! Por ejemplo, si el error actual supera al 99% de todos los errores pasados,
//! el resultado es 0.99 (alta sospecha de anomalía).
//!
//! ¿Cómo funciona por dentro?
//! - Mantiene una ventana circular con los últimos 16,000 errores de predicción.
//! - Utiliza un vector de índices ordenados para buscar la posición del nuevo valor en tiempo logarítmico (búsqueda binaria).
//! - Cuando la ventana se llena (16,000 elementos), el dato más viejo se expulsa de manera limpia.

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

impl Default for EcdfWindow {
    fn default() -> Self {
        Self::new()
    }
}

