//! # relic_core::kuramoto
//!
//! Motor determinista de osciladores acoplados de Kuramoto en grafos restringidos.
//! Resuelve activación asociativa y desambiguación contextual por resonancia de fase
//! ($\frac{d\theta_i}{dt} = \omega_i + \sum_j K_{ij} \sin(\theta_j - \theta_i)$)
//! con integración Runge-Kutta de 4to orden (RK4), parámetro de orden complejo
//! ($r e^{i\psi} = \frac{1}{|C|} \sum_{j \in C} e^{i\theta_j}$) y abstención física
//! cuando $r(t_{\text{final}}) < r^*$. Menos de 180 líneas.

use std::f64::consts::PI;
use serde::{Deserialize, Serialize};
use crate::quantum::Complex;
use crate::spreading_activation::compute_shannon_entropy;

/// Especificación de un oscilador acoplado de Kuramoto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KuramotoOscillator {
    pub id: String,
    pub initial_theta: f64,
    pub natural_omega: f64,
}

impl KuramotoOscillator {
    pub fn new(id: impl Into<String>, initial_theta: f64, natural_omega: f64) -> Self {
        Self { id: id.into(), initial_theta: wrap_angle(initial_theta), natural_omega }
    }
}

/// Especificación del sistema dinámico de Kuramoto sobre un grafo con acoplamientos con signo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KuramotoNetworkSpec {
    pub oscillators: Vec<KuramotoOscillator>,
    /// Matriz de acoplamiento $K_{ij}$ ($N \times N$): $K_{ij} > 0$ atrae a $0$, $K_{ij} < 0$ repele a $\pi$.
    pub coupling_matrix: Vec<Vec<f64>>,
    pub steps: usize,
    pub dt: f64,
    pub required_r: f64,
    pub alignment_cos_threshold: f64,
}

/// Traza completa de evolución temporal del sistema de Kuramoto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KuramotoTrace {
    pub initial_r: f64,
    pub final_r: f64,
    pub mean_phase_psi: f64,
    pub r_trajectory: Vec<f64>,
    pub final_phases: Vec<(String, f64)>,
    pub synchronized_cluster: Vec<String>,
    pub phase_entropy_bits: f64,
}

/// Resultado verificado de la dinámica de sincronización de Kuramoto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SynchronizationOutcome {
    Synchronized {
        order_parameter_r: f64,
        required_r: f64,
        mean_phase_psi: f64,
        synchronized_cluster: Vec<String>,
        trace: KuramotoTrace,
    },
    IncoherentAbstention {
        order_parameter_r: f64,
        required_r: f64,
        entropy: f64,
        reason: String,
        trace: KuramotoTrace,
    },
}

#[inline]
pub fn wrap_angle(theta: f64) -> f64 {
    let mut a = (theta + PI) % (2.0 * PI);
    if a < 0.0 { a += 2.0 * PI; }
    a - PI
}

fn derivatives(thetas: &[f64], omegas: &[f64], k_mat: &[Vec<f64>]) -> Vec<f64> {
    let n = thetas.len();
    let mut dtheta = vec![0.0; n];
    for i in 0..n {
        let (mut sum_coupling, row, ti) = (0.0, &k_mat[i], thetas[i]);
        for (j, &tj) in thetas.iter().enumerate() {
            if i != j {
                if let Some(&k_ij) = row.get(j) { sum_coupling += k_ij * (tj - ti).sin(); }
            }
        }
        dtheta[i] = omegas[i] + sum_coupling;
    }
    dtheta
}

/// Calcula el parámetro de orden complejo $r e^{i\psi}$ sobre un subconjunto de índices (o todos si `subset` es `None`).
pub fn compute_kuramoto_order_parameter(thetas: &[f64], subset: Option<&[usize]>) -> (f64, f64) {
    if thetas.is_empty() { return (0.0, 0.0); }
    let mut z = Complex::ZERO;
    let count = if let Some(indices) = subset {
        if indices.is_empty() { return (0.0, 0.0); }
        for &idx in indices { if let Some(&th) = thetas.get(idx) { z += Complex::from_polar(1.0, th); } }
        indices.len()
    } else {
        for &th in thetas { z += Complex::from_polar(1.0, th); }
        thetas.len()
    };
    let mean = z.scale(1.0 / count as f64);
    (mean.norm().clamp(0.0, 1.0), mean.phase())
}

/// Ejecuta la integración determinista RK4 del sistema de Kuramoto y evalúa la compuerta de abstención física.
pub fn simulate_kuramoto_synchronization(spec: &KuramotoNetworkSpec) -> SynchronizationOutcome {
    let n = spec.oscillators.len();
    if n == 0 {
        let trace = KuramotoTrace {
            initial_r: 0.0, final_r: 0.0, mean_phase_psi: 0.0, r_trajectory: Vec::new(),
            final_phases: Vec::new(), synchronized_cluster: Vec::new(), phase_entropy_bits: 0.0,
        };
        return SynchronizationOutcome::IncoherentAbstention {
            order_parameter_r: 0.0, required_r: spec.required_r, entropy: 0.0,
            reason: "Sistema de osciladores vacío.".to_string(), trace,
        };
    }

    let mut thetas: Vec<f64> = spec.oscillators.iter().map(|o| wrap_angle(o.initial_theta)).collect();
    let omegas: Vec<f64> = spec.oscillators.iter().map(|o| o.natural_omega).collect();
    let (dt, steps) = (spec.dt.max(1e-5), spec.steps.max(1));
    let mut r_trajectory = Vec::with_capacity(steps + 1);
    let (r0, _) = compute_kuramoto_order_parameter(&thetas, None);
    r_trajectory.push(r0);

    for _ in 0..steps {
        let k1 = derivatives(&thetas, &omegas, &spec.coupling_matrix);
        let t2: Vec<f64> = (0..n).map(|i| thetas[i] + 0.5 * dt * k1[i]).collect();
        let k2 = derivatives(&t2, &omegas, &spec.coupling_matrix);
        let t3: Vec<f64> = (0..n).map(|i| thetas[i] + 0.5 * dt * k2[i]).collect();
        let k3 = derivatives(&t3, &omegas, &spec.coupling_matrix);
        let t4: Vec<f64> = (0..n).map(|i| thetas[i] + dt * k3[i]).collect();
        let k4 = derivatives(&t4, &omegas, &spec.coupling_matrix);
        for i in 0..n {
            thetas[i] = wrap_angle(thetas[i] + (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]));
        }
        r_trajectory.push(compute_kuramoto_order_parameter(&thetas, None).0);
    }

    let (global_r, global_psi) = compute_kuramoto_order_parameter(&thetas, None);
    let cos_thresh = spec.alignment_cos_threshold.clamp(-1.0, 1.0);
    let synchronized_cluster: Vec<String> = (0..n)
        .filter(|&i| (thetas[i] - global_psi).cos() >= cos_thresh)
        .map(|i| spec.oscillators[i].id.clone())
        .collect();
    let alignment_weights: Vec<(String, f64)> = (0..n)
        .map(|i| (spec.oscillators[i].id.clone(), (0.5 * (1.0 + (thetas[i] - global_psi).cos())).max(1e-9)))
        .collect();
    let (_, entropy) = compute_shannon_entropy(&alignment_weights);
    let final_phases = (0..n).map(|i| (spec.oscillators[i].id.clone(), thetas[i])).collect();
    let trace = KuramotoTrace {
        initial_r: r0, final_r: global_r, mean_phase_psi: global_psi, r_trajectory,
        final_phases, synchronized_cluster: synchronized_cluster.clone(), phase_entropy_bits: entropy,
    };

    if global_r < spec.required_r || synchronized_cluster.is_empty() {
        SynchronizationOutcome::IncoherentAbstention {
            order_parameter_r: global_r, required_r: spec.required_r, entropy,
            reason: format!("Abstención física por desorden de fase / quimera: r ({global_r:.4}) < r* ({:.4}), H = {entropy:.4} bits.", spec.required_r),
            trace,
        }
    } else {
        SynchronizationOutcome::Synchronized {
            order_parameter_r: global_r, required_r: spec.required_r, mean_phase_psi: global_psi, synchronized_cluster, trace,
        }
    }
}

