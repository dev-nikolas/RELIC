//! # NodeSpatialNeighborhood & TopologicalResidualEngine
//!
//! Representación de memoria contigua para vecindarios espaciales y motores de residuos.
//! Layout alineado a 128 bytes (2 líneas de caché L1 contiguas) con cotas convexas
//! `[weight_min, weight_max]` para prevenir el ataque de deriva lenta (*Boiling Frog*).

/// Capacidad máxima de vecinos directos 1-hop por nodo.
pub const MAX_SPATIAL_NEIGHBORS: usize = 8;
/// Capacidad máxima de vecinos calibrables con cotas politópicas.
pub const MAX_BOUNDED_NEIGHBORS: usize = 4;
/// Ratio de tolerancia física por defecto (±25% sobre impedancia nominal).
pub const DEFAULT_TOLERANCE_RATIO: f32 = 0.25;

/// Vecindad causal alineada a 2 líneas de caché L1 (128 bytes).
/// Incluye cotas duras convexas `[weight_min, weight_max]` para evitar deriva adversarial.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct NodeSpatialNeighborhood {
    pub weights: [f32; MAX_SPATIAL_NEIGHBORS],    // 32 bytes (offset 0)
    pub indices: [u16; MAX_SPATIAL_NEIGHBORS],    // 16 bytes (offset 32)
    pub count: u8,                                // 1 byte   (offset 48)
    pub _pad_header: [u8; 15],                    // 15 bytes (offset 49) -> Total 64B
    pub weight_min: [f32; MAX_BOUNDED_NEIGHBORS], // 16 bytes (offset 64)
    pub weight_max: [f32; MAX_BOUNDED_NEIGHBORS], // 16 bytes (offset 80)
    pub _pad_tail: [u8; 32],                      // 32 bytes (offset 96) -> Total 128B
}

const _: () = assert!(std::mem::size_of::<NodeSpatialNeighborhood>() == 128);
const _: () = assert!(std::mem::align_of::<NodeSpatialNeighborhood>() == 64);

impl NodeSpatialNeighborhood {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            weights: [0.0f32; MAX_SPATIAL_NEIGHBORS], indices: [0u16; MAX_SPATIAL_NEIGHBORS],
            count: 0, _pad_header: [0u8; 15],
            weight_min: [0.0f32; MAX_BOUNDED_NEIGHBORS], weight_max: [1.0f32; MAX_BOUNDED_NEIGHBORS],
            _pad_tail: [0u8; 32],
        }
    }

    pub fn new(neighbors: &[(u16, f32)]) -> Result<Self, &'static str> {
        Self::with_tolerance(neighbors, DEFAULT_TOLERANCE_RATIO)
    }

    pub fn with_tolerance(neighbors: &[(u16, f32)], tolerance_ratio: f32) -> Result<Self, &'static str> {
        if neighbors.len() > MAX_SPATIAL_NEIGHBORS {
            return Err("Supera MAX_SPATIAL_NEIGHBORS (8)");
        }
        let mut node = Self::empty();
        let mut sum = 0.0f32;
        let tol = tolerance_ratio.clamp(0.0, 1.0);

        for (i, &(idx, w)) in neighbors.iter().enumerate() {
            if !w.is_finite() || w < 0.0 {
                return Err("Los pesos deben ser finitos y no negativos");
            }
            node.indices[i] = idx;
            node.weights[i] = w;
            sum += w;
        }

        node.count = neighbors.len() as u8;
        if node.count > 0 {
            if sum <= 1e-7 {
                return Err("La suma de pesos debe ser positiva");
            }
            let inv_sum = 1.0f32 / sum;
            for i in 0..node.count as usize {
                node.weights[i] *= inv_sum;
                if i < MAX_BOUNDED_NEIGHBORS {
                    let nominal = node.weights[i];
                    node.weight_min[i] = (nominal * (1.0 - tol)).max(0.0);
                    node.weight_max[i] = (nominal * (1.0 + tol)).min(1.0);
                }
            }
        }
        Ok(node)
    }

    pub fn add_neighbor(&mut self, node_id: u16, weight: f32) -> Result<(), &'static str> {
        if (self.count as usize) >= MAX_SPATIAL_NEIGHBORS {
            return Err("Vecindad saturada: máximo 8 vecinos");
        }
        if !weight.is_finite() || weight < 0.0 {
            return Err("El peso espacial debe ser finito y no negativo");
        }
        let idx = self.count as usize;
        self.indices[idx] = node_id;
        self.weights[idx] = weight;
        self.count += 1;
        Ok(())
    }

    pub fn normalize(&mut self) {
        if self.count == 0 { return; }
        let c = (self.count as usize).min(MAX_SPATIAL_NEIGHBORS);
        let sum: f32 = self.weights[..c].iter().sum();
        if sum > 1e-7 {
            let inv_sum = 1.0 / sum;
            for i in 0..c {
                self.weights[i] *= inv_sum;
                if i < MAX_BOUNDED_NEIGHBORS {
                    let nominal = self.weights[i];
                    self.weight_min[i] = (nominal * (1.0 - DEFAULT_TOLERANCE_RATIO)).max(0.0);
                    self.weight_max[i] = (nominal * (1.0 + DEFAULT_TOLERANCE_RATIO)).min(1.0);
                }
            }
        }
    }

    #[inline(always)]
    pub fn compute_residual(&self, target_val: f32, sensor_snapshot: &[f32]) -> f32 {
        let n = (self.count as usize).min(MAX_SPATIAL_NEIGHBORS);
        let len = sensor_snapshot.len();
        let mut predicted = 0.0f32;

        for k in 0..MAX_SPATIAL_NEIGHBORS {
            if k < n {
                let idx = self.indices[k] as usize;
                if idx < len {
                    predicted = self.weights[k].mul_add(sensor_snapshot[idx], predicted);
                }
            }
        }
        target_val - predicted
    }
}

impl Default for NodeSpatialNeighborhood {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

/// Contenedor estático Zero-Alloc para la topología de la red de sensores.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct TopologicalResidualEngine<const MAX_NODES: usize> {
    pub neighborhoods: [NodeSpatialNeighborhood; MAX_NODES],
}

impl<const MAX_NODES: usize> TopologicalResidualEngine<MAX_NODES> {
    pub const fn new() -> Self {
        Self {
            neighborhoods: [NodeSpatialNeighborhood::empty(); MAX_NODES],
        }
    }

    pub fn set_neighborhood(&mut self, node_id: usize, neighborhood: NodeSpatialNeighborhood) -> Result<(), &'static str> {
        if node_id >= MAX_NODES {
            return Err("node_id fuera de los límites de MAX_NODES");
        }
        self.neighborhoods[node_id] = neighborhood;
        Ok(())
    }

    #[inline(always)]
    pub fn compute_node_residual(&self, node_id: usize, snapshot: &[f32; MAX_NODES]) -> f32 {
        if node_id >= MAX_NODES { return 0.0f32; }
        self.neighborhoods[node_id].compute_residual(snapshot[node_id], snapshot)
    }

    #[inline(always)]
    pub fn compute_all_residuals(&self, snapshot: &[f32; MAX_NODES], out_residuals: &mut [f32; MAX_NODES]) {
        for i in 0..MAX_NODES {
            out_residuals[i] = self.neighborhoods[i].compute_residual(snapshot[i], snapshot);
        }
    }
}

impl<const MAX_NODES: usize> Default for TopologicalResidualEngine<MAX_NODES> {
    #[inline]
    fn default() -> Self { Self::new() }
}
