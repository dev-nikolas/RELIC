//! # BitAdjacencyGraph
//!
//! Representación compacta de matriz de adyacencia de bits para 64 nodos (512 bytes).
//! Reside completamente en 8 líneas de caché L1 con aislamiento y restauración O(1).

pub const MAX_GRAPH_NODES: usize = 64;

/// Matriz de adyacencia por máscaras de bits alineada a 32 bytes (512 bytes en RAM).
#[repr(C, align(32))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BitAdjacencyGraph {
    pub adj: [u64; MAX_GRAPH_NODES],
}

const _: () = assert!(std::mem::size_of::<BitAdjacencyGraph>() == 512);

impl BitAdjacencyGraph {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            adj: [0u64; MAX_GRAPH_NODES],
        }
    }

    #[inline(always)]
    pub fn add_bidirectional_edge(&mut self, u: u8, v: u8) {
        let u_idx = (u as usize) % MAX_GRAPH_NODES;
        let v_idx = (v as usize) % MAX_GRAPH_NODES;
        self.adj[u_idx] |= 1u64 << v_idx;
        self.adj[v_idx] |= 1u64 << u_idx;
    }

    #[inline(always)]
    pub fn quarantine_edge(&mut self, u: u8, v: u8) {
        let u_idx = (u as usize) % MAX_GRAPH_NODES;
        let v_idx = (v as usize) % MAX_GRAPH_NODES;
        self.adj[u_idx] &= !(1u64 << v_idx);
        self.adj[v_idx] &= !(1u64 << u_idx);
    }

    #[inline(always)]
    pub fn restore_edge(&mut self, u: u8, v: u8) {
        let u_idx = (u as usize) % MAX_GRAPH_NODES;
        let v_idx = (v as usize) % MAX_GRAPH_NODES;
        self.adj[u_idx] |= 1u64 << v_idx;
        self.adj[v_idx] |= 1u64 << u_idx;
    }

    #[inline(always)]
    pub fn is_connected(&self, u: u8, v: u8) -> bool {
        let u_idx = (u as usize) % MAX_GRAPH_NODES;
        let v_idx = (v as usize) % MAX_GRAPH_NODES;
        (self.adj[u_idx] & (1u64 << v_idx)) != 0
    }
}

impl Default for BitAdjacencyGraph {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

