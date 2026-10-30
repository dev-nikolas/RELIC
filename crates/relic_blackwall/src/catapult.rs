//! Catapulta Causal Bitwise para la frontera del Blackwall.
//! Evalúa cortes inductivos en 1 ciclo de ALU para omitir regresiones inocuas.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C, align(32))]
pub struct RegressionSuiteMask(pub [u64; 4]);

impl RegressionSuiteMask {
    pub const ZERO: Self = Self([0; 4]);
    pub const ALL: Self = Self([!0u64; 4]);

    #[inline(always)]
    pub fn is_zero(&self) -> bool {
        (self.0[0] | self.0[1] | self.0[2] | self.0[3]) == 0
    }

    #[inline(always)]
    pub fn set_bit(&mut self, bit: usize) {
        if bit < 256 {
            self.0[bit / 64] |= 1u64 << (bit % 64);
        }
    }

    #[inline(always)]
    pub fn get_bit(&self, bit: usize) -> bool {
        if bit >= 256 { false } else { (self.0[bit / 64] & (1u64 << (bit % 64))) != 0 }
    }

    #[inline(always)]
    pub fn iter_indices(&self) -> impl Iterator<Item = usize> {
        let mut indices = Vec::new();
        for (w, &word) in self.0.iter().enumerate() {
            let mut bits = word;
            while bits != 0 {
                let bit_idx = w * 64 + bits.trailing_zeros() as usize;
                indices.push(bit_idx);
                bits &= bits - 1;
            }
        }
        indices.into_iter()
    }
}

impl std::ops::BitAnd for RegressionSuiteMask {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self {
        Self([
            self.0[0] & rhs.0[0],
            self.0[1] & rhs.0[1],
            self.0[2] & rhs.0[2],
            self.0[3] & rhs.0[3],
        ])
    }
}

impl std::ops::BitOr for RegressionSuiteMask {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Self([
            self.0[0] | rhs.0[0],
            self.0[1] | rhs.0[1],
            self.0[2] | rhs.0[2],
            self.0[3] | rhs.0[3],
        ])
    }
}

impl std::ops::Not for RegressionSuiteMask {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self {
        Self([!self.0[0], !self.0[1], !self.0[2], !self.0[3]])
    }
}

#[derive(Debug, Clone, Default)]
pub struct CausalCatapult {
    pub in_cut_bitset: Vec<u64>,
    pub node_to_mask_idx: Vec<u32>,
    pub cut_masks: Vec<RegressionSuiteMask>,
    pub unbounded_closed_cut_mask: RegressionSuiteMask,
}

impl CausalCatapult {
    pub fn new(capacity: usize) -> Self {
        let words = capacity.div_ceil(64);
        Self {
            in_cut_bitset: vec![0u64; words],
            node_to_mask_idx: vec![u32::MAX; capacity],
            cut_masks: Vec::new(),
            unbounded_closed_cut_mask: RegressionSuiteMask::ZERO,
        }
    }

    pub fn register_cut_node(&mut self, node: u32, test_idx: usize) {
        let u_idx = node as usize;
        let words = (u_idx + 64) / 64;
        if self.in_cut_bitset.len() < words {
            self.in_cut_bitset.resize(words, 0);
        }
        if self.node_to_mask_idx.len() <= u_idx {
            self.node_to_mask_idx.resize(u_idx + 1, u32::MAX);
        }

        self.in_cut_bitset[u_idx / 64] |= 1u64 << (u_idx % 64);
        let slot = self.node_to_mask_idx[u_idx];
        if slot == u32::MAX {
            let new_slot = self.cut_masks.len() as u32;
            let mut mask = RegressionSuiteMask::ZERO;
            mask.set_bit(test_idx);
            self.cut_masks.push(mask);
            self.node_to_mask_idx[u_idx] = new_slot;
        } else if (slot as usize) < self.cut_masks.len() {
            self.cut_masks[slot as usize].set_bit(test_idx);
        }
    }

    #[inline(always)]
    pub fn get_cut_mask(&self, node: u32) -> RegressionSuiteMask {
        let idx = node as usize;
        let word = idx >> 6;
        if word >= self.in_cut_bitset.len() || (self.in_cut_bitset[word] & (1u64 << (idx & 63))) == 0 {
            return RegressionSuiteMask::ZERO;
        }
        if idx >= self.node_to_mask_idx.len() {
            return RegressionSuiteMask::ZERO;
        }
        let slot = self.node_to_mask_idx[idx];
        if slot == u32::MAX || (slot as usize) >= self.cut_masks.len() {
            RegressionSuiteMask::ZERO
        } else {
            self.cut_masks[slot as usize]
        }
    }

    /// Evalúa la adición de arista u -> v contra los cortes cerrados activos.
    /// Retorna RegressionSuiteMask::ZERO si u -> v NO cruza ningún corte inductivo cerrado.
    #[inline(always)]
    pub fn evaluate_edge_addition(&self, u: u32, v: u32) -> RegressionSuiteMask {
        let u_idx = u as usize;
        let u_word = u_idx >> 6;
        let u_bit = u_idx & 63;

        // Fast-path de 1 ciclo: si u no está en ningún corte inductivo, breaks es estrictamente cero.
        if u_word >= self.in_cut_bitset.len() || (self.in_cut_bitset[u_word] & (1u64 << u_bit)) == 0 {
            return RegressionSuiteMask::ZERO;
        }

        let mask_u = self.get_cut_mask(u);
        let mask_v = self.get_cut_mask(v);

        // Cruza el corte <=> u in S_i AND v NOT in S_i (restringido a cortes cerrados)
        mask_u & !mask_v & self.unbounded_closed_cut_mask
    }
}

