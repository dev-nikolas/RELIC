//! Índice Invertido Causal para la Suite de Regresión del Blackwall (Fase 1).
//! Garantiza completitud matemática estricta respetando los dos invariantes de F0-F7.
use std::collections::{HashMap, HashSet};
use relic_cli::{answer_intent, parse_dispatch_intent_with_lexicon, DispatchIntent};
use relic_core::certificate::SearchOutcome;
use relic_core::ReasoningEngine;
use relic_lexicon::{LexiconPayload, LexiconView};
use crate::catapult::CausalCatapult;
use crate::regression::{canonical_regression_suite, RegressionCase, RegressionFailure};

#[derive(Debug, Clone, Default)]
pub struct CausalDependencyIndex {
    pub edge_to_positive_tests: HashMap<(u32, u32), Vec<usize>>,
    pub cut_node_to_negative_tests: HashMap<u32, Vec<usize>>,
    pub query_tokens: Vec<(usize, String)>,
    pub independent_tests: HashSet<usize>,
    pub total_indexed: usize,
    pub catapult: CausalCatapult,
}

impl CausalDependencyIndex {
    pub fn build(view: &mut LexiconView, extra_cases: &[RegressionCase]) -> Self {
        let base_cases = canonical_regression_suite();
        let all_cases: Vec<&RegressionCase> = base_cases.iter().chain(extra_cases.iter()).collect();
        let total = all_cases.len();
        let max_nodes = view.episodic.interner.len();

        let mut idx = Self {
            edge_to_positive_tests: HashMap::new(),
            cut_node_to_negative_tests: HashMap::new(),
            query_tokens: Vec::with_capacity(total),
            independent_tests: HashSet::new(),
            total_indexed: total,
            catapult: CausalCatapult::new(max_nodes),
        };

        for (i, case) in all_cases.iter().enumerate() {
            idx.query_tokens.push((i, case.query.to_lowercase()));
            if case.expected_route == "deterministic_arithmetic" || case.expected_route == "epistemic_rejection" {
                idx.independent_tests.insert(i);
                continue;
            }
            if case.expected_route == "deterministic_graph" {
                let intent = parse_dispatch_intent_with_lexicon(&case.query, Some(view));
                if let DispatchIntent::GraphReachability(spec) = intent {
                    if let (Some(ref start), Some(ref target)) = (spec.initial, spec.target) {
                        let depth = spec.max_depth.unwrap_or(30);
                        let res = ReasoningEngine::solve_reachability_bfs(&mut view.episodic, start, target, depth);
                        match res.outcome {
                            SearchOutcome::Reachable { path_ids, .. } => {
                                if path_ids.len() >= 2 {
                                    for w in path_ids.windows(2) {
                                        idx.edge_to_positive_tests.entry((w[0], w[1])).or_default().push(i);
                                    }
                                }
                            }
                            SearchOutcome::Unreachable { closed_set_ids, .. } => {
                                idx.catapult.unbounded_closed_cut_mask.set_bit(i);
                                for node_id in closed_set_ids {
                                    idx.catapult.register_cut_node(node_id, i);
                                    idx.cut_node_to_negative_tests.entry(node_id).or_default().push(i);
                                }
                            }
                            _ => { idx.independent_tests.insert(i); }
                        }
                    } else { idx.independent_tests.insert(i); }
                } else { idx.independent_tests.insert(i); }
            } else { idx.independent_tests.insert(i); }
        }
        idx
    }

    /// Retorna los índices de los tests cuyo cono causal se intersecta con la mutación propuesta (con catapulta por defecto).
    pub fn find_affected_tests(&self, payload: &LexiconPayload, view: &LexiconView) -> Vec<usize> {
        self.find_affected_tests_mode(payload, view, true)
    }

    /// Retorna los índices de los tests con opción explícita de Catapulta activa o inactiva (A/B testing).
    pub fn find_affected_tests_mode(&self, payload: &LexiconPayload, view: &LexiconView, use_catapult: bool) -> Vec<usize> {
        let mut affected = HashSet::new();
        match payload {
            LexiconPayload::GraphFact { from, to, replaces, .. } => {
                if let Some((r_from, _r_rel, r_to)) = replaces {
                    if let (Some(ru), Some(rv)) = (view.episodic.interner.get_id(r_from), view.episodic.interner.get_id(r_to)) {
                        if let Some(tests) = self.edge_to_positive_tests.get(&(ru, rv)) {
                            for &t in tests { affected.insert(t); }
                        }
                    }
                }
                if let (Some(u), Some(v)) = (view.episodic.interner.get_id(from), view.episodic.interner.get_id(to)) {
                    if use_catapult {
                        let broken_cuts = self.catapult.evaluate_edge_addition(u, v);
                        for test_idx in broken_cuts.iter_indices() {
                            affected.insert(test_idx);
                        }
                    } else {
                        if let Some(tests) = self.cut_node_to_negative_tests.get(&u) {
                            for &t in tests { affected.insert(t); }
                        }
                        if let Some(tests) = self.cut_node_to_negative_tests.get(&v) {
                            for &t in tests { affected.insert(t); }
                        }
                    }
                    if let Some(tests) = self.edge_to_positive_tests.get(&(u, v)) {
                        for &t in tests { affected.insert(t); }
                    }
                }
            }
            LexiconPayload::EdgeReinforcement { .. } => {}
            LexiconPayload::Rollback { .. } | LexiconPayload::ConsolidationCheckpoint { .. } => {
                return (0..self.total_indexed).collect();
            }
            LexiconPayload::OpAlias { alias_word, .. } | LexiconPayload::QuantumPolysemousAlias { alias_word, .. } => {
                let w = alias_word.to_lowercase();
                for (idx, q) in &self.query_tokens {
                    if q.contains(&w) { affected.insert(*idx); }
                }
            }
            LexiconPayload::EntityAlias { alias, canonical, .. } => {
                let a = alias.to_lowercase();
                let c = canonical.to_lowercase();
                for (idx, q) in &self.query_tokens {
                    if q.contains(&a) || q.contains(&c) { affected.insert(*idx); }
                }
            }
            LexiconPayload::PhraseMacro { phrase, expansion, .. } => {
                let p = phrase.to_lowercase();
                let e = expansion.to_lowercase();
                for (idx, q) in &self.query_tokens {
                    if q.contains(&p) || q.contains(&e) { affected.insert(*idx); }
                }
            }
        }
        let mut res: Vec<usize> = affected.into_iter().collect();
        res.sort_unstable();
        res
    }

    pub fn run_selective(
        &self, view: &mut LexiconView, extra_cases: &[RegressionCase], test_indices: &[usize],
    ) -> Result<usize, RegressionFailure> {
        let base_cases = canonical_regression_suite();
        let all_cases: Vec<&RegressionCase> = base_cases.iter().chain(extra_cases.iter()).collect();

        for &idx in test_indices {
            if idx >= all_cases.len() { continue; }
            let case = all_cases[idx];
            let intent = parse_dispatch_intent_with_lexicon(&case.query, Some(view));
            let answer = answer_intent(&intent, &mut view.episodic);

            let text_ok = answer.text.to_lowercase().contains(&case.expected_substring.to_lowercase());
            let route_ok = answer.route == case.expected_route;
            let status_ok = answer.status == case.expected_status;

            if !text_ok || !route_ok || !status_ok {
                return Err(RegressionFailure {
                    case_id: case.id.clone(),
                    query: case.query.clone(),
                    reason: format!(
                        "Esperaba [status={}, route={}, texto~='{}'], obtuvo [status={}, route={}, texto='{}']",
                        case.expected_status, case.expected_route, case.expected_substring,
                        answer.status, answer.route, answer.text
                    ),
                });
            }
        }
        Ok(test_indices.len())
    }
}
