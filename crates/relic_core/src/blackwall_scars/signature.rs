//! # WL1Signature & CriticalEdgeKey
//!
//! Firmas topológicas canónicas de 1-hop invariantes a permutaciones.
//! Ordenamiento en stack 100% branchless mediante redes de comparación fijas (Bose-Nelson).

/// Clave compacta empaquetada de una arista crítica u64 = (u << 32) | v con u < v.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CriticalEdgeKey(pub u64);

impl CriticalEdgeKey {
    #[inline(always)]
    pub const fn from_nodes(u: u32, v: u32) -> Self {
        if u <= v {
            Self(((u as u64) << 32) | (v as u64))
        } else {
            Self(((v as u64) << 32) | (u as u64))
        }
    }

    #[inline(always)]
    pub const fn u(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    #[inline(always)]
    pub const fn v(&self) -> u32 {
        self.0 as u32
    }
}

/// Etiqueta topológica compacta de un vecino 1-hop (rol, grado, estado de aislamiento).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NeighborTopologyToken(pub u16);

impl NeighborTopologyToken {
    #[inline(always)]
    pub const fn new(role: u8, degree: u8, isolated: bool) -> Self {
        let iso_bit = if isolated { 1u16 << 15 } else { 0 };
        let role_bits = (role as u16) << 8;
        let deg_bits = degree as u16;
        Self(iso_bit | role_bits | deg_bits)
    }

    #[inline(always)]
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

/// Firma Canónica Weisfeiler-Lehman 1-Hop (128 bits, alineada a 16 bytes).
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WL1Signature(pub [u8; 16]);

const _: () = assert!(std::mem::size_of::<WL1Signature>() == 16);
const _: () = assert!(std::mem::align_of::<WL1Signature>() == 16);

impl WL1Signature {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self([0u8; 16])
    }

    #[inline(always)]
    fn sort_pair(arr: &mut [u16; 8], i: usize, j: usize) {
        let a = arr[i];
        let b = arr[j];
        arr[i] = a.min(b);
        arr[j] = a.max(b);
    }

    /// Red de ordenamiento fija Bose-Nelson para 8 elementos: 100% Branchless.
    #[inline(always)]
    fn branchless_sort_8(arr: &mut [u16; 8]) {
        Self::sort_pair(arr, 0, 1);
        Self::sort_pair(arr, 2, 3);
        Self::sort_pair(arr, 4, 5);
        Self::sort_pair(arr, 6, 7);

        Self::sort_pair(arr, 0, 2);
        Self::sort_pair(arr, 1, 3);
        Self::sort_pair(arr, 4, 6);
        Self::sort_pair(arr, 5, 7);

        Self::sort_pair(arr, 1, 2);
        Self::sort_pair(arr, 5, 6);
        Self::sort_pair(arr, 0, 4);
        Self::sort_pair(arr, 3, 7);

        Self::sort_pair(arr, 1, 5);
        Self::sort_pair(arr, 2, 6);

        Self::sort_pair(arr, 1, 4);
        Self::sort_pair(arr, 3, 6);

        Self::sort_pair(arr, 2, 4);
        Self::sort_pair(arr, 3, 5);

        Self::sort_pair(arr, 2, 3);
    }

    /// Genera la firma canónica ordenando vecindades en stack con redes branchless.
    pub fn compute_canonical(
        u_role: u8,
        v_role: u8,
        u_neighbors: &[u16],
        v_neighbors: &[u16],
    ) -> Self {
        let mut u_sorted = [u16::MAX; 8];
        let mut v_sorted = [u16::MAX; 8];

        let u_count = u_neighbors.len().min(8);
        let v_count = v_neighbors.len().min(8);

        for i in 0..u_count {
            u_sorted[i] = u_neighbors[i];
        }
        for i in 0..v_count {
            v_sorted[i] = v_neighbors[i];
        }

        Self::branchless_sort_8(&mut u_sorted);
        Self::branchless_sort_8(&mut v_sorted);

        #[inline(always)]
        fn fmix64(mut k: u64) -> u64 {
            k ^= k >> 33;
            k = k.wrapping_mul(0xff51afd7ed558ccd);
            k ^= k >> 33;
            k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
            k ^= k >> 33;
            k
        }

        let mut h_u = 0x9e3779b97f4a7c15u64 ^ ((u_role as u64) << 32) ^ (u_count as u64);
        for i in 0..u_count {
            h_u = fmix64(h_u ^ ((u_sorted[i] as u64) << (i * 8)));
        }

        let mut h_v = 0xbf58476d1ce4e5b9u64 ^ ((v_role as u64) << 32) ^ (v_count as u64);
        for i in 0..v_count {
            h_v = fmix64(h_v ^ ((v_sorted[i] as u64) << (i * 8)));
        }

        let mut sig = [0u8; 16];
        sig[0..8].copy_from_slice(&h_u.to_le_bytes());
        sig[8..16].copy_from_slice(&h_v.to_le_bytes());
        Self(sig)
    }

    /// Comparación SIMD SSE4.1 ultra-rápida en 1 ciclo de reloj.
    #[inline(always)]
    pub fn matches_simd(&self, other: &Self) -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            #[cfg(target_feature = "sse4.1")]
            {
                return unsafe {
                    use core::arch::x86_64::*;
                    let a = _mm_load_si128(self.0.as_ptr() as *const __m128i);
                    let b = _mm_load_si128(other.0.as_ptr() as *const __m128i);
                    let xor_res = _mm_xor_si128(a, b);
                    _mm_testz_si128(xor_res, xor_res) == 1
                };
            }
            #[cfg(not(target_feature = "sse4.1"))]
            {
                if is_x86_feature_detected!("sse4.1") {
                    return unsafe {
                        use core::arch::x86_64::*;
                        let a = _mm_load_si128(self.0.as_ptr() as *const __m128i);
                        let b = _mm_load_si128(other.0.as_ptr() as *const __m128i);
                        let xor_res = _mm_xor_si128(a, b);
                        _mm_testz_si128(xor_res, xor_res) == 1
                    };
                }
            }
        }
        self.0 == other.0
    }
}
