//! # NodeSpatialNeighborhood & TopologicalResidualEngine
//!
//! Representación SoA y cálculo diferencial de residuos espaciales.
//! Layout alineado a 64 bytes para garantizar residencia en 1 línea de caché L1.

/// Capacidad máxima de vecinos directos 1-hop por nodo.
pub const MAX_SPATIAL_NEIGHBORS: usize = 8;

/// Representación SoA de la vecindad causal de un nodo alineada a 64 bytes.
///
/// Memoria:
/// - weights: [f32; 8]  => 32 bytes (offset 0)
/// - indices: [u16; 8]  => 16 bytes (offset 32)
/// - count:   u8        => 1 byte   (offset 48)
/// - _pad:    [u8; 15]  => 15 bytes (offset 49)
/// Total = 64 bytes, alineado a 64 bytes. Coincide exactamente con 1 línea de caché L1.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct NodeSpatialNeighborhood {
    pub weights: [f32; MAX_SPATIAL_NEIGHBORS],
    pub indices: [u16; MAX_SPATIAL_NEIGHBORS],
    pub count: u8,
    pub _padding: [u8; 15],
}

const _: () = assert!(std::mem::size_of::<NodeSpatialNeighborhood>() == 64);
const _: () = assert!(std::mem::align_of::<NodeSpatialNeighborhood>() == 64);

impl NodeSpatialNeighborhood {
    /// Construye una vecindad vacía (nodo aislado / sin acoplamiento espacial).
    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            weights: [0.0f32; MAX_SPATIAL_NEIGHBORS],
            indices: [0u16; MAX_SPATIAL_NEIGHBORS],
            count: 0,
            _padding: [0u8; 15],
        }
    }

    /// Construye una vecindad normalizada a partir de una lista de pares (nodo_vecino, peso).
    pub fn new(neighbors: &[(u16, f32)]) -> Result<Self, &'static str> {
        if neighbors.len() > MAX_SPATIAL_NEIGHBORS {
            return Err("El número de vecinos espaciales excede MAX_SPATIAL_NEIGHBORS (8)");
        }

        let mut node = Self::empty();
        let mut sum_weights = 0.0f32;

        for (i, &(idx, weight)) in neighbors.iter().enumerate() {
            if !weight.is_finite() || weight < 0.0 {
                return Err("Los pesos espaciales deben ser números finitos no negativos");
            }
            node.indices[i] = idx;
            node.weights[i] = weight;
            sum_weights += weight;
        }

        node.count = neighbors.len() as u8;

        if node.count > 0 {
            if sum_weights <= 1e-7 {
                return Err("La suma de pesos espaciales debe ser estrictamente positiva");
            }
            let inv_sum = 1.0f32 / sum_weights;
            for i in 0..node.count as usize {
                node.weights[i] *= inv_sum;
            }
        }

        Ok(node)
    }

    /// Registra un vecino espacial individual. Si se supera el límite de 8, retorna error.
    pub fn add_neighbor(&mut self, node_id: u16, weight: f32) -> Result<(), &'static str> {
        if (self.count as usize) >= MAX_SPATIAL_NEIGHBORS {
            return Err("Vecindad saturada: no se pueden agregar más de 8 vecinos");
        }
        if !weight.is_finite() || weight < 0.0 {
            return Err("El peso espacial debe ser un valor finito no negativo");
        }

        let idx = self.count as usize;
        self.indices[idx] = node_id;
        self.weights[idx] = weight;
        self.count += 1;
        Ok(())
    }

    /// Normaliza los pesos de los vecinos registrados para que sumen exactamente 1.0.
    pub fn normalize(&mut self) {
        if self.count == 0 {
            return;
        }
        let mut sum = 0.0f32;
        let c = (self.count as usize).min(MAX_SPATIAL_NEIGHBORS);
        for i in 0..c {
            sum += self.weights[i];
        }
        if sum > 1e-7 {
            let inv_sum = 1.0 / sum;
            for i in 0..c {
                self.weights[i] *= inv_sum;
            }
        }
    }

    /// Calcula el residuo diferencial topológico \varepsilon_i(t) en tiempo constante.
    ///
    /// Utiliza FMA (`f32::mul_add`) para calcular `weights[k] * snapshot[idx] + acc` en 1 ciclo.
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

    pub fn set_neighborhood(
        &mut self,
        node_id: usize,
        neighborhood: NodeSpatialNeighborhood,
    ) -> Result<(), &'static str> {
        if node_id >= MAX_NODES {
            return Err("node_id fuera de los límites de MAX_NODES");
        }
        self.neighborhoods[node_id] = neighborhood;
        Ok(())
    }

    #[inline(always)]
    pub fn compute_node_residual(&self, node_id: usize, snapshot: &[f32; MAX_NODES]) -> f32 {
        if node_id >= MAX_NODES {
            return 0.0f32;
        }
        let target = snapshot[node_id];
        self.neighborhoods[node_id].compute_residual(target, snapshot)
    }

    #[inline(always)]
    pub fn compute_all_residuals(
        &self,
        snapshot: &[f32; MAX_NODES],
        out_residuals: &mut [f32; MAX_NODES],
    ) {
        for i in 0..MAX_NODES {
            let target = snapshot[i];
            out_residuals[i] = self.neighborhoods[i].compute_residual(target, snapshot);
        }
    }
}

impl<const MAX_NODES: usize> Default for TopologicalResidualEngine<MAX_NODES> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

