//! # Constraint Types & UnsatCore Diagnostic
//!
//! Definición de tipos de violación, límites de aristas y núcleo de insatisfacibilidad.
//! Todas las estructuras residen en stack con memoria estática contigua (Zero-Alloc).

pub const MAX_VIOLATIONS: usize = 8;

/// Clasificación formal de las transgresiones de invariantes del grafo.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ViolationType {
    CapacityExceeded = 0,
    UnderflowBound = 1,
    RampRateExceeded = 2,
    BudgetConservationExceeded = 3,
    EdgeQuarantined = 10,
    TopologyDisconnected = 11,
    ScarInvariantViolated = 12,
    NonFiniteParameter = 13,
}

impl ViolationType {
    #[inline(always)]
    pub const fn is_repairable(&self) -> bool {
        matches!(
            self,
            ViolationType::CapacityExceeded
                | ViolationType::UnderflowBound
                | ViolationType::RampRateExceeded
                | ViolationType::BudgetConservationExceeded
        )
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            ViolationType::CapacityExceeded => "CAPACITY_EXCEEDED",
            ViolationType::UnderflowBound => "UNDERFLOW_BOUND",
            ViolationType::RampRateExceeded => "RAMP_RATE_EXCEEDED",
            ViolationType::BudgetConservationExceeded => "BUDGET_CONSERVATION_EXCEEDED",
            ViolationType::EdgeQuarantined => "EDGE_QUARANTINED",
            ViolationType::TopologyDisconnected => "TOPOLOGY_DISCONNECTED",
            ViolationType::ScarInvariantViolated => "SCAR_INVARIANT_VIOLATED",
            ViolationType::NonFiniteParameter => "NON_FINITE_PARAMETER",
        }
    }
}

/// Registro individual de transgresión de cota en una arista o presupuesto.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ConstraintViolation {
    pub op_index: u8,
    pub edge_id: u32,
    pub violation_type: ViolationType,
    pub requested_val: f64,
    pub admissible_min: f64,
    pub admissible_max: f64,
    pub repairable: bool,
}

impl ConstraintViolation {
    pub const fn empty() -> Self {
        Self {
            op_index: 0,
            edge_id: 0,
            violation_type: ViolationType::CapacityExceeded,
            requested_val: 0.0,
            admissible_min: 0.0,
            admissible_max: 0.0,
            repairable: true,
        }
    }
}

/// Certificado de Contraejemplo Mínimo (UnsatCore) libre de asignaciones dinámicas.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct UnsatCore {
    pub violations: [ConstraintViolation; MAX_VIOLATIONS],
    pub count: u8,
}

impl UnsatCore {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            violations: [ConstraintViolation::empty(); MAX_VIOLATIONS],
            count: 0,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, v: ConstraintViolation) -> bool {
        if (self.count as usize) < MAX_VIOLATIONS {
            self.violations[self.count as usize] = v;
            self.count += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.count as usize
    }

    #[inline(always)]
    pub fn all_repairable(&self) -> bool {
        let n = self.len();
        for i in 0..n {
            if !self.violations[i].repairable {
                return false;
            }
        }
        true
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[ConstraintViolation] {
        &self.violations[0..self.len()]
    }
}

impl Default for UnsatCore {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

/// Cotas físicas y lógicas operativas de una arista en el grafo causal.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EdgeOperatingBounds {
    pub min_val: f64,
    pub max_val: f64,
    pub max_ramp_delta: f64,
    pub quarantined: bool,
    pub active_scar_mask: u64,
    pub _padding: [u8; 7],
}

impl EdgeOperatingBounds {
    pub const fn new(min_val: f64, max_val: f64, max_ramp_delta: f64) -> Self {
        Self {
            min_val,
            max_val,
            max_ramp_delta,
            quarantined: false,
            active_scar_mask: 0,
            _padding: [0u8; 7],
        }
    }

    pub const fn quarantined() -> Self {
        Self {
            min_val: 0.0,
            max_val: 0.0,
            max_ramp_delta: 0.0,
            quarantined: true,
            active_scar_mask: 0,
            _padding: [0u8; 7],
        }
    }
}

