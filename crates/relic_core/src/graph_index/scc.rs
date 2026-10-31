//! # Componentes fuertemente conexas
//!
//! Kosaraju iterativo con arrays fijos. Dos pasadas: orden de finalización
//! sobre sucesores y asignación de componentes sobre predecesores. No hay
//! recursión, así que la profundidad no depende de la pila del hilo.

use super::{IndexFault, MAX_COMPONENTS, MAX_INDEX_NODES, NODE_WORDS};
use crate::episodic_memory::EpisodicMemory;

#[inline]
fn visited(set: &[u64; NODE_WORDS], id: u32) -> bool {
    set[(id as usize) >> 6] & (1u64 << (id & 63)) != 0
}

#[inline]
fn mark(set: &mut [u64; NODE_WORDS], id: u32) {
    set[(id as usize) >> 6] |= 1u64 << (id & 63);
}

/// Asigna a cada nodo `0..n` su componente en `component`.
/// Devuelve el número de componentes o `TooManyComponents`.
pub(super) fn components(
    mem: &EpisodicMemory,
    n: usize,
    component: &mut [u16; MAX_INDEX_NODES],
) -> Result<usize, IndexFault> {
    let mut order = [0u32; MAX_INDEX_NODES];
    let mut order_len = 0usize;

    // Pasada 1: orden de finalización (sucesores).
    let mut seen = [0u64; NODE_WORDS];
    let mut cursor = [0u32; MAX_INDEX_NODES];
    let mut stack = [0u32; MAX_INDEX_NODES];
    for start in 0..n as u32 {
        if visited(&seen, start) {
            continue;
        }
        mark(&mut seen, start);
        cursor[start as usize] = 0;
        stack[0] = start;
        let mut sp = 1usize;
        while sp > 0 {
            let u = stack[sp - 1];
            let nbrs = mem.get_neighbors_u32(u);
            let mut i = cursor[u as usize] as usize;
            let mut descended = false;
            while i < nbrs.len() {
                let v = nbrs[i];
                i += 1;
                if (v as usize) < n && !visited(&seen, v) {
                    mark(&mut seen, v);
                    cursor[u as usize] = i as u32;
                    cursor[v as usize] = 0;
                    stack[sp] = v;
                    sp += 1;
                    descended = true;
                    break;
                }
            }
            if !descended {
                cursor[u as usize] = i as u32;
                sp -= 1;
                order[order_len] = u;
                order_len += 1;
            }
        }
    }

    // Pasada 2: asignación sobre predecesores, en orden inverso de finalización.
    for slot in component[..n].iter_mut() {
        *slot = u16::MAX;
    }
    let mut count = 0usize;
    for idx in (0..order_len).rev() {
        let start = order[idx];
        if component[start as usize] != u16::MAX {
            continue;
        }
        if count >= MAX_COMPONENTS {
            return Err(IndexFault::TooManyComponents { max: MAX_COMPONENTS });
        }
        let cid = count as u16;
        component[start as usize] = cid;
        stack[0] = start;
        let mut sp = 1usize;
        while sp > 0 {
            sp -= 1;
            let u = stack[sp];
            for &v in mem.get_predecessors_u32(u) {
                if (v as usize) < n && component[v as usize] == u16::MAX {
                    component[v as usize] = cid;
                    stack[sp] = v;
                    sp += 1;
                }
            }
        }
        count += 1;
    }

    debug_assert_eq!(order_len, n);
    Ok(count)
}
