//! # CausalReachabilityFilter
//!
//! Algoritmo Multi-Source Breadth-First Search (MS-BFS) bit-paralelo evaluando
//! 4 fuentes simultáneas en registros vectoriales de 256 bits (AVX2).

use super::graph::{BitAdjacencyGraph, MAX_GRAPH_NODES};

#[repr(C, align(32))]
#[derive(Copy, Clone, Debug)]
pub struct AlignedSimd<T>(pub T);

/// Filtro de alcanzabilidad causal determinista mediante MS-BFS vectorial.
pub struct CausalReachabilityFilter;

impl CausalReachabilityFilter {
    pub fn verify_critical_pairs(
        graph: &BitAdjacencyGraph,
        sources: &[u8; 4],
        sinks_mask: &[u64; 4],
    ) -> u8 {
        let mut visited = AlignedSimd([
            1u64 << ((sources[0] as usize) % MAX_GRAPH_NODES),
            1u64 << ((sources[1] as usize) % MAX_GRAPH_NODES),
            1u64 << ((sources[2] as usize) % MAX_GRAPH_NODES),
            1u64 << ((sources[3] as usize) % MAX_GRAPH_NODES),
        ]);

        let mut frontier = AlignedSimd(visited.0);

        for _ in 0..MAX_GRAPH_NODES {
            let mut next_raw = AlignedSimd([0u64; 4]);

            for k in 0..4 {
                let mut f = frontier.0[k];
                let mut union_adj = 0u64;
                while f != 0 {
                    let u = f.trailing_zeros() as usize;
                    union_adj |= graph.adj[u];
                    f &= f - 1;
                }
                next_raw.0[k] = union_adj;
            }

            let has_new_frontier = Self::avx2_step(&mut frontier.0, &mut visited.0, &next_raw.0);
            if !has_new_frontier {
                break;
            }
        }

        let mut satisfaction_mask = 0u8;
        for k in 0..4 {
            let reached = visited.0[k];
            let required = sinks_mask[k];
            if (reached & required) == required {
                satisfaction_mask |= 1 << k;
            }
        }

        satisfaction_mask
    }

    #[inline(always)]
    fn avx2_step(frontier: &mut [u64; 4], visited: &mut [u64; 4], next_raw: &[u64; 4]) -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            #[cfg(target_feature = "avx2")]
            {
                return unsafe {
                    use core::arch::x86_64::*;
                    let v_vis = _mm256_load_si256(visited.as_ptr() as *const __m256i);
                    let v_raw = _mm256_load_si256(next_raw.as_ptr() as *const __m256i);
                    let v_new = _mm256_andnot_si256(v_vis, v_raw);
                    let v_updated_vis = _mm256_or_si256(v_vis, v_new);

                    _mm256_store_si256(visited.as_mut_ptr() as *mut __m256i, v_updated_vis);
                    _mm256_store_si256(frontier.as_mut_ptr() as *mut __m256i, v_new);

                    _mm256_testz_si256(v_new, v_new) == 0
                };
            }
            #[cfg(not(target_feature = "avx2"))]
            {
                if is_x86_feature_detected!("avx2") {
                    return unsafe {
                        use core::arch::x86_64::*;
                        let v_vis = _mm256_load_si256(visited.as_ptr() as *const __m256i);
                        let v_raw = _mm256_load_si256(next_raw.as_ptr() as *const __m256i);
                        let v_new = _mm256_andnot_si256(v_vis, v_raw);
                        let v_updated_vis = _mm256_or_si256(v_vis, v_new);

                        _mm256_store_si256(visited.as_mut_ptr() as *mut __m256i, v_updated_vis);
                        _mm256_store_si256(frontier.as_mut_ptr() as *mut __m256i, v_new);

                        _mm256_testz_si256(v_new, v_new) == 0
                    };
                }
            }
        }

        let mut changed = false;
        for k in 0..4 {
            let new_bits = next_raw[k] & !visited[k];
            visited[k] |= new_bits;
            frontier[k] = new_bits;
            if new_bits != 0 {
                changed = true;
            }
        }
        changed
    }
}

