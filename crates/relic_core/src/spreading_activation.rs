//! # SpreadingActivation
//!
//! Motor determinista de Activación Propagada (Spreading Activation) sobre `EpisodicMemory`.
//! Calcula energía por paso, entropía de Shannon $H = -\sum p_i \log_2 p_i$ y emite abstención
//! formal (`AMBIGUOUS`) cuando la incertidumbre supera el umbral o existe empate en el tope.
//! Menos de 180 líneas.

use std::collections::{BTreeMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::episodic_memory::EpisodicMemory;

/// Traza matemática auditable de un paso individual de propagación de energía.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivationStepTrace {
    pub step: usize,
    pub node_energies: Vec<(String, f64)>,
    pub total_energy: f64,
    pub shannon_entropy_bits: f64,
}

/// Estado epistemológico del resultado de activación propagada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActivationStatus {
    /// Existe un nodo asociado claramente dominante con entropía bajo control.
    Valid {
        top_concept: String,
        top_energy: f64,
        probability: f64,
    },
    /// Abstención honesta por empate de pesos máximos o entropía superior al umbral permitido.
    Ambiguous {
        reason: String,
        tied_candidates: Vec<String>,
        entropy_bits: f64,
    },
    /// Ningún nodo destino recibió energía desde las semillas dadas.
    Empty,
}

/// Resultado completo y auditable del motor de activación propagada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpreadingActivationResult {
    pub seeds: Vec<String>,
    pub decay_gamma: f64,
    pub steps_executed: usize,
    /// Ranking final de conceptos alcanzados (excluyendo semillas) ordenado por energía descendente.
    pub ranking: Vec<(String, f64)>,
    /// Distribución de probabilidad normalizada sobre los conceptos activados.
    pub distribution: Vec<(String, f64)>,
    /// Entropía de Shannon $H = -\sum p_i \log_2 p_i$ (en bits) de la distribución resultante.
    pub shannon_entropy_bits: f64,
    pub status: ActivationStatus,
    pub step_traces: Vec<ActivationStepTrace>,
}

/// Calcula la entropía de Shannon $H = -\sum p_i \log_2(p_i)$ en bits para un conjunto de energías no negativas.
pub fn compute_shannon_entropy(energies: &[(String, f64)]) -> (Vec<(String, f64)>, f64) {
    let total: f64 = energies.iter().map(|(_, e)| *e).filter(|e| *e > 0.0).sum();
    if total <= 1e-12 {
        return (Vec::new(), 0.0);
    }
    let mut dist = Vec::with_capacity(energies.len());
    let mut entropy = 0.0;
    for (node, e) in energies {
        if *e > 0.0 {
            let p = *e / total;
            dist.push((node.clone(), p));
            if p > 1e-15 {
                entropy -= p * p.log2();
            }
        }
    }
    (dist, entropy)
}

/// Propaga la activación desde `seeds` con factor de decaimiento `gamma` $\in (0, 1)$ durante `steps` pasos.
/// Si la entropía final supera `max_entropy_threshold` o los dos primeros conceptos empatan, devuelve `ActivationStatus::Ambiguous`.
pub fn propagate_activation(
    memory: &EpisodicMemory,
    seeds: &[&str],
    gamma: f64,
    steps: usize,
    max_entropy_threshold: f64,
) -> SpreadingActivationResult {
    let clamped_gamma = gamma.clamp(0.01, 0.999);
    let seed_set: HashSet<String> = seeds.iter().map(|s| (*s).to_string()).collect();

    let mut current_energy: BTreeMap<String, f64> = BTreeMap::new();
    let mut accumulated: BTreeMap<String, f64> = BTreeMap::new();

    for &s in seeds {
        if memory.contains_node(s) {
            current_energy.insert(s.to_string(), 1.0);
        }
    }

    let mut step_traces = Vec::with_capacity(steps);

    for step_idx in 1..=steps {
        let mut next_energy: BTreeMap<String, f64> = BTreeMap::new();
        for (u, &e_u) in &current_energy {
            if e_u <= 1e-12 {
                continue;
            }
            for (v, weight) in memory.neighbors_with_weights(u) {
                let flow = e_u * clamped_gamma * weight;
                if flow > 1e-12 {
                    *next_energy.entry(v.clone()).or_insert(0.0) += flow;
                    if !seed_set.contains(&v) {
                        *accumulated.entry(v).or_insert(0.0) += flow;
                    }
                }
            }
        }

        let mut step_vec: Vec<(String, f64)> = next_energy
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        step_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.cmp(&b.0)));

        let total_step: f64 = step_vec.iter().map(|(_, e)| *e).sum();
        let (_, step_h) = compute_shannon_entropy(&step_vec);

        step_traces.push(ActivationStepTrace {
            step: step_idx,
            node_energies: step_vec,
            total_energy: total_step,
            shannon_entropy_bits: step_h,
        });

        current_energy = next_energy;
        if current_energy.is_empty() {
            break;
        }
    }

    let mut ranking: Vec<(String, f64)> = accumulated.into_iter().collect();
    ranking.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let (distribution, shannon_entropy_bits) = compute_shannon_entropy(&ranking);

    let status = if ranking.is_empty() {
        ActivationStatus::Empty
    } else {
        let (top_node, top_e) = &ranking[0];
        let top_p = distribution.first().map_or(0.0, |(_, p)| *p);

        // Detectar empate exacto o cuasi-exacto en la cima del ranking
        let tied: Vec<String> = ranking
            .iter()
            .filter(|(_, e)| (*e - *top_e).abs() <= 1e-9)
            .map(|(n, _)| n.clone())
            .collect();

        if tied.len() > 1 {
            ActivationStatus::Ambiguous {
                reason: format!(
                    "Abstención por empate de activación máxima ({top_e:.4}) entre {tied:?}; entropía H = {shannon_entropy_bits:.4} bits"
                ),
                tied_candidates: tied,
                entropy_bits: shannon_entropy_bits,
            }
        } else if shannon_entropy_bits > max_entropy_threshold {
            let candidates: Vec<String> = ranking.iter().take(3).map(|(n, _)| n.clone()).collect();
            ActivationStatus::Ambiguous {
                reason: format!(
                    "Abstención por incertidumbre: entropía de Shannon H = {shannon_entropy_bits:.4} bits supera el umbral máximo ({max_entropy_threshold:.4} bits)"
                ),
                tied_candidates: candidates,
                entropy_bits: shannon_entropy_bits,
            }
        } else {
            ActivationStatus::Valid {
                top_concept: top_node.clone(),
                top_energy: *top_e,
                probability: top_p,
            }
        }
    };

    SpreadingActivationResult {
        seeds: seeds.iter().map(|s| (*s).to_string()).collect(),
        decay_gamma: clamped_gamma,
        steps_executed: step_traces.len(),
        ranking,
        distribution,
        shannon_entropy_bits,
        status,
        step_traces,
    }
}

