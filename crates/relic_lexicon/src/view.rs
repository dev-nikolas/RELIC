//! Proyección en RAM de la vista activa del léxico y hechos episódicos.
use std::collections::HashMap;
use relic_core::{ArithmeticOp, Complex, EpisodicMemory};
use crate::log::LexiconPayload;
pub use crate::polysemy::{ClassicalPolysemyResult, PolysemousEntry};

pub const GENESIS_PREV_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone)]
pub enum InverseOp {
    RemoveFact { from: String, to: String },
    RestoreFact { from: String, to: String, prior: (String, f64, u64) },
    RestoreEdgeStats { from: String, to: String, old_w: f64, old_u: u64 },
    RemoveOpAlias { word: String },
    RestoreOpAlias { word: String, old_op: ArithmeticOp, old_w: (f64, u64) },
    RestoreQuantumPolysemy { word: String, old: Option<Vec<PolysemousEntry>> },
    RestorePhraseMacros { old: Vec<(String, String)> },
    RestoreEntityAlias { alias: String, canonical: String, old: Option<String> },
    Composite(Vec<InverseOp>),
    NoOp,
}

#[derive(Debug, Clone, Default)]
pub struct LexiconView {
    pub episodic: EpisodicMemory,
    pub op_aliases: HashMap<String, ArithmeticOp>,
    pub op_alias_weights: HashMap<String, (f64, u64)>,
    pub quantum_polysemy: HashMap<String, Vec<PolysemousEntry>>,
    pub phrase_macros: Vec<(String, String)>,
    pub entity_aliases: HashMap<String, String>,
    pub active_facts: HashMap<(String, String), (String, f64, u64)>,
    pub current_version: u64,
    pub head_hash: String,
}

impl LexiconView {
    pub fn resolve_op_alias(&self, word: &str) -> Option<ArithmeticOp> { self.op_aliases.get(&word.trim().to_lowercase()).copied() }

    pub fn apply_phrase_macros(&self, text: &str) -> String {
        let mut out = text.to_lowercase();
        for (p, e) in &self.phrase_macros { if out.contains(p.as_str()) { out = out.replace(p.as_str(), e.as_str()); } }
        out
    }

    pub fn apply_payload(&mut self, payload: &LexiconPayload) {
        let _ = self.apply_payload_recording_undo(payload);
    }

    pub fn apply_payload_recording_undo(&mut self, payload: &LexiconPayload) -> InverseOp {
        match payload {
            LexiconPayload::GraphFact { from, relation, to, weight, usage_count, replaces } => {
                let undo_replace = if let Some((r_from, _r_rel, r_to)) = replaces {
                    let old_fact = self.active_facts.remove(&(r_from.clone(), r_to.clone()));
                    self.episodic.remove_edge(r_from, r_to);
                    old_fact.map(|f| InverseOp::RestoreFact { from: r_from.clone(), to: r_to.clone(), prior: f })
                } else { None };
                let prior = self.active_facts.insert((from.clone(), to.clone()), (relation.clone(), *weight, *usage_count));
                self.episodic.insert_edge_with_weight(from.clone(), to.clone(), *weight);
                self.episodic.set_edge_stats(from, to, *weight, *usage_count);
                let undo_insert = match prior {
                    Some(old) => InverseOp::RestoreFact { from: from.clone(), to: to.clone(), prior: old },
                    None => InverseOp::RemoveFact { from: from.clone(), to: to.clone() },
                };
                if let Some(ur) = undo_replace { InverseOp::Composite(vec![undo_insert, ur]) } else { undo_insert }
            }
            LexiconPayload::EdgeReinforcement { from, to, new_weight, usage_count } => {
                let prior = self.active_facts.get_mut(&(from.clone(), to.clone())).map(|f| {
                    let old = (f.1, f.2);
                    f.1 = *new_weight; f.2 = *usage_count;
                    old
                });
                self.episodic.set_edge_stats(from, to, *new_weight, *usage_count);
                prior.map_or(InverseOp::NoOp, |(w, u)| InverseOp::RestoreEdgeStats { from: from.clone(), to: to.clone(), old_w: w, old_u: u })
            }
            LexiconPayload::OpAlias { alias_word, target_op, weight, usage_count } => {
                let k = alias_word.trim().to_lowercase();
                let old_op = self.op_aliases.insert(k.clone(), *target_op);
                let old_w = self.op_alias_weights.insert(k.clone(), (*weight, *usage_count));
                match old_op {
                    Some(op) => InverseOp::RestoreOpAlias { word: k, old_op: op, old_w: old_w.unwrap_or((*weight, *usage_count)) },
                    None => InverseOp::RemoveOpAlias { word: k },
                }
            }
            LexiconPayload::QuantumPolysemousAlias { alias_word, target_op, amplitude_r, phase_theta, context_word } => {
                let (k, amp) = (alias_word.trim().to_lowercase(), Complex::from_polar(*amplitude_r, *phase_theta));
                let old = self.quantum_polysemy.get(&k).cloned();
                let ctx = context_word.as_ref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
                let list = self.quantum_polysemy.entry(k.clone()).or_default();
                if let Some(ex) = list.iter_mut().find(|e| e.target_op == *target_op && e.context_word == ctx) { ex.amplitude = amp; }
                else { list.push(PolysemousEntry { target_op: *target_op, amplitude: amp, context_word: ctx }); }
                InverseOp::RestoreQuantumPolysemy { word: k, old }
            }
            LexiconPayload::PhraseMacro { phrase, expansion, .. } => {
                let (p, e) = (phrase.trim().to_lowercase(), expansion.trim().to_lowercase());
                let old = self.phrase_macros.clone();
                self.phrase_macros.retain(|(existing, _)| existing != &p);
                self.phrase_macros.push((p, e));
                self.phrase_macros.sort_by_key(|a| std::cmp::Reverse(a.0.len()));
                InverseOp::RestorePhraseMacros { old }
            }
            LexiconPayload::EntityAlias { alias, canonical, .. } => {
                let k = alias.trim().to_lowercase();
                let old = self.entity_aliases.insert(k, canonical.clone());
                self.episodic.register_entity_alias(alias.clone(), canonical.clone());
                InverseOp::RestoreEntityAlias { alias: alias.clone(), canonical: canonical.clone(), old }
            }
            LexiconPayload::ConsolidationCheckpoint { pruned_edges, .. } => {
                for edge in pruned_edges { self.active_facts.remove(edge); self.episodic.remove_edge(&edge.0, &edge.1); }
                InverseOp::NoOp
            }
            LexiconPayload::Rollback { .. } => InverseOp::NoOp,
        }
    }

    pub fn apply_inverse_op(&mut self, op: InverseOp) {
        match op {
            InverseOp::RemoveFact { from, to } => {
                self.active_facts.remove(&(from.clone(), to.clone()));
                self.episodic.remove_edge(&from, &to);
            }
            InverseOp::RestoreFact { from, to, prior } => {
                self.active_facts.insert((from.clone(), to.clone()), prior.clone());
                self.episodic.insert_edge_with_weight(&from, &to, prior.1);
                self.episodic.set_edge_stats(&from, &to, prior.1, prior.2);
            }
            InverseOp::RestoreEdgeStats { from, to, old_w, old_u } => {
                if let Some(f) = self.active_facts.get_mut(&(from.clone(), to.clone())) { f.1 = old_w; f.2 = old_u; }
                self.episodic.set_edge_stats(&from, &to, old_w, old_u);
            }
            InverseOp::RemoveOpAlias { word } => { self.op_aliases.remove(&word); self.op_alias_weights.remove(&word); }
            InverseOp::RestoreOpAlias { word, old_op, old_w } => {
                self.op_aliases.insert(word.clone(), old_op);
                self.op_alias_weights.insert(word, old_w);
            }
            InverseOp::RestoreQuantumPolysemy { word, old } => match old {
                Some(list) => { self.quantum_polysemy.insert(word, list); }
                None => { self.quantum_polysemy.remove(&word); }
            },
            InverseOp::RestorePhraseMacros { old } => { self.phrase_macros = old; }
            InverseOp::RestoreEntityAlias { alias, canonical, old } => match old {
                Some(can) => {
                    self.entity_aliases.insert(alias.trim().to_lowercase(), can.clone());
                    self.episodic.unregister_entity_alias(&alias, &canonical);
                    self.episodic.register_entity_alias(alias, can);
                }
                None => {
                    self.entity_aliases.remove(&alias.trim().to_lowercase());
                    self.episodic.unregister_entity_alias(&alias, &canonical);
                }
            },
            InverseOp::Composite(ops) => {
                for sub_op in ops { self.apply_inverse_op(sub_op); }
            }
            InverseOp::NoOp => {}
        }
    }
}
