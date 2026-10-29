//! # mask256
//!
//! Mascara SIMD de 256 bits alineada a 32 bytes para operaciones contrafactuales paralelas.
//! Menos de 180 lineas.

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C, align(32))]
pub struct Mask256(pub [u64; 4]);

impl Mask256 {
    pub const ZERO: Self = Mask256([0; 4]);
    pub const ALL: Self = Mask256([!0u64; 4]);

    #[inline(always)]
    pub fn is_zero(&self) -> bool { (self.0[0] | self.0[1] | self.0[2] | self.0[3]) == 0 }

    #[inline(always)]
    pub fn set_bit(&mut self, bit: usize) {
        if bit < 256 { self.0[bit / 64] |= 1u64 << (bit % 64); }
    }

    #[inline(always)]
    pub fn get_bit(&self, bit: usize) -> bool {
        if bit >= 256 { false } else { (self.0[bit / 64] & (1u64 << (bit % 64))) != 0 }
    }

    #[inline(always)]
    pub fn from_bit(bit: usize) -> Self {
        let mut m = Self::ZERO; m.set_bit(bit); m
    }

    pub fn ones(k: usize) -> Self {
        let mut m = Self::ZERO;
        for i in 0..k.min(256) { m.set_bit(i); }
        m
    }
}

impl BitOr for Mask256 {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Mask256([self.0[0] | rhs.0[0], self.0[1] | rhs.0[1], self.0[2] | rhs.0[2], self.0[3] | rhs.0[3]])
    }
}

impl BitOrAssign for Mask256 {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0[0] |= rhs.0[0]; self.0[1] |= rhs.0[1]; self.0[2] |= rhs.0[2]; self.0[3] |= rhs.0[3];
    }
}

impl BitAnd for Mask256 {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self {
        Mask256([self.0[0] & rhs.0[0], self.0[1] & rhs.0[1], self.0[2] & rhs.0[2], self.0[3] & rhs.0[3]])
    }
}

impl BitAndAssign for Mask256 {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0[0] &= rhs.0[0]; self.0[1] &= rhs.0[1]; self.0[2] &= rhs.0[2]; self.0[3] &= rhs.0[3];
    }
}

impl Not for Mask256 {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self { Mask256([!self.0[0], !self.0[1], !self.0[2], !self.0[3]]) }
}

