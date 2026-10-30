//! # relic_cli::bench_continual
//!
//! Benchmark 4 — ContinualEditStress (Estilo CounterFact / zsRE / CLUTRR).
//! Valida la retención de conocimiento, ausencia de olvido catastrófico (Locality 100%),
//! eficacia de asimilación continua (Efficacy 100%) y escalabilidad multi-salto ($k=2 \dots 10$).
use std::time::Instant;
use relic_core::ArithmeticOp;
use relic_lexicon::{EngramStore, EntryStatus, LexiconPayload};
use serde::{Deserialize, Serialize};
use relic_cli::answer::answer_intent;
use relic_cli::dispatch::parse_dispatch_intent_with_lexicon;

/// Métricas científicas de aprendizaje continuo y edición de modelos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinualLearningMetrics {
    pub total_edits: usize,
    pub successful_edits: usize,
    pub efficacy_pct: f64,
    pub locality_retention_pct: f64,
    pub clutrr_max_hops_tested: usize,
    pub clutrr_accuracy_pct: f64,
    pub rollback_fidelity_pct: f64,
    pub mean_edit_latency_us: f64,
    pub p99_edit_latency_us: f64,
}

fn compute_p99(latencies: &[f64]) -> f64 {
    if latencies.is_empty() { return 0.0; }
    let mut sorted = latencies.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() as f64 * 0.99).ceil() as usize).saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

/// Ejecuta la batería continua de estrés: 200 mutaciones secuenciales + test CLUTRR multi-salto ($k=2 \dots 10$).
pub fn run_continual_learning_benchmark() -> ContinualLearningMetrics {
    let mut store = EngramStore::with_genesis();
    let initial_version = store.current_version();
    let baseline_queries = [
        ("1 + 1", "2"),
        ("¿Es posible ir de Bogotá a Lima?", "Bogotá -> Lima"),
        ("¿Puede Marta llegar a la biblioteca pasando por la estación?", "Marta -> estación -> biblioteca"),
        ("¿Depende el crate relic_cli del paquete memmap2?", "relic_cli -> relic_core -> memmap2"),
    ];
    let num_edits = 200usize;
    let mut latencies_us = Vec::with_capacity(num_edits);
    let mut successful_edits = 0usize;

    // 1. Inyección continua de 200 hechos/alias sin olvidar los casos base
    for i in 1..=num_edits {
        let t0 = Instant::now();
        let (status, payload) = if i % 2 == 0 {
            (EntryStatus::LexicalAlias, LexiconPayload::op_alias(format!("slang_sub_{i}"), ArithmeticOp::Subtract))
        } else {
            (EntryStatus::ConfirmedFact, LexiconPayload::graph_fact(format!("Entidad_A_{i}"), "asociada_con", format!("Entidad_B_{i}")))
        };
        let res = store.append_entry(10_000 + i as u64, status, payload);
        latencies_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);
        if res.is_ok() {
            successful_edits += 1;
        }
    }

    // 2. Eficacia: ¿Aprendió el hecho más reciente?
    let intent_last = parse_dispatch_intent_with_lexicon("Tengo 50 monedas y me slang_sub_200 15", Some(store.active_view()));
    let ans_last = answer_intent(&intent_last, &mut store.active_view_mut().episodic);
    let efficacy_ok = ans_last.status == "VALID" && ans_last.text.contains("35 monedas");
    let efficacy_pct = if efficacy_ok && successful_edits == num_edits { 100.0 } else { 0.0 };

    // 3. Localidad / Cero Olvido Catastrófico: ¿Los casos iniciales siguen respondiendo 100% idéntico?
    let mut retained_count = 0usize;
    for (q, expected) in &baseline_queries {
        let intent = parse_dispatch_intent_with_lexicon(q, Some(store.active_view()));
        let ans = answer_intent(&intent, &mut store.active_view_mut().episodic);
        if ans.status == "VALID" && ans.text.contains(expected) {
            retained_count += 1;
        }
    }
    let locality_retention_pct = (retained_count as f64 / baseline_queries.len() as f64) * 100.0;

    // 4. CLUTRR: Cadena relacional multi-salto sintética de k=2 a k=10
    let mut clutrr_correct = 0usize;
    let clutrr_tests = 9usize; // k=2..10
    for k in 2..=10 {
        let chain_prefix = format!("clutrr_k{k}");
        for step in 0..k {
            let u = format!("{chain_prefix}_node_{step}");
            let v = format!("{chain_prefix}_node_{}", step + 1);
            let p = LexiconPayload::graph_fact(u, "conecta_con", v);
            let _ = store.append_entry(50_000 + (k * 100 + step) as u64, EntryStatus::ConfirmedFact, p);
        }
        let start = format!("{chain_prefix}_node_0");
        let dest = format!("{chain_prefix}_node_{k}");
        let query = format!("¿Se puede llegar a {dest} desde {start}?");
        let intent = parse_dispatch_intent_with_lexicon(&query, Some(store.active_view()));
        let ans = answer_intent(&intent, &mut store.active_view_mut().episodic);
        if ans.status == "VALID" && ans.text.contains(&start) && ans.text.contains(&dest) {
            clutrr_correct += 1;
        }
    }
    let clutrr_accuracy_pct = (clutrr_correct as f64 / clutrr_tests as f64) * 100.0;

    // 5. Rollback transaccional a la versión inicial: ¿Restaura fidelidad 100%?
    let rb_payload = LexiconPayload::Rollback { target_version: initial_version, reason: "Rollback estrés".to_string() };
    let rb_ok = store.append_entry(99_999, EntryStatus::Reverted, rb_payload).is_ok();
    let rollback_fidelity_pct = if rb_ok {
        let intent_rb = parse_dispatch_intent_with_lexicon("Tengo 50 monedas y me slang_sub_200 15", Some(store.active_view()));
        let ans_rb = answer_intent(&intent_rb, &mut store.active_view_mut().episodic);
        if ans_rb.status == "INCOMPLETE" { 100.0 } else { 0.0 }
    } else {
        0.0
    };

    let sum_lat: f64 = latencies_us.iter().sum();
    let mean_edit_latency_us = if !latencies_us.is_empty() { sum_lat / latencies_us.len() as f64 } else { 0.0 };
    let p99_edit_latency_us = compute_p99(&latencies_us);

    ContinualLearningMetrics {
        total_edits: num_edits,
        successful_edits,
        efficacy_pct,
        locality_retention_pct,
        clutrr_max_hops_tested: 10,
        clutrr_accuracy_pct,
        rollback_fidelity_pct,
        mean_edit_latency_us,
        p99_edit_latency_us,
    }
}
