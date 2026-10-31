//! Grafo jerárquico de 2 niveles (Backbone Macro + Islas Micro de 64 nodos).
//!
//! Resuelve alcanzabilidad exacta multi-clúster en memoria estática (Zero-Alloc)
//! verificando conectividad interna entre nodos locales y puertos de puentes activos.

use super::bridge::{BridgeError, BridgeRegistry, HierarchicalNode};
use super::graph::{BitAdjacencyGraph, MAX_GRAPH_NODES};
use super::reachability::CausalReachabilityFilter;

/// Grafo jerárquico parametrizado por el número de islas `K` (1 ≤ K ≤ 64).
#[repr(C)]
#[derive(Clone, Debug)]
pub struct HierarchicalBitGraph<const K: usize> {
    pub backbone: BitAdjacencyGraph,
    pub islands:  [BitAdjacencyGraph; K],
    pub bridges:  BridgeRegistry,
}

impl<const K: usize> HierarchicalBitGraph<K> {
    pub const fn new() -> Self {
        assert!(K >= 1 && K <= MAX_GRAPH_NODES);
        Self {
            backbone: BitAdjacencyGraph::new(),
            islands:  [BitAdjacencyGraph::new(); K],
            bridges:  BridgeRegistry::new(),
        }
    }

    #[inline(always)]
    fn validate(node: HierarchicalNode) -> Result<(), BridgeError> {
        if (node.cluster as usize) >= K      { return Err(BridgeError::ClusterOutOfBounds); }
        if (node.cluster as usize) >= MAX_GRAPH_NODES { return Err(BridgeError::ClusterOutOfBounds); }
        if (node.local   as usize) >= MAX_GRAPH_NODES { return Err(BridgeError::LocalNodeOutOfBounds); }
        Ok(())
    }

    /// Arista bidireccional dentro de una isla.
    pub fn add_intra_cluster_edge(&mut self, cluster: u8, u: u8, v: u8) -> Result<(), BridgeError> {
        let un = HierarchicalNode::new(cluster, u)?;
        let vn = HierarchicalNode::new(cluster, v)?;
        Self::validate(un)?;
        Self::validate(vn)?;
        self.islands[cluster as usize].add_bidirectional_edge(u, v);
        Ok(())
    }

    /// Registra un puente inter-clúster (actualiza refcount y backbone).
    pub fn add_inter_cluster_bridge(&mut self, _edge_id: u32, src: HierarchicalNode, dst: HierarchicalNode) -> Result<(), BridgeError> {
        Self::validate(src)?;
        Self::validate(dst)?;
        self.bridges.add_bridge(&mut self.backbone, src, dst)
    }

    /// Pone en cuarentena un puente (por par src/dst).
    pub fn quarantine_bridge(&mut self, src: HierarchicalNode, dst: HierarchicalNode) -> Result<(), BridgeError> {
        self.bridges.quarantine_bridge(&mut self.backbone, src, dst)
    }

    /// Restaura un puente inactivo (por par src/dst).
    pub fn restore_bridge(&mut self, src: HierarchicalNode, dst: HierarchicalNode) -> Result<(), BridgeError> {
        self.bridges.restore_bridge(&mut self.backbone, src, dst)
    }

    /// Cierre transitivo dentro de una sola isla desde `seed_mask`. Zero-alloc.
    #[inline(always)]
    pub fn island_closure(island: &BitAdjacencyGraph, seed_mask: u64) -> u64 {
        let mut visited  = seed_mask;
        let mut frontier = seed_mask;
        while frontier != 0 {
            let mut next = 0u64;
            let mut f = frontier;
            while f != 0 {
                let u = f.trailing_zeros() as usize;
                next |= island.adj[u];
                f &= f - 1;
            }
            frontier = next & !visited;
            visited |= frontier;
        }
        visited
    }

    /// Alcanzabilidad exacta entre dos nodos jerárquicos. Zero-alloc.
    pub fn is_reachable(&self, src: HierarchicalNode, dst: HierarchicalNode) -> bool {
        if Self::validate(src).is_err() || Self::validate(dst).is_err() { return false; }
        if src == dst { return true; }

        // Poda macro: si islas distintas, el backbone debe conectarlas.
        if src.cluster != dst.cluster {
            let macro_reach = Self::island_closure(&self.backbone, 1u64 << src.cluster);
            if (macro_reach & (1u64 << dst.cluster)) == 0 { return false; }
        }

        // Propagación de punto fijo sobre máscaras locales por isla.
        let mut reached = [0u64; K];
        let si = src.cluster as usize;
        reached[si] = Self::island_closure(&self.islands[si], 1u64 << src.local);

        let total = self.bridges.count as usize;
        loop {
            let mut changed = false;
            for i in 0..total {
                let b = &self.bridges.bridges[i];
                if !b.active { continue; }
                let cu = b.src.cluster as usize;
                let cv = b.dst.cluster as usize;
                if cu >= K || cv >= K { continue; }

                // u → v
                if (reached[cu] & (1u64 << b.src.local)) != 0 {
                    let seed = 1u64 << b.dst.local;
                    if (reached[cv] & seed) == 0 {
                        reached[cv] |= Self::island_closure(&self.islands[cv], seed);
                        changed = true;
                    }
                }
                // v → u (bidireccional)
                if (reached[cv] & (1u64 << b.dst.local)) != 0 {
                    let seed = 1u64 << b.src.local;
                    if (reached[cu] & seed) == 0 {
                        reached[cu] |= Self::island_closure(&self.islands[cu], seed);
                        changed = true;
                    }
                }
            }
            if !changed { break; }
        }

        (reached[dst.cluster as usize] & (1u64 << dst.local)) != 0
    }

    /// Verifica 4 pares críticos jerárquicos → máscara de 4 bits.
    pub fn verify_hierarchical_pairs(
        &self,
        sources:          &[HierarchicalNode; 4],
        sinks_cluster:    &[u8; 4],
        sinks_local_mask: &[u64; 4],
    ) -> u8 {
        // Caso K = 1 sin puentes → delegar al filtro plano de dominio único bit a bit.
        // La guarda de máscara vacía se aplica antes de delegar.
        if K == 1 && self.bridges.count == 0 {
            let flat = [sources[0].local, sources[1].local, sources[2].local, sources[3].local];
            // Prohibición de satisfacción vacua: si la máscara del carril es 0, el bit queda apagado.
            let base = CausalReachabilityFilter::verify_critical_pairs(&self.islands[0], &flat, sinks_local_mask);
            let mut result = 0u8;
            for (lane, &mask) in sinks_local_mask.iter().enumerate().take(4) {
                if mask != 0 && (base & (1u8 << lane)) != 0 {
                    result |= 1u8 << lane;
                }
            }
            return result;
        }

        let mut mask = 0u8;
        for lane in 0..4 {
            let sc = sinks_cluster[lane] as usize;
            if sc >= K || Self::validate(sources[lane]).is_err() { continue; }
            if sinks_local_mask[lane] == 0 { continue; } // Prohibición de satisfacción vacua en máscaras nulas

            let mut m = sinks_local_mask[lane];
            let mut ok = false;
            while m != 0 {
                let bit = m.trailing_zeros() as u8;
                if let Ok(dst) = HierarchicalNode::new(sinks_cluster[lane], bit) {
                    if self.is_reachable(sources[lane], dst) { ok = true; break; }
                }
                m &= m - 1;
            }
            if ok { mask |= 1u8 << lane; }
        }
        mask
    }
}

impl<const K: usize> Default for HierarchicalBitGraph<K> {
    fn default() -> Self {
        Self::new()
    }
}
