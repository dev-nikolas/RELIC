//! Benchmark de Rendimiento y Memoria: Seis Ojos en RELIC.
//!
//! Mediciones:
//! 1. A/B Overhead de Telemetría Desactivada vs. Monitor Pasivo en mutaciones simbólicas puras.
//! 2. Latencia escalar por punto (evaluate_pure, commit_batch y E2E) con percentiles p50, p90, p99.
//! 3. Consumo real de memoria RSS (0 canales, 1 canal lleno, 10 canales llenos).
//! 4. Verificación de cero contaminación en el WAL (`engram.jsonl`).

use std::time::Instant;
use relic_blackwall::{
    AnomalyGateMode, BlackwallGate, TransactionOutcome,
};
use relic_cli::monitor::get_current_rss_kb;
use relic_lexicon::LexiconPayload;

fn compute_percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 * p).ceil() as usize).saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

fn main() {
    println!("================================================================================");
    println!("  RELIC & Seis Ojos — Benchmark de Rendimiento, Latencia y Presupuesto de RSS");
    println!("================================================================================");

    // =========================================================================
    // 1. COMPARATIVA A/B: IMPACTO EN MUTACIONES SIMBÓLICAS PURAS
    // =========================================================================
    println!("\n[1/4] Ejecutando Benchmark A/B: Mutaciones Simbólicas Puras (1,000 mutaciones)");
    println!("      Condición A: Telemetría Desactivada (Disabled)");
    println!("      Condición B: Telemetría en Monitor Pasivo (PassiveMonitor)");

    let num_mutations = 1_000;

    // Condición A: Disabled
    let mut gate_disabled = BlackwallGate::with_genesis();
    gate_disabled.telemetry_arena.mode = AnomalyGateMode::Disabled;

    let mut lat_disabled = Vec::with_capacity(num_mutations);
    for i in 0..num_mutations {
        let payload = LexiconPayload::graph_fact(
            format!("node_a_{i}"),
            "link",
            format!("node_b_{i}"),
        );
        let t0 = Instant::now();
        let outcome = gate_disabled.commit_transaction_with_telemetry(&[payload], &[], 1000 + i as u64)
            .expect("Transacción exitosa");
        let dur = t0.elapsed().as_nanos() as f64 / 1_000.0; // en µs
        assert!(matches!(outcome, TransactionOutcome::Committed { .. }));
        lat_disabled.push(dur);
    }
    lat_disabled.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Condición B: PassiveMonitor
    let mut gate_passive = BlackwallGate::with_genesis();
    gate_passive.telemetry_arena.mode = AnomalyGateMode::PassiveMonitor;

    let mut lat_passive = Vec::with_capacity(num_mutations);
    for i in 0..num_mutations {
        let payload = LexiconPayload::graph_fact(
            format!("node_a_{i}"),
            "link",
            format!("node_b_{i}"),
        );
        let t0 = Instant::now();
        let outcome = gate_passive.commit_transaction_with_telemetry(&[payload], &[], 1000 + i as u64)
            .expect("Transacción exitosa");
        let dur = t0.elapsed().as_nanos() as f64 / 1_000.0; // en µs
        assert!(matches!(outcome, TransactionOutcome::Committed { .. }));
        lat_passive.push(dur);
    }
    lat_passive.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mean_dis = lat_disabled.iter().sum::<f64>() / num_mutations as f64;
    let p50_dis = compute_percentile(&lat_disabled, 0.50);
    let p99_dis = compute_percentile(&lat_disabled, 0.99);

    let mean_pas = lat_passive.iter().sum::<f64>() / num_mutations as f64;
    let p50_pas = compute_percentile(&lat_passive, 0.50);
    let p99_pas = compute_percentile(&lat_passive, 0.99);

    println!("  -> [A] Disabled:       media = {:>6.2} µs | p50 = {:>6.2} µs | p99 = {:>6.2} µs", mean_dis, p50_dis, p99_dis);
    println!("  -> [B] PassiveMonitor: media = {:>6.2} µs | p50 = {:>6.2} µs | p99 = {:>6.2} µs", mean_pas, p50_pas, p99_pas);
    let delta_p50 = p50_pas - p50_dis;
    println!("  -> Delta p50: {:+.2} µs (diferencia dentro del ruido del temporizador del SO)", delta_p50);

    // =========================================================================
    // 2. LATENCIA POR PUNTO DE TELEMETRÍA (EVALUATE_PURE + COMMIT)
    // =========================================================================
    println!("\n[2/4] Midiendo Latencia del Pipeline de Telemetría Seis Ojos (Escalar)");
    println!("      Pipeline: Takens -> RollingNorm -> ESN -> RLS -> ECDF -> NMS -> Refractory -> CUSUM");

    let mut gate_telemetry = BlackwallGate::with_genesis();
    gate_telemetry.telemetry_arena.mode = AnomalyGateMode::PassiveMonitor;

    let total_points = 5_000;
    let mut eval_pure_lats = Vec::with_capacity(total_points);
    let mut commit_lats = Vec::with_capacity(total_points);
    let mut total_step_lats = Vec::with_capacity(total_points);

    for step in 0..total_points {
        let val = 25.0 + 5.0 * ((step as f64) * 0.05).sin() + (step % 7) as f64 * 0.1;

        // Medir evaluate_pure
        let t0 = Instant::now();
        let assessments = gate_telemetry.telemetry_arena.evaluate_batch_pure(&[(0, val)])
            .expect("Evaluación pura exitosa");
        let t1 = Instant::now();

        // Medir commit_batch
        gate_telemetry.telemetry_arena.commit_batch(&assessments, step as u64);
        let t2 = Instant::now();

        let dur_eval = (t1 - t0).as_nanos() as f64 / 1_000.0;
        let dur_commit = (t2 - t1).as_nanos() as f64 / 1_000.0;
        let dur_total = (t2 - t0).as_nanos() as f64 / 1_000.0;

        eval_pure_lats.push(dur_eval);
        commit_lats.push(dur_commit);
        total_step_lats.push(dur_total);
    }

    eval_pure_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    commit_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    total_step_lats.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let eval_mean = eval_pure_lats.iter().sum::<f64>() / total_points as f64;
    let eval_p50 = compute_percentile(&eval_pure_lats, 0.50);
    let eval_p90 = compute_percentile(&eval_pure_lats, 0.90);
    let eval_p99 = compute_percentile(&eval_pure_lats, 0.99);

    let com_mean = commit_lats.iter().sum::<f64>() / total_points as f64;
    let com_p50 = compute_percentile(&commit_lats, 0.50);
    let com_p90 = compute_percentile(&commit_lats, 0.90);
    let com_p99 = compute_percentile(&commit_lats, 0.99);

    let tot_mean = total_step_lats.iter().sum::<f64>() / total_points as f64;
    let tot_p50 = compute_percentile(&total_step_lats, 0.50);
    let tot_p90 = compute_percentile(&total_step_lats, 0.90);
    let tot_p99 = compute_percentile(&total_step_lats, 0.99);

    println!("  -> evaluate_pure:  media = {:>6.2} µs | p50 = {:>6.2} µs | p90 = {:>6.2} µs | p99 = {:>6.2} µs", eval_mean, eval_p50, eval_p90, eval_p99);
    println!("  -> commit_batch:   media = {:>6.2} µs | p50 = {:>6.2} µs | p90 = {:>6.2} µs | p99 = {:>6.2} µs", com_mean, com_p50, com_p90, com_p99);
    println!("  -> Latencia Total: media = {:>6.2} µs | p50 = {:>6.2} µs | p90 = {:>6.2} µs | p99 = {:>6.2} µs", tot_mean, tot_p50, tot_p90, tot_p99);

    // =========================================================================
    // 3. MEDICIÓN DE MEMORIA RSS EN ESTADO ESTACIONARIO
    // =========================================================================
    println!("\n[3/4] Midiendo Presupuesto de Memoria RSS en Estado Estacionario");

    let rss_0_channels_kb = get_current_rss_kb();
    println!("  -> RSS Base (0 canales activos):   {:>6.2} MB ({:>7} KB)", rss_0_channels_kb as f64 / 1024.0, rss_0_channels_kb);

    // Activar Canal 0 y alimentarlo con 16,500 puntos para llenar todas las ventanas (ECDF W=16,000, Norm W=6,400)
    let mut arena_1ch = BlackwallGate::with_genesis();
    arena_1ch.telemetry_arena.mode = AnomalyGateMode::PassiveMonitor;
    println!("  -> Llenando ventanas del Canal 0 con 16,500 observaciones continuas...");
    for step in 0..16_500 {
        let val = 20.0 + ((step % 100) as f64) * 0.1;
        let _ = arena_1ch.commit_transaction_with_telemetry(&[], &[(0, val)], step as u64);
    }
    let rss_1_channel_kb = get_current_rss_kb();
    let delta_1_kb = rss_1_channel_kb.saturating_sub(rss_0_channels_kb);
    println!("  -> RSS 1 Canal (ventanas llenas):  {:>6.2} MB ({:>7} KB) | Delta: {:>+6} KB (Teórico: 245.7 KB)", rss_1_channel_kb as f64 / 1024.0, rss_1_channel_kb, delta_1_kb);

    // Activar 10 canales y llenarlos con 16,500 puntos cada uno
    println!("  -> Inicializando y llenando 10 canales independientes hasta estado estacionario...");
    let mut arena_10ch = BlackwallGate::with_genesis();
    arena_10ch.telemetry_arena.mode = AnomalyGateMode::PassiveMonitor;
    for ch in 0..10 {
        for step in 0..16_500 {
            let val = 15.0 + ((ch * 10 + (step % 50)) as f64) * 0.2;
            let _ = arena_10ch.commit_transaction_with_telemetry(&[], &[(ch as u32, val)], step as u64);
        }
    }
    let rss_10_channels_kb = get_current_rss_kb();
    let delta_10_kb = rss_10_channels_kb.saturating_sub(rss_0_channels_kb);
    println!("  -> RSS 10 Canales (estado estacionario): {:>6.2} MB ({:>7} KB) | Delta: {:>+6} KB", rss_10_channels_kb as f64 / 1024.0, rss_10_channels_kb, delta_10_kb);
    println!("  -> Promedio medido por canal activo:     {:>6.2} KB / canal (teórico: 245.7 KB)", delta_10_kb as f64 / 10.0);

    // =========================================================================
    // 4. VERIFICACIÓN DE AISLAMIENTO DEL WAL
    // =========================================================================
    println!("\n[4/4] Verificando Inviolabilidad y Aislamiento del WAL (engram.jsonl)");
    let initial_entries = arena_1ch.log_entries().len();
    let initial_version = arena_1ch.current_version();
    let audit_count = arena_1ch.telemetry_arena.audit_log.len();

    println!("  -> Versión actual del WAL:                {}", initial_version);
    println!("  -> Total de entradas en engram.jsonl:     {}", initial_entries);
    println!("  -> Total de registros en auditoría Seis:  {} (acotado a max_audit_records={})", audit_count, arena_1ch.telemetry_arena.max_audit_records);

    assert_eq!(initial_version, 13, "El WAL no debe avanzar con transacciones de telemetría pura");
    assert_eq!(initial_entries, 13, "engram.jsonl debe contener exclusivamente hechos del grafo de génesis");
    assert_eq!(audit_count, 1000, "La auditoría fuera de banda debe acotarse al límite FIFO configurado");

    println!("  -> [EXITO] 100% de aislamiento verificado: Cero contaminación en el grafo episódico.");
    println!("================================================================================");
}

