//! # Causal Syndrome Decoding Types & Epistemic Scratchpad
//!
//! Tipos y layouts de memoria alineados a 64 bytes para la decodificación de
//! síndromes causales en tiempo constante O(1) y el búfer circular volátil con TTL.

/// Capacidad máxima de firmas de paridad topológica por matriz.
pub const MAX_SYNDROME_NODES: usize = 64;
/// Capacidad estática del scratchpad efímero de conjeturas epistémicas.
pub const SCRATCHPAD_CAPACITY: usize = 64;

/// Tipo de entidad causante de la distorsión del residuo topológico.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FaultClass {
    EdgeDisruption = 0,
    SensorCalibrationDrift = 1,
    ActuatorSaturating = 2,
    UnknownExogenous = 3,
}

/// Hipótesis elemental candidata a explicar el patrón de excitación de residuos.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FaultCandidate {
    pub target_id: u32,
    pub class: FaultClass,
    pub confidence_score: u8,
}

impl FaultCandidate {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            target_id: 0,
            class: FaultClass::UnknownExogenous,
            confidence_score: 0,
        }
    }
}

/// Veredicto formal emitido por el decodificador de paridad causal.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SyndromeVerdict {
    /// No hay anomalías detectables en el vector de residuos.
    NominalQuiescence,
    /// Causa inequívoca identificada con distancia de Hamming dominante.
    UnambiguousIdentification {
        primary: FaultCandidate,
        hamming_distance: u8,
    },
    /// Equifinalidad detectada: empate métrico entre al menos dos causas físicamente distintas.
    /// Prohíbe formalmente amputar la topología hasta desambiguar.
    EpistemicUncertainty {
        candidate_a: FaultCandidate,
        candidate_b: FaultCandidate,
        ambiguity_margin: u8,
    },
}

/// Registro efímero en el scratchpad de hipótesis con tiempo de vida (TTL).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ScratchpadEntry {
    pub candidate: FaultCandidate,
    pub observed_mask: u64,
    pub recorded_tick: u64,
    pub ttl_ticks: u32,
    pub active: bool,
    pub _pad: [u8; 3],
}

/// Búfer circular fijo en stack para conjeturas e incertidumbre epistémica (Zero-Alloc).
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct EpistemicScratchpad {
    pub entries: [ScratchpadEntry; SCRATCHPAD_CAPACITY],
    pub write_cursor: u8,
    pub active_count: u8,
    pub _pad: [u8; 62],
}

const _: () = assert!(std::mem::size_of::<EpistemicScratchpad>() == 2112);
const _: () = assert!(std::mem::align_of::<EpistemicScratchpad>() == 64);

impl EpistemicScratchpad {
    #[inline(always)]
    pub const fn new() -> Self {
        const EMPTY_ENTRY: ScratchpadEntry = ScratchpadEntry {
            candidate: FaultCandidate::empty(),
            observed_mask: 0,
            recorded_tick: 0,
            ttl_ticks: 0,
            active: false,
            _pad: [0u8; 3],
        };
        Self {
            entries: [EMPTY_ENTRY; SCRATCHPAD_CAPACITY],
            write_cursor: 0,
            active_count: 0,
            _pad: [0u8; 62],
        }
    }

    #[inline(always)]
    pub fn record_hypothesis(&mut self, candidate: FaultCandidate, mask: u64, tick: u64, ttl: u32) {
        let idx = (self.write_cursor as usize) % SCRATCHPAD_CAPACITY;
        self.entries[idx] = ScratchpadEntry {
            candidate,
            observed_mask: mask,
            recorded_tick: tick,
            ttl_ticks: ttl,
            active: true,
            _pad: [0u8; 3],
        };
        self.write_cursor = self.write_cursor.wrapping_add(1) % (SCRATCHPAD_CAPACITY as u8);
        if (self.active_count as usize) < SCRATCHPAD_CAPACITY {
            self.active_count += 1;
        }
    }

    #[inline(always)]
    pub fn prune_expired(&mut self, current_tick: u64) {
        let mut count = 0u8;
        for entry in self.entries.iter_mut() {
            if entry.active {
                if current_tick.saturating_sub(entry.recorded_tick) > (entry.ttl_ticks as u64) {
                    entry.active = false;
                } else {
                    count += 1;
                }
            }
        }
        self.active_count = count;
    }
}

impl Default for EpistemicScratchpad {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Matriz de paridad causal topológica (1088 bytes alineados a 64B, residente en L1).
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct CausalParityMatrix {
    pub edge_fault_signatures: [u64; MAX_SYNDROME_NODES],
    pub sensor_fault_signatures: [u64; MAX_SYNDROME_NODES],
    pub signature_count: u8,
    pub _pad: [u8; 63],
}

const _: () = assert!(std::mem::size_of::<CausalParityMatrix>() == 1088);
const _: () = assert!(std::mem::align_of::<CausalParityMatrix>() == 64);

impl CausalParityMatrix {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            edge_fault_signatures: [0u64; MAX_SYNDROME_NODES],
            sensor_fault_signatures: [0u64; MAX_SYNDROME_NODES],
            signature_count: 0,
            _pad: [0u8; 63],
        }
    }
}

impl Default for CausalParityMatrix {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

