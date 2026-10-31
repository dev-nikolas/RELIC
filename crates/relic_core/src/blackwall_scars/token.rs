//! # EmpiricalFailureToken & Falsification Gate
//!
//! Token de prueba tipado para certificar que una falla física ocurrió realmente
//! en el entorno ciberfísico, emitido únicamente cuando la martingala SPRT
//! confirma la violación estadística tras la ejecución de una orden.
//!
//! Un `UnsatCore` del plano cognitivo (rechazo del LLM) carece de acceso a este token,
//! protegiendo al Blackwall contra ataques de auto-inmunidad (*Auto-Immune DoS*).

use super::signature::CriticalEdgeKey;

/// Token formal de falsación empírica.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EmpiricalFailureToken {
    pub tick: u64,
    pub edge_key: CriticalEdgeKey,
    pub log_martingale: f32,
    _private: (),
}

impl EmpiricalFailureToken {
    /// Emite un token formal de falla solo si la martingala de Wald ha disparado.
    #[inline(always)]
    pub fn issue_after_martingale_alarm(
        tick: u64,
        edge_key: CriticalEdgeKey,
        log_martingale: f32,
        threshold_h: f32,
    ) -> Result<Self, &'static str> {
        if !log_martingale.is_finite() || log_martingale < threshold_h {
            return Err("Rechazado: La martingala no confirma una anomalía física empírica");
        }
        Ok(Self {
            tick,
            edge_key,
            log_martingale,
            _private: (),
        })
    }

    /// Emisor auxiliar exclusivo para suites de pruebas y certificación formal.
    #[doc(hidden)]
    pub const fn test_token(tick: u64, edge_key: CriticalEdgeKey) -> Self {
        Self {
            tick,
            edge_key,
            log_martingale: 14.0,
            _private: (),
        }
    }
}

