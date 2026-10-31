//! # Bridge module
//! Coordenada jerárquica HierarchicalNode (2 B), descriptor InterClusterBridge (12 B)
//! y registro de puentes activos BridgeRegistry con refcount por par de clústeres.
//! Zero heap allocations.

use super::graph::BitAdjacencyGraph;

pub const MAX_CLUSTERS: usize = 64;
pub const MAX_BRIDGES:  usize = 128;

// ── Errores ──────────────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BridgeError {
    InvalidCluster(u8),
    InvalidLocal(u8),
    SameNode,
    ClusterOutOfBounds,
    LocalNodeOutOfBounds,
    BridgeTableFull,
    IntraClusterEdgeNotAllowed,
}

// ── HierarchicalNode — 2 B ───────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HierarchicalNode {
    pub cluster: u8,
    pub local:   u8,
}
const _: () = assert!(core::mem::size_of::<HierarchicalNode>() == 2);

impl HierarchicalNode {
    /// Constructor sin módulo silencioso. Rechaza cluster >= 64 o local >= 64.
    pub const fn new(cluster: u8, local: u8) -> Result<Self, BridgeError> {
        if cluster as usize >= MAX_CLUSTERS { return Err(BridgeError::InvalidCluster(cluster)); }
        if local   as usize >= MAX_CLUSTERS { return Err(BridgeError::InvalidLocal(local)); }
        Ok(Self { cluster, local })
    }

    /// Convierte id global sin módulo silencioso. Rechaza id >= 64*64.
    pub fn from_global(id: u16) -> Result<Self, BridgeError> {
        let cluster = (id / 64) as u8;
        let local   = (id % 64) as u8;
        if cluster as usize >= MAX_CLUSTERS { return Err(BridgeError::InvalidCluster(cluster)); }
        Ok(Self { cluster, local })
    }
}

// ── InterClusterBridge — 12 B ────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InterClusterBridge {
    pub edge_id: u32,
    pub src:     HierarchicalNode,
    pub dst:     HierarchicalNode,
    pub active:  bool,
    pub _pad:    [u8; 3],
}
const _: () = assert!(core::mem::size_of::<InterClusterBridge>() == 12);

// ── BridgeRegistry ───────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Debug)]
pub struct BridgeRegistry {
    pub bridges: [InterClusterBridge; MAX_BRIDGES],
    pub count:   u16,
    counts:      [[u16; MAX_CLUSTERS]; MAX_CLUSTERS],
}

impl BridgeRegistry {
    pub const fn new() -> Self {
        const Z: InterClusterBridge = InterClusterBridge {
            edge_id: 0,
            src: HierarchicalNode { cluster: 0, local: 0 },
            dst: HierarchicalNode { cluster: 0, local: 0 },
            active: false,
            _pad: [0; 3],
        };
        Self { bridges: [Z; MAX_BRIDGES], count: 0, counts: [[0u16; MAX_CLUSTERS]; MAX_CLUSTERS] }
    }

    /// Añade un puente inter-clúster activo. Rechaza intra-clúster (prohibición de aristas intra-isla en el registro de puentes).
    pub fn add_bridge(
        &mut self, backbone: &mut BitAdjacencyGraph,
        src: HierarchicalNode, dst: HierarchicalNode,
    ) -> Result<(), BridgeError> {
        if src.cluster == dst.cluster { return Err(BridgeError::IntraClusterEdgeNotAllowed); }
        let idx = self.count as usize;
        if idx >= MAX_BRIDGES { return Err(BridgeError::BridgeTableFull); }
        let cu = src.cluster as usize;
        let cv = dst.cluster as usize;
        self.counts[cu][cv] = self.counts[cu][cv].saturating_add(1);
        self.bridges[idx] = InterClusterBridge { edge_id: idx as u32, src, dst, active: true, _pad: [0;3] };
        self.count += 1;
        backbone.restore_edge(cu as u8, cv as u8);
        Ok(())
    }

    /// Pone en cuarentena (idempotente si ya count==0).
    pub fn quarantine_bridge(
        &mut self, backbone: &mut BitAdjacencyGraph,
        src: HierarchicalNode, dst: HierarchicalNode,
    ) -> Result<(), BridgeError> {
        if src.cluster == dst.cluster { return Err(BridgeError::IntraClusterEdgeNotAllowed); }
        let cu = src.cluster as usize;
        let cv = dst.cluster as usize;
        if self.counts[cu][cv] == 0 { return Ok(()); }
        self.counts[cu][cv] -= 1;
        // Desactivar descriptor coincidente exacto en src y dst
        for i in 0..self.count as usize {
            let b = &mut self.bridges[i];
            if b.active && b.src == src && b.dst == dst {
                b.active = false;
                break;
            }
        }
        if self.counts[cu][cv] == 0 { backbone.quarantine_edge(cu as u8, cv as u8); }
        Ok(())
    }

    /// Restaura un puente inactivo (idempotente si todos los descriptores del par ya están activos).
    pub fn restore_bridge(
        &mut self, backbone: &mut BitAdjacencyGraph,
        src: HierarchicalNode, dst: HierarchicalNode,
    ) -> Result<(), BridgeError> {
        if src.cluster == dst.cluster { return Err(BridgeError::IntraClusterEdgeNotAllowed); }
        let cu = src.cluster as usize;
        let cv = dst.cluster as usize;
        let mut found = false;
        for i in 0..self.count as usize {
            let b = &mut self.bridges[i];
            if !b.active && b.src == src && b.dst == dst {
                b.active = true;
                found = true;
                break;
            }
        }
        if found {
            self.counts[cu][cv] = self.counts[cu][cv].saturating_add(1);
            backbone.restore_edge(cu as u8, cv as u8);
        }
        Ok(())
    }

    #[inline] pub fn bridge_count(&self, src: HierarchicalNode, dst: HierarchicalNode) -> u16 {
        self.counts[src.cluster as usize][dst.cluster as usize]
    }

    #[inline] pub fn backbone_connected(
        &self, backbone: &BitAdjacencyGraph,
        src: HierarchicalNode, dst: HierarchicalNode,
    ) -> bool { backbone.is_connected(src.cluster, dst.cluster) }
}

impl Default for BridgeRegistry { fn default() -> Self { Self::new() } }
