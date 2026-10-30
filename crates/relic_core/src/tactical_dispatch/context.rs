//! # TacticalKernelContext & ReflexTickInput
//!
//! Objetos de contexto (Context Object Pattern) para el paso de parámetros
//! en registros de CPU sin sobrecargar la pila de ejecución.

use crate::anomaly::topology_coupling::{NodeSpatialNeighborhood, ResidualSprtMartingale};
use crate::blackwall_scars::{StructuralDiscriminationEngine, WL1Signature};
use crate::contingency_bfs::{BitAdjacencyGraph, PreCertifiedContingencyLibrary};
use crate::repair::EdgeOperatingBounds;
use super::ring_buffer::StagedOrderRingBuffer;

/// Contexto mutable del kernel de control táctico.
pub struct TacticalKernelContext<'a> {
    pub martingale: &'a mut ResidualSprtMartingale,
    pub scar_engine: &'a mut StructuralDiscriminationEngine,
    pub contingency_lib: &'a PreCertifiedContingencyLibrary,
    pub graph: &'a mut BitAdjacencyGraph,
    pub bounds: &'a mut [EdgeOperatingBounds],
    pub ring_buffer: &'a StagedOrderRingBuffer,
}

/// Parámetros inmutables de entrada para 1 tick reflejo.
#[derive(Copy, Clone, Debug)]
pub struct ReflexTickInput<'a> {
    pub current_tick: u64,
    pub target_edge: (u8, u8, u32),
    pub monitored_node: u8,
    pub sensor_snapshot: &'a [f32],
    pub neighborhood: &'a NodeSpatialNeighborhood,
    pub wl1_sig: &'a WL1Signature,
    pub sources: &'a [u8; 4],
    pub sinks_mask: &'a [u64; 4],
}

