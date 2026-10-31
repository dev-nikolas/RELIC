//! # Cierre transitivo sobre componentes
//!
//! Floyd-Warshall con bitsets. El bucle usa `count`, no la capacidad máxima,
//! así que un grafo pequeño no paga el coste de 512 componentes.

use super::{MAX_COMPONENTS, MAX_INDEX_NODES, WORDS};
use crate::episodic_memory::EpisodicMemory;

/// Rellena `closure` a partir de las aristas del grafo y la partición en SCC.
/// La diagonal se marca antes del cierre: todo componente alcanza a sí mismo.
pub(super) fn fill(
    mem: &EpisodicMemory,
    n: usize,
    component: &[u16; MAX_INDEX_NODES],
    count: usize,
    closure: &mut [[u64; WORDS]; MAX_COMPONENTS],
) {
    for c in 0..count {
        closure[c][c >> 6] |= 1u64 << (c & 63);
    }
    for u in 0..n as u32 {
        let cu = component[u as usize] as usize;
        for &v in mem.get_neighbors_u32(u) {
            let vi = v as usize;
            if vi < n {
                let cv = component[vi] as usize;
                closure[cu][cv >> 6] |= 1u64 << (cv & 63);
            }
        }
    }
    for k in 0..count {
        let row = closure[k];
        let (word, bit) = (k >> 6, 1u64 << (k & 63));
        for i in 0..count {
            if closure[i][word] & bit != 0 {
                for w in 0..WORDS {
                    closure[i][w] |= row[w];
                }
            }
        }
    }
}
