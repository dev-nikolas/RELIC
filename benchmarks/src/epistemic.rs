//! Benchmark 2 — Razonamiento Estándar (GSM8K-ES / SVAMP / bAbI) y Trampas Epistémicas.
//! Menos de 180 líneas.

use std::time::Instant;
use relic_core::{propagate_activation, ActivationStatus, ArithmeticOp};
use relic_lexicon::{EngramStore, EntryStatus, LexiconPayload};
use serde::{Deserialize, Serialize};
use relic_cli::answer::answer_intent;
use crate::compute_mean_and_p99;
use relic_cli::dispatch::{parse_dispatch_intent_with_lexicon, DispatchIntent};

/// Métricas del Benchmark 2 (GSM8K-ES / SVAMP / bAbI + Trampas Epistémicas).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningEpistemicMetrics {
    pub total_cases: usize,
    pub answerable_cases: usize,
    pub unanswerable_trap_cases: usize,
    pub selective_accuracy_pct: f64,
    pub coverage_on_answerable_pct: f64,
    pub hallucination_rate_pct: f64,
    pub exact_trace_match_pct: f64,
    pub mean_latency_us: f64,
    pub p99_latency_us: f64,
}

enum Bench2Case {
    Answerable {
        query: &'static str,
        expected_substring: &'static str,
        expected_trace_substring: &'static str,
    },
    UnanswerableTrap { query: &'static str },
    SymmetricEntropyTrap { seed: &'static str },
}

/// Ejecuta el Benchmark 2: Razonamiento Estándar y Trampas Epistémicas (32 casos).
pub fn run_bench2_reasoning_and_epistemic() -> ReasoningEpistemicMetrics {
    let mut store = EngramStore::with_genesis();
    store.append_entry(20_001, EntryStatus::LexicalAlias, LexiconPayload::op_alias("embolatan", ArithmeticOp::Subtract)).unwrap();
    store.append_entry(20_002, EntryStatus::LexicalAlias, LexiconPayload::phrase_macro("media docena de", "6")).unwrap();
    store.append_entry(20_003, EntryStatus::ConfirmedFact, LexiconPayload::graph_fact_weighted("Nodo_Simetrico", "conecta", "Rama_Norte", 1.0)).unwrap();
    store.append_entry(20_004, EntryStatus::ConfirmedFact, LexiconPayload::graph_fact_weighted("Nodo_Simetrico", "conecta", "Rama_Sur", 1.0)).unwrap();

    let cases: Vec<Bench2Case> = vec![
        Bench2Case::Answerable { query: "la mitad de 50", expected_substring: "25", expected_trace_substring: "50 / 2 = 25" },
        Bench2Case::Answerable { query: "el doble de (10 + 5)", expected_substring: "30", expected_trace_substring: "15 * 2 = 30" },
        Bench2Case::Answerable { query: "el triple de 12", expected_substring: "36", expected_trace_substring: "12 * 3 = 36" },
        Bench2Case::Answerable { query: "un tercio de 90", expected_substring: "30", expected_trace_substring: "90 / 3 = 30" },
        Bench2Case::Answerable { query: "un cuarto de 80", expected_substring: "20", expected_trace_substring: "80 / 4 = 20" },
        Bench2Case::Answerable { query: "el 20% de 150", expected_substring: "30", expected_trace_substring: "0.2 * 150 = 30" },
        Bench2Case::Answerable { query: "200 más el 15%", expected_substring: "230", expected_trace_substring: "200 + 30 = 230" },
        Bench2Case::Answerable { query: "500 menos el 10%", expected_substring: "450", expected_trace_substring: "500 - 50 = 450" },
        Bench2Case::Answerable { query: "repartir 100 entre 4", expected_substring: "25", expected_trace_substring: "reparto exacto: cociente 25, resto 0" },
        Bench2Case::Answerable { query: "repartir 10 entre 3", expected_substring: "3.33", expected_trace_substring: "reparto inexacto: cociente entero 3, resto 1" },
        Bench2Case::Answerable { query: "Tengo 40 monedas, compro media docena de monedas y luego me embolatan 16", expected_substring: "30 monedas", expected_trace_substring: "40 monedas + 6 monedas - 16 monedas" },
        Bench2Case::Answerable { query: "Tengo 100 cajas y regalo el 25% de 80", expected_substring: "80", expected_trace_substring: "100 - 20 = 80" },
        Bench2Case::Answerable { query: "¿Se puede llegar a París desde Madrid?", expected_substring: "Sí, es posible alcanzar París desde Madrid", expected_trace_substring: "Madrid -> Barcelona -> París" },
        Bench2Case::Answerable { query: "¿Se puede llegar a París desde Madrid pasando por Barcelona?", expected_substring: "Sí, es posible alcanzar París desde Madrid pasando por Barcelona", expected_trace_substring: "Madrid -> Barcelona -> París" },
        Bench2Case::Answerable { query: "¿Se puede llegar a París desde Madrid evitando Barcelona?", expected_substring: "No, no existe ninguna ruta viable desde Madrid hasta París", expected_trace_substring: "No, no existe ninguna ruta viable" },
        Bench2Case::Answerable { query: "¿Puede Marta llegar a la biblioteca?", expected_substring: "Sí, es posible alcanzar biblioteca desde Marta", expected_trace_substring: "Marta -> estación -> biblioteca" },
        Bench2Case::Answerable { query: "¿Puede Marta llegar a la biblioteca evitando estación?", expected_substring: "No, no existe ninguna ruta viable desde Marta hasta biblioteca", expected_trace_substring: "No, no existe ninguna ruta viable" },
        Bench2Case::Answerable { query: "¿Se puede llegar a Madrid desde París?", expected_substring: "No, no existe ninguna ruta viable desde París hasta Madrid", expected_trace_substring: "No, no existe ninguna ruta viable" },
        Bench2Case::Answerable { query: "¿Depende el crate relic_cli del paquete memmap2?", expected_substring: "Sí, es posible alcanzar memmap2 desde relic_cli", expected_trace_substring: "relic_cli -> relic_core -> memmap2" },
        Bench2Case::Answerable { query: "¿Depende el crate relic_core del paquete relic_cli?", expected_substring: "No, no existe ninguna ruta viable desde relic_core hasta relic_cli", expected_trace_substring: "No, no existe ninguna ruta viable" },
        Bench2Case::Answerable { query: "Ana es más alta que Bruno y Bruno es más alto que Carlos, ¿quién es el más alto?", expected_substring: "El más alto es Ana", expected_trace_substring: "El más alto es Ana" },
        Bench2Case::Answerable { query: "David es más bajo que Elena y Elena es más baja que Sofía, ¿quién es el más alto?", expected_substring: "El más alto es Sofía", expected_trace_substring: "El más alto es Sofía" },
        Bench2Case::UnanswerableTrap { query: "Tengo algunas manzanas y compro 5" },
        Bench2Case::UnanswerableTrap { query: "100 / 0" },
        Bench2Case::UnanswerableTrap { query: "25 + ?" },
        Bench2Case::UnanswerableTrap { query: "¿Se puede llegar a Atlántida desde Madrid?" },
        Bench2Case::UnanswerableTrap { query: "¿Se puede llegar a Atenas desde Narnia?" },
        Bench2Case::UnanswerableTrap { query: "Ana es más alta que Bruno y Bruno es más alto que Ana. ¿Quién es el más alto?" },
        Bench2Case::UnanswerableTrap { query: "Ana es más alta que Bruno y Carlos es más alto que David. ¿Quién es el más alto?" },
        Bench2Case::UnanswerableTrap { query: "Ana supera a Bruno y Bruno supera a Carlos. ¿Quién es el más alto?" },
        Bench2Case::UnanswerableTrap { query: "¿Cuál es el sentido metafísico del universo?" },
        Bench2Case::SymmetricEntropyTrap { seed: "Nodo_Simetrico" },
    ];

    let total_cases = cases.len();
    let (mut answerable_cases, mut unanswerable_trap_cases, mut answered_on_answerable) = (0usize, 0usize, 0usize);
    let (mut correct_on_answered, mut exact_trace_matches, mut hallucinations_on_traps) = (0usize, 0usize, 0usize);
    let mut latencies_us = Vec::with_capacity(total_cases);

    for case in &cases {
        match case {
            Bench2Case::Answerable { query, expected_substring, expected_trace_substring } => {
                answerable_cases += 1;
                let t0 = Instant::now();
                let intent = parse_dispatch_intent_with_lexicon(query, Some(store.active_view()));
                let ans = answer_intent(&intent, &mut store.active_view_mut().episodic);
                latencies_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);

                if ans.status == "VALID" {
                    answered_on_answerable += 1;
                    if ans.text.contains(expected_substring) { correct_on_answered += 1; }
                    let trace_ok = match &intent {
                        DispatchIntent::Arithmetic(plan) => {
                            let formula = plan.format_formula();
                            let note = plan.exact_division_note.as_deref().unwrap_or("");
                            formula.contains(expected_trace_substring)
                                || ans.text.contains(expected_trace_substring)
                                || note.contains(expected_trace_substring)
                        }
                        _ => ans.text.contains(expected_trace_substring),
                    };
                    if trace_ok { exact_trace_matches += 1; }
                }
            }
            Bench2Case::UnanswerableTrap { query } => {
                unanswerable_trap_cases += 1;
                let t0 = Instant::now();
                let intent = parse_dispatch_intent_with_lexicon(query, Some(store.active_view()));
                let ans = answer_intent(&intent, &mut store.active_view_mut().episodic);
                latencies_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);
                if ans.status == "VALID" { hallucinations_on_traps += 1; }
            }
            Bench2Case::SymmetricEntropyTrap { seed } => {
                unanswerable_trap_cases += 1;
                let t0 = Instant::now();
                let act = propagate_activation(&store.active_view().episodic, &[seed], 0.8, 1, 0.5);
                latencies_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);
                if matches!(act.status, ActivationStatus::Valid { .. }) { hallucinations_on_traps += 1; }
            }
        }
    }

    let selective_accuracy_pct = if answered_on_answerable > 0 { (correct_on_answered as f64 / answered_on_answerable as f64) * 100.0 } else { 0.0 };
    let coverage_on_answerable_pct = (answered_on_answerable as f64 / answerable_cases as f64) * 100.0;
    let hallucination_rate_pct = (hallucinations_on_traps as f64 / unanswerable_trap_cases as f64) * 100.0;
    let exact_trace_match_pct = if answered_on_answerable > 0 { (exact_trace_matches as f64 / answered_on_answerable as f64) * 100.0 } else { 0.0 };
    let (mean_latency_us, p99_latency_us) = compute_mean_and_p99(&latencies_us);

    ReasoningEpistemicMetrics {
        total_cases, answerable_cases, unanswerable_trap_cases, selective_accuracy_pct,
        coverage_on_answerable_pct, hallucination_rate_pct, exact_trace_match_pct, mean_latency_us, p99_latency_us,
    }
}

