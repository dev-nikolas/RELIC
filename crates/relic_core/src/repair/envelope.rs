//! # StagedEnvelope & RepairOutcome
//!
//! Envolventes de directivas y resultados de la proyección convexa.
//! Memoria contigua sin indirecciones ni asignaciones en heap.

use super::types::UnsatCore;

pub const MAX_STAGED_OPS: usize = 8;

/// Estado del ciclo de vida de un StagedEnvelope.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnvelopeStatus {
    Draft = 0,
    Certified = 1,
    Repaired = 2,
    Vetoed = 3,
}

/// Operación atómica sobre una arista del grafo.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StagedOperation {
    pub edge_id: u32,
    pub requested_val: f64,
    pub current_val: f64,
    pub op_type: u8,
    pub repaired: bool,
    pub certified: bool,
    pub _padding: [u8; 5],
}

impl StagedOperation {
    pub const fn empty() -> Self {
        Self {
            edge_id: 0,
            requested_val: 0.0,
            current_val: 0.0,
            op_type: 0,
            repaired: false,
            certified: false,
            _padding: [0u8; 5],
        }
    }
}

/// Envolvente de directivas u órdenes pre-escenificadas en memoria continua.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StagedEnvelope {
    pub envelope_id: u64,
    pub ops: [StagedOperation; MAX_STAGED_OPS],
    pub op_count: u8,
    pub status: EnvelopeStatus,
    pub _padding: [u8; 6],
}

impl StagedEnvelope {
    pub const fn new(envelope_id: u64) -> Self {
        Self {
            envelope_id,
            ops: [StagedOperation::empty(); MAX_STAGED_OPS],
            op_count: 0,
            status: EnvelopeStatus::Draft,
            _padding: [0u8; 6],
        }
    }

    pub fn add_op(
        &mut self,
        edge_id: u32,
        requested_val: f64,
        current_val: f64,
        op_type: u8,
    ) -> Result<(), &'static str> {
        if (self.op_count as usize) >= MAX_STAGED_OPS {
            return Err("StagedEnvelope saturado: capacidad máxima 8 operaciones");
        }
        let idx = self.op_count as usize;
        self.ops[idx] = StagedOperation {
            edge_id,
            requested_val,
            current_val,
            op_type,
            repaired: false,
            certified: false,
            _padding: [0u8; 5],
        };
        self.op_count += 1;
        Ok(())
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.op_count as usize
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.op_count == 0
    }

    #[inline(always)]
    pub fn ops_slice(&self) -> &[StagedOperation] {
        &self.ops[0..self.len()]
    }

    #[inline(always)]
    pub fn ops_mut_slice(&mut self) -> &mut [StagedOperation] {
        let n = self.op_count as usize;
        &mut self.ops[0..n]
    }
}

/// Resultado de la verificación formal y reparación en tiempo de ejecución.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RepairOutcome {
    CertifiedAsIs,
    RepairedInPlace {
        core: UnsatCore,
        repaired_ops_mask: u8,
    },
    VetoedIrreparable {
        core: UnsatCore,
    },
}

