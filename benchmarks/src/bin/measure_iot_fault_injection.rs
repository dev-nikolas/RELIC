//! Evaluación Empírica Rigurosa de Detección de Anomalías y Fallas en Telemetría Real.
//!
//! Cumple estrictamente con los criterios de salida:
//! 1. Línea Base y Tasa de Falsas Alarmas (FP/día a 288 muestras/día) en series reales limpias.
//! 2. CUSUM con referencia fija y política de re-anclaje.
//! 3. Retardo de detección medido en pasos discretos (mediana, p90, tasa de pérdida en H=50 pasos).
//! 4. Inyecciones estrictamente dentro de rango [min, max] ± 5% (amplitudes 0.5σ, 1.0σ, 1.5σ, 2.0σ).
//! 5. Ablación sistemática con y sin SpatialSpikeBypass.
//! 6. 30 posiciones aleatorias de inyección por cada configuración.
//! 7. Medición empírica con `Instant` del costo de resolución MS-BFS con el mapa de overrides poblado.

use relic_blackwall::{BindingTarget, BlackwallGate};
use relic_core::anomaly::{load_weights_from_bytes, SixEyesChannel, WEIGHTS_SEED42_BYTES};
use relic_lexicon::LexiconPayload;
use serde_json;
use std::fs;
use std::time::Instant;

fn load_series(path: &str) -> Vec<f64> {
    let content = fs::read_to_string(path).unwrap_or_else(|_| panic!("No se pudo leer {}", path));
    serde_json::from_str(&content).unwrap_or_else(|_| panic!("Error al parsear {}", path))
}

fn compute_stats(vals: &[f64]) -> (f64, f64, f64, f64) {
    let n = vals.len() as f64;
    let mean = vals.iter().sum::<f64>() / n;
    let var = vals.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std = var.sqrt();
    let min_v = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_v = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (mean, std, min_v, max_v)
}

fn compute_median_and_p90(mut vals: Vec<usize>) -> (f64, f64) {
    if vals.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    vals.sort_unstable();
    let n = vals.len();
    let median = if n % 2 == 1 {
        vals[n / 2] as f64
    } else {
        (vals[n / 2 - 1] + vals[n / 2]) as f64 / 2.0
    };
    let idx_p90 = ((n as f64 * 0.90).ceil() as usize).saturating_sub(1).min(n - 1);
    (median, vals[idx_p90] as f64)
}

fn evaluate_baseline_clean(
    name: &str,
    series: &[f64],
    alert_thresh: f64,
    block_thresh: f64,
    win: &[[f64; 16]; 64],
    wres: &[[f64; 64]; 64],
) {
    let mut ch = SixEyesChannel::new();
    let total_steps = series.len();
    let mut alert_count = 0usize;
    let mut block_count = 0usize;
    let mut refractory_steps = 0usize;

    for &val in series {
        let assess = ch.evaluate_pure(val, win, wres);
        if assess.score_relic >= alert_thresh || assess.range_violation || assess.drift_alarm {
            alert_count += 1;
        }
        if assess.score_relic >= block_thresh || assess.range_violation {
            block_count += 1;
        }
        if assess.is_refractory_suppressed {
            refractory_steps += 1;
        }
        ch.commit(val, &assess);
    }

    let days = total_steps as f64 / 288.0;
    let alerts_per_day = alert_count as f64 / days;
    let blocks_per_day = block_count as f64 / days;

    println!("  [{}] N = {} muestras ({:.2} días a 5 min/muestra):", name, total_steps, days);
    println!("    - Nivel Alerta ({:.3}): {} alarmas  ->  {:.2} falsas alarmas / día", alert_thresh, alert_count, alerts_per_day);
    println!("    - Nivel Bloqueo ({:.3}): {} bloques  ->  {:.2} bloqueos en frío / día", block_thresh, block_count, blocks_per_day);
    println!("    - Pasos silenciados por refractario: {}", refractory_steps);
}

fn run_fault_sweep(
    series: &[f64],
    fault_type: &str,
    amplitude_sigma: f64,
    bypass_active: bool,
    win: &[[f64; 16]; 64],
    wres: &[[f64; 64]; 64],
) -> (f64, f64, f64, String) {
    let (_mean, std, min_v, max_v) = compute_stats(&series[..400]);
    let span = max_v - min_v;
    let horizon = 50usize;
    let num_positions = 30usize;

    let mut injection_points = Vec::with_capacity(num_positions);
    let mut seed = 0x9876543210fedcbau64;
    let min_p = 350usize;
    let max_p = series.len().saturating_sub(horizon + 5);

    if max_p <= min_p {
        panic!("Serie demasiado corta para el barrido");
    }

    for _ in 0..num_positions {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let p = min_p + ((seed >> 16) as usize % (max_p - min_p));
        injection_points.push(p);
    }

    let mut detected_latencies = Vec::new();
    let mut missed_count = 0usize;
    let mut primary_trigger = "None".to_string();

    for &t_inj in &injection_points {
        let mut ch = SixEyesChannel::new();
        let mut detected_step = None;

        for step in 0..series.len() {
            let mut val = series[step];

            if step >= t_inj && step < t_inj + horizon {
                let delta = match fault_type {
                    "step_pos" => amplitude_sigma * std,
                    "step_neg" => -amplitude_sigma * std,
                    "ramp" => 0.05 * std * (step - t_inj + 1) as f64,
                    "flatline" => series[t_inj] - val,
                    _ => 0.0,
                };

                val += delta;
                let bound_lo = min_v - 0.04 * span;
                let bound_hi = max_v + 0.04 * span;
                val = val.clamp(bound_lo, bound_hi);
            }

            let mut assess = ch.evaluate_pure(val, win, wres);

            if !bypass_active {
                assess.range_violation = false;
                assess.score_relic = assess.nms_score;
            }

            if step >= t_inj && detected_step.is_none() {
                if assess.score_relic >= 0.990 || assess.range_violation || assess.drift_alarm {
                    detected_step = Some(step - t_inj);
                    if primary_trigger == "None" {
                        primary_trigger = if assess.range_violation {
                            "Bypass".to_string()
                        } else if assess.drift_alarm {
                            "CUSUM".to_string()
                        } else {
                            "ESN".to_string()
                        };
                    }
                }
            }

            ch.commit(val, &assess);

            if step > t_inj + horizon {
                break;
            }
        }

        if let Some(lat) = detected_step {
            if lat <= horizon {
                detected_latencies.push(lat);
            } else {
                missed_count += 1;
            }
        } else {
            missed_count += 1;
        }
    }

    let miss_rate = (missed_count as f64 / num_positions as f64) * 100.0;
    let (med, p90) = compute_median_and_p90(detected_latencies);
    (med, p90, miss_rate, primary_trigger)
}

fn measure_msbfs_cost_with_overrides() {
    println!("\n[4] Medición Real de Impacto Topológico MS-BFS (RAII Guard y Costo de Overrides):");
    let mut gate = BlackwallGate::with_genesis();

    let mut facts = Vec::new();
    for i in 0..15 {
        facts.push(LexiconPayload::graph_fact(
            format!("node_{i}"),
            "links",
            format!("node_{}", i + 1),
        ));
    }
    gate.commit_transaction_with_telemetry(&facts, &[], 100).unwrap();

    for i in 0..8 {
        gate.telemetry_arena.register_critical_pair(format!("node_{i}"), format!("node_{}", i + 8));
    }

    let target = BindingTarget::Edge {
        source: "node_4".to_string(),
        target: "node_5".to_string(),
    };

    let iters = 10_000usize;
    let t0 = Instant::now();
    let mut total_lost = 0usize;

    for _ in 0..iters {
        let lost = gate.compute_topological_impact_set(&target);
        total_lost += lost.len();
    }
    let elapsed = t0.elapsed();
    let us_per_call = elapsed.as_nanos() as f64 / (iters as f64 * 1000.0);

    println!("  -> Iteraciones: {} llamadas", iters);
    println!("  -> Tiempo total: {:.2?}", elapsed);
    println!("  -> Latencia promedio por cálculo de impacto: {:.2} µs (con mapa de overrides poblado)", us_per_call);
    println!("  -> Pares críticos cortados detectados por llamada: {}", total_lost / iters);
    println!("  -> Verificación fast-path tras {} llamadas: is_empty() = {}", iters, gate.active_view().episodic.edge_world_overrides.is_empty());
}

fn main() {
    println!("================================================================================");
    println!("  EVALUACIÓN FORMAL Y RIGUROSA DE SEIS OJOS EN TELEMETRÍA REAL (NAB & IIoT)");
    println!("================================================================================");

    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Carga de pesos");

    let s_iio = load_series("RELIC/benchmarks/iio_clean.json");
    let s_traffic = load_series("RELIC/benchmarks/traffic_clean.json");
    let s_art = load_series("RELIC/benchmarks/art_daily_clean.json");

    println!("\n[1] TASA DE FALSAS ALARMAS SOBRE SERIES LIMPIAS (288 muestras/día = 5 min/paso):");
    evaluate_baseline_clean("realAWSCloudwatch_IIoT (i-a2eb1cd9)", &s_iio, 0.990, 0.999, &win, &wres);
    evaluate_baseline_clean("realTraffic_occupancy_6005", &s_traffic, 0.990, 0.999, &win, &wres);
    evaluate_baseline_clean("artificialNoAnomaly_art_daily", &s_art, 0.990, 0.999, &win, &wres);

    println!("\n[2] BARRIDO DE RETARDO DENTRO DE RANGO — CON SPATIAL BYPASS ACTIVO (H=50 pasos, N=30 posiciones):");
    println!("{:<24} | {:<8} | {:<10} | {:<10} | {:<10} | {:<10}",
        "Tipo de Falla", "Amplitud", "Mediana (t)", "p90 (t)", "Pérdida %", "Gatillo"
    );
    println!("{:-<24}-|-{:-<8}-|-{:-<10}-|-{:-<10}-|-{:-<10}-|-{:-<10}", "", "", "", "", "", "");

    for &(ft, amp) in &[
        ("step_pos", 0.5),
        ("step_pos", 1.0),
        ("step_pos", 1.5),
        ("step_pos", 2.0),
        ("step_neg", 1.0),
        ("ramp", 0.05),
        ("flatline", 0.0),
    ] {
        let (med, p90, miss, trig) = run_fault_sweep(&s_traffic, ft, amp, true, &win, &wres);
        println!("{:<24} | {:<8.2} | {:<10.1} | {:<10.1} | {:<10.1}% | {:<10}",
            ft, amp, med, p90, miss, trig
        );
    }

    println!("\n[3] ABLACIÓN PURA: RENDIMIENTO DEL ESN + CUSUM (SIN SPATIAL BYPASS, H=50 pasos, N=30 posiciones):");
    println!("{:<24} | {:<8} | {:<10} | {:<10} | {:<10} | {:<10}",
        "Tipo de Falla", "Amplitud", "Mediana (t)", "p90 (t)", "Pérdida %", "Gatillo"
    );
    println!("{:-<24}-|-{:-<8}-|-{:-<10}-|-{:-<10}-|-{:-<10}-|-{:-<10}", "", "", "", "", "", "");

    for &(ft, amp) in &[
        ("step_pos", 0.5),
        ("step_pos", 1.0),
        ("step_pos", 1.5),
        ("step_pos", 2.0),
        ("step_neg", 1.0),
        ("ramp", 0.05),
        ("flatline", 0.0),
    ] {
        let (med, p90, miss, trig) = run_fault_sweep(&s_traffic, ft, amp, false, &win, &wres);
        println!("{:<24} | {:<8.2} | {:<10.1} | {:<10.1} | {:<10.1}% | {:<10}",
            ft, amp, med, p90, miss, trig
        );
    }

    measure_msbfs_cost_with_overrides();

    println!("\n================================================================================");
    println!("  EVALUACIÓN FORMAL COMPLETADA CON ÉXITO");
    println!("================================================================================");
}
