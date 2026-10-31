//! # ReachHierarchy
//!
//! Índice de alcanzabilidad jerárquico y exacto sobre `EpisodicMemory`.
//!
//! Dos niveles: cada nodo se mapea a su componente fuertemente conexa (SCC) y
//! el cierre transitivo se guarda **solo entre componentes**. Como todos los
//! nodos de una SCC son mutuamente alcanzables, la consulta es un test de bit.
//!
//! Propiedades:
//!
//! - **Exacto**: no hay sobre-aproximación. `Unreachable` es una respuesta
//!   demostrada, no una estimación.
//! - **Cero heap**: construir y consultar solo escriben en arrays fijos. Las
//!   únicas lecturas son `interner.len`, `get_neighbors_u32` y
//!   `get_predecessors_u32`, que no asignan.
//! - **Abstención formal**: si el grafo excede la capacidad, `build` devuelve
//!   `IndexFault` y no se construye un índice parcial. Nunca hay resultados
//!   aproximados.
//!
//! Límite de capacidad: 1024 nodos y 512 componentes. Un DAG de más de 512
//! nodos se abstiene (`TooManyComponents`); el llamador conserva el BFS
//! existente como respaldo.

mod closure;
mod scc;

use crate::episodic_memory::EpisodicMemory;

/// Nodos indexables. Los ids del interner deben ser contiguos `0..n`.
pub const MAX_INDEX_NODES: usize = 1024;
/// Componentes fuertemente conexas indexables.
pub const MAX_COMPONENTS: usize = 512;

pub(crate) const WORDS: usize = MAX_COMPONENTS / 64;
pub(crate) const NODE_WORDS: usize = MAX_INDEX_NODES / 64;

const _: () = assert!(MAX_COMPONENTS % 64 == 0 && MAX_INDEX_NODES % 64 == 0);

/// Motivo formal por el que el índice no se construye.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFault {
    TooManyNodes { nodes: usize, max: usize },
    TooManyComponents { max: usize },
}

/// Respuesta de una consulta. No existe `Indeterminate`: el índice es exacto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachVerdict {
    Reachable,
    Unreachable,
    OutOfRange,
}

#[repr(C)]
pub struct ReachHierarchy {
    closure: [[u64; WORDS]; MAX_COMPONENTS],
    component: [u16; MAX_INDEX_NODES],
    nodes: u16,
    components: u16,
    _pad: [u8; 4],
}

const _: () = assert!(core::mem::size_of::<ReachHierarchy>() == 34_824);

impl ReachHierarchy {
    /// Construye el índice. Devuelve `Err` si el grafo excede la capacidad.
    pub fn build(mem: &EpisodicMemory) -> Result<Self, IndexFault> {
        let n = mem.interner.len();
        if n > MAX_INDEX_NODES {
            return Err(IndexFault::TooManyNodes { nodes: n, max: MAX_INDEX_NODES });
        }
        let mut index = Self::empty(n);
        let count = scc::components(mem, n, &mut index.component)?;
        index.components = count as u16;
        closure::fill(mem, n, &index.component, count, &mut index.closure);
        Ok(index)
    }

    fn empty(n: usize) -> Self {
        Self {
            closure: [[0u64; WORDS]; MAX_COMPONENTS],
            component: [u16::MAX; MAX_INDEX_NODES],
            nodes: n as u16,
            components: 0,
            _pad: [0u8; 4],
        }
    }

    /// `Reachable` / `Unreachable` son exactos; `OutOfRange` si algún id ≥ n.
    #[inline]
    pub fn reaches(&self, u: u32, v: u32) -> ReachVerdict {
        if u >= self.nodes as u32 || v >= self.nodes as u32 {
            return ReachVerdict::OutOfRange;
        }
        let (cu, cv) = (self.component[u as usize], self.component[v as usize]);
        if cu == cv {
            return ReachVerdict::Reachable;
        }
        let (cu, cv) = (cu as usize, cv as usize);
        if self.closure[cu][cv >> 6] & (1u64 << (cv & 63)) != 0 {
            ReachVerdict::Reachable
        } else {
            ReachVerdict::Unreachable
        }
    }

    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes as usize
    }

    #[inline]
    pub fn component_count(&self) -> usize {
        self.components as usize
    }

    /// Componente de `node`, o `None` si está fuera del índice.
    #[inline]
    pub fn component_of(&self, node: u32) -> Option<u16> {
        if node >= self.nodes as u32 {
            return None;
        }
        let c = self.component[node as usize];
        if c == u16::MAX {
            None
        } else {
            Some(c)
        }
    }
}

#[cfg(test)]
mod tests;
