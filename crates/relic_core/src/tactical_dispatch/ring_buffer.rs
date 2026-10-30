//! # StagedOrderRingBuffer & SPSC Lock-Free Mechanics
//!
//! Búfer circular Single-Producer Single-Consumer (SPSC) 100% Lock-Free y Zero-Alloc.
//! Punteros atómicos alineados a 64 bytes para evitar false sharing entre cores de CPU.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::repair::{EdgeOperatingBounds, LocalConstraintRepairer, RepairOutcome, StagedEnvelope};

pub const RING_CAPACITY: usize = 16;
pub const RING_MASK: usize = RING_CAPACITY - 1;

/// Entero atómico con padding de 64 bytes para residir en su propia línea de caché L1.
#[repr(C, align(64))]
pub struct CachePaddedAtomicUsize {
    pub value: AtomicUsize,
    pub _pad: [u8; 56],
}

impl CachePaddedAtomicUsize {
    pub const fn new(val: usize) -> Self {
        Self {
            value: AtomicUsize::new(val),
            _pad: [0u8; 56],
        }
    }
}

/// Slot contiguo para una directiva pre-aprobada en el ring buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StagedPolicySlot {
    pub envelope: StagedEnvelope,
    pub valid_until_tick: u64,
    pub target_critical_edge: (u8, u8, u32),
    pub _padding: [u8; 2],
}

impl StagedPolicySlot {
    pub const fn empty() -> Self {
        Self {
            envelope: StagedEnvelope::new(0),
            valid_until_tick: 0,
            target_critical_edge: (0, 0, 0),
            _padding: [0u8; 2],
        }
    }
}

/// Búfer Circular SPSC Lock-Free alineado a líneas de caché.
#[repr(C, align(64))]
pub struct StagedOrderRingBuffer {
    pub head: CachePaddedAtomicUsize,
    pub tail: CachePaddedAtomicUsize,
    pub slots: [UnsafeCell<StagedPolicySlot>; RING_CAPACITY],
}

unsafe impl Sync for StagedOrderRingBuffer {}
unsafe impl Send for StagedOrderRingBuffer {}

impl StagedOrderRingBuffer {
    pub const fn new() -> Self {
        const EMPTY_SLOT: UnsafeCell<StagedPolicySlot> = UnsafeCell::new(StagedPolicySlot::empty());
        Self {
            head: CachePaddedAtomicUsize::new(0),
            tail: CachePaddedAtomicUsize::new(0),
            slots: [EMPTY_SLOT; RING_CAPACITY],
        }
    }

    pub fn try_stage_from_cognitive(
        &self,
        mut envelope: StagedEnvelope,
        valid_until_tick: u64,
        target_edge: (u8, u8, u32),
        bounds: &[EdgeOperatingBounds],
        max_total_budget: f64,
    ) -> Result<RepairOutcome, &'static str> {
        let outcome = LocalConstraintRepairer::repair_or_veto(&mut envelope, bounds, max_total_budget);

        if let RepairOutcome::VetoedIrreparable { .. } = outcome {
            return Ok(outcome);
        }

        let head = self.head.value.load(Ordering::Relaxed);
        let tail = self.tail.value.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= RING_CAPACITY {
            return Err("Búfer SPSC saturado: capacidad máxima 16 slots alcanzada");
        }

        let slot = StagedPolicySlot {
            envelope,
            valid_until_tick,
            target_critical_edge: target_edge,
            _padding: [0u8; 2],
        };

        let idx = head & RING_MASK;
        unsafe {
            *self.slots[idx].get() = slot;
        }

        self.head.value.store(head.wrapping_add(1), Ordering::Release);
        Ok(outcome)
    }

    #[inline(always)]
    pub fn try_pop_tactical(&self) -> Option<StagedPolicySlot> {
        let tail = self.tail.value.load(Ordering::Relaxed);
        let head = self.head.value.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let idx = tail & RING_MASK;
        let slot = unsafe { *self.slots[idx].get() };
        self.tail.value.store(tail.wrapping_add(1), Ordering::Release);
        Some(slot)
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.head.value.load(Ordering::Relaxed) == self.tail.value.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.head.value.load(Ordering::Relaxed).wrapping_sub(self.tail.value.load(Ordering::Relaxed))
    }
}

impl Default for StagedOrderRingBuffer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

