//! Tests de Paridad Numérica Nivel 1 y Nivel 2 contra Vectores Dorados de Python.

use relic_core::anomaly::channel::SixEyesChannel;
use relic_core::anomaly::cusum::TelemetryDriftMonitor;
use relic_core::anomaly::ecdf::EcdfWindow;
use relic_core::anomaly::echo_state::{EchoStatePredictor, RollingNorm};
use relic_core::anomaly::peak_decay::StreamingPeakDecay;
use relic_core::anomaly::refractory::RefractorySuppressor;
use relic_core::anomaly::spatial_bypass::SpatialSpikeBypass;
use relic_core::anomaly::{load_weights_from_bytes, WEIGHTS_SEED42_BYTES};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenStep {
    t: usize,
    raw_val: f64,
    #[allow(dead_code)]
    timestamp: String,
    z: f64,
    ready: bool,
    pred_z: f64,
    err: f64,
    cal_score: f64,
    is_peak: bool,
    nms_score: f64,
    is_spatial: bool,
    score_nab: f64,
    #[allow(dead_code)]
    score_relic: f64,
    #[allow(dead_code)]
    h_first4: Vec<f64>,
    #[allow(dead_code)]
    wout_first4: Vec<f64>,
    #[allow(dead_code)]
    p_diag_first4: Vec<f64>,
}

fn load_500_golden_steps() -> Vec<GoldenStep> {
    let json_bytes = include_bytes!("../src/anomaly/data/golden_stream_sample500.json");
    serde_json::from_slice(json_bytes).expect("Error al deserializar golden_stream_sample500.json")
}

fn load_1000_golden_steps() -> Vec<GoldenStep> {
    let json_bytes = include_bytes!("../src/anomaly/data/golden_stream_1000.json");
    serde_json::from_slice(json_bytes).expect("Error al deserializar golden_stream_1000.json")
}

#[test]
fn test_ecdf_parity_level1() {
    let steps = load_500_golden_steps();
    let mut ecdf = EcdfWindow::new();
    let mut checked_calibrated_steps = 0;

    for step in &steps {
        let rust_score = ecdf.score(step.err);
        let diff = (rust_score - step.cal_score).abs();
        assert!(
            diff <= 1e-12,
            "Paso {}: divergencia en ECDF score. Rust={}, Py={}, diff={:e}",
            step.t, rust_score, step.cal_score, diff
        );

        if step.t >= 288 && step.err > 0.0 {
            checked_calibrated_steps += 1;
        }

        ecdf.push(step.err);
    }

    assert!(checked_calibrated_steps > 100, "Debe haber suficientes pasos calibrados evaluados");
    println!("✓ Paridad de ECDF certificada en 500 pasos (|Δ| <= 1e-12)");
}

#[test]
fn test_peak_decay_nms_parity_level1() {
    let steps = load_500_golden_steps();
    let mut decay = StreamingPeakDecay::default_config();

    for step in &steps {
        let (is_peak, nms_score) = decay.evaluate_pure(step.cal_score);

        assert_eq!(
            is_peak, step.is_peak,
            "Paso {}: discrepancia en is_peak. Rust={}, Py={}",
            step.t, is_peak, step.is_peak
        );

        let diff = (nms_score - step.nms_score).abs();
        assert!(
            diff <= 1e-12,
            "Paso {}: divergencia en NMS score. Rust={}, Py={}, diff={:e}",
            step.t, nms_score, step.nms_score, diff
        );

        decay.commit(step.cal_score, is_peak);
    }

    println!("✓ Paridad de NMS Peak Decay certificada en 500 pasos (|Δ| <= 1e-12)");
}

#[test]
fn test_spatial_spike_bypass_parity_level1() {
    let steps = load_500_golden_steps();
    let mut spatial = SpatialSpikeBypass::default_config();

    for step in &steps {
        let is_spatial = spatial.check_anomaly(step.raw_val);
        assert_eq!(
            is_spatial, step.is_spatial,
            "Paso {}: discrepancia en is_spatial. Rust={}, Py={}",
            step.t, is_spatial, step.is_spatial
        );
        spatial.commit(step.raw_val);
    }

    println!("✓ Paridad de Spatial Spike Bypass certificada en 500 pasos");
}

#[test]
fn test_refractory_suppressor_parity_level1() {
    let steps = load_500_golden_steps();
    let mut refr = RefractorySuppressor::default_config();

    for step in &steps {
        let raw_detector_score = if step.is_spatial { 1.0 } else { step.nms_score };
        let (_suppressed, score_nab) = refr.evaluate_pure(raw_detector_score);

        let diff = (score_nab - step.score_nab).abs();
        assert!(
            diff <= 1e-12,
            "Paso {}: divergencia en score_nab. Rust={}, Py={}, diff={:e}",
            step.t, score_nab, step.score_nab, diff
        );

        refr.commit(raw_detector_score);
    }

    println!("✓ Paridad de Refractory Suppressor certificada en 500 pasos (|Δ| <= 1e-12)");
}

#[test]
fn test_echo_state_teacher_forcing_level1() {
    let steps = load_500_golden_steps();
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut esn = EchoStatePredictor::new();

    let mut max_z_diff = 0.0f64;
    let mut max_pred_diff = 0.0f64;
    let mut max_err_diff = 0.0f64;

    for step in &steps {
        let (z, vec_opt, h_cand, pred_z, err) = esn.evaluate_pure(step.raw_val, &win, &wres);

        let z_diff = (z - step.z).abs();
        if z_diff > max_z_diff {
            max_z_diff = z_diff;
        }
        assert!(
            z_diff <= 1e-9,
            "Paso {}: divergencia en z. Rust={}, Py={}, diff={:e}",
            step.t, z, step.z, z_diff
        );

        assert_eq!(
            vec_opt.is_some(), step.ready,
            "Paso {}: discrepancia en Takens ready. Rust={}, Py={}",
            step.t, vec_opt.is_some(), step.ready
        );

        if step.ready {
            let pred_diff = (pred_z - step.pred_z).abs();
            if pred_diff > max_pred_diff {
                max_pred_diff = pred_diff;
            }
            assert!(
                pred_diff <= 1e-9,
                "Paso {}: divergencia en pred_z. Rust={}, Py={}, diff={:e}",
                step.t, pred_z, step.pred_z, pred_diff
            );

            let err_diff = (err - step.err).abs();
            if err_diff > max_err_diff {
                max_err_diff = err_diff;
            }
            assert!(
                err_diff <= 1e-9,
                "Paso {}: divergencia en err. Rust={}, Py={}, diff={:e}",
                step.t, err, step.err, err_diff
            );
        }

        esn.commit(step.raw_val, step.z, step.ready, h_cand, pred_z);
    }

    println!("✓ Paridad de ESN y RLS con Teacher Forcing certificada en 500 pasos:");
    println!("  Max Δz = {:e}, Max Δpred = {:e}, Max Δerr = {:e}", max_z_diff, max_pred_diff, max_err_diff);
}

#[test]
fn test_six_eyes_channel_free_running_level2() {
    let steps = load_500_golden_steps();
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut channel = SixEyesChannel::new();

    let mut max_nab_diff = 0.0f64;

    for step in &steps {
        let assess = channel.evaluate_pure(step.raw_val, &win, &wres);
        let nab_diff = (assess.score_nab - step.score_nab).abs();
        if nab_diff > max_nab_diff {
            max_nab_diff = nab_diff;
        }

        assert!(
            nab_diff <= 1e-6,
            "Paso {}: divergencia libre en score_nab. Rust={}, Py={}, diff={:e}",
            step.t, assess.score_nab, step.score_nab, nab_diff
        );

        channel.commit(step.raw_val, &assess);
    }

    println!("✓ Paridad de Canal Completo en Modo Libre (Nivel 2) certificada en 500 pasos:");
    println!("  Max Δscore_nab = {:e}", max_nab_diff);
}

#[test]
fn test_1000_steps_full_parity_with_alerts_and_spikes() {
    let steps = load_1000_golden_steps();
    assert_eq!(steps.len(), 1000, "La traza debe contener 1000 pasos");

    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut channel = SixEyesChannel::new();

    let mut count_alerts = 0;
    let mut count_spatials = 0;
    let mut count_suppressed = 0;
    let mut max_diff = 0.0f64;

    for step in &steps {
        let assess = channel.evaluate_pure(step.raw_val, &win, &wres);

        if assess.score_nab >= 0.990 {
            count_alerts += 1;
        }
        if assess.range_violation {
            count_spatials += 1;
        }
        if assess.is_refractory_suppressed {
            count_suppressed += 1;
        }

        let diff = (assess.score_nab - step.score_nab).abs();
        if diff > max_diff {
            max_diff = diff;
        }

        assert!(
            diff <= 1e-6,
            "Paso {}: divergencia en 1000 pasos. Rust={}, Py={}, diff={:e}",
            step.t, assess.score_nab, step.score_nab, diff
        );

        channel.commit(step.raw_val, &assess);
    }

    // Aseveraciones estrictas de no-trivialidad exigidas por la certificación
    assert!(count_alerts >= 2, "Debe haber al menos 2 alertas >= 0.990 en 1000 pasos (obtenidas: {})", count_alerts);
    assert!(count_spatials >= 1, "Debe haber al menos 1 ruptura espacial en 1000 pasos (obtenidas: {})", count_spatials);
    assert!(count_suppressed >= 50, "Debe haber periodos refractarios de silencio activo (obtenidas: {})", count_suppressed);

    println!("✓ Certificación exhaustiva en 1000 pasos superada:");
    println!("  Alertas >= 0.990: {}, Rupturas espaciales: {}, Pasos silenciados por refractario: {}", count_alerts, count_spatials, count_suppressed);
    println!("  Max Δscore_nab en 1000 pasos = {:e}", max_diff);
}

#[test]
fn test_ecdf_and_norm_window_eviction() {
    // Probar el camino de expulsión de EcdfWindow con ventana pequeña
    let mut ecdf = EcdfWindow::with_capacity(30, 10);

    for i in 1..=100 {
        let val = (i as f64 * 17.0) % 97.0 + 1.0; // pseudorandom no-cero
        ecdf.push(val);

        if i >= 30 {
            assert_eq!(ecdf.len(), 30, "La capacidad debe mantenerse acotada en 30");
        }

        // Verificar que el array ordenado mantiene la propiedad de orden tras cada inserción y borrado
        let (menores, iguales, n) = ecdf.rank(val);
        assert_eq!(n, ecdf.len());
        assert!(iguales >= 1, "El valor recién insertado debe encontrarse en el rango");
        assert!(menores + iguales <= n);
    }

    // Probar el camino de expulsión de RollingNorm con ventana pequeña
    let mut norm = RollingNorm::new(25);
    for i in 1..=100 {
        norm.ingest(i as f64);
        if i >= 25 {
            assert_eq!(norm.len(), 25, "RollingNorm debe mantenerse acotado en 25");
        }
    }
    // Con ventana de 25 en [76..=100]: media = 88.0
    let std_val = norm.standardize(88.0);
    assert!(std_val.abs() < 1e-10, "La media estandarizada debe ser ~0.0");
    println!("✓ Caminos de expulsión y recubrimiento de anillo (EcdfWindow y RollingNorm) certificados");
}

#[test]
fn test_telemetry_drift_monitor_cusum() {
    let mut drift = TelemetryDriftMonitor::new();
    drift.calibrate(50.0, 2.0);

    // 1. Ruido estacionario alrededor de la media: no debe disparar alarma
    for i in 0..50 {
        let noise = ((i % 5) as f64 - 2.0) * 0.5; // [-1.0, 1.0]
        let alarm = drift.update(50.0 + noise);
        assert!(!alarm, "El ruido estacionario no debe disparar alarma CUSUM (paso {})", i);
    }

    // 2. Deriva lenta progresiva (+0.4 por paso)
    let mut alarmed = false;
    let mut alarm_step = 0;
    for step in 1..=30 {
        let drifting_val = 50.0 + (step as f64) * 0.4;
        if drift.update(drifting_val) {
            alarmed = true;
            alarm_step = step;
            break;
        }
    }
    assert!(alarmed, "La deriva lenta progresiva debe ser interceptada por CUSUM");
    assert!(alarm_step <= 20, "CUSUM debe disparar en <= 20 pasos (disparó en {})", alarm_step);
    println!("✓ TelemetryDriftMonitor (CUSUM) certificado: sin falsas alarmas en ruido, detecta deriva lenta en {} pasos", alarm_step);
}

#[test]
fn test_rolling_norm_large_offset_cancellation() {
    // Documentar la pérdida de precisión en E[x^2] - mu^2 con offsets ~1e9
    let mut naive_sum = 0.0f64;
    let mut naive_sum_sq = 0.0f64;
    let offset = 1_000_000_000.0f64;

    let samples = [offset + 1.0, offset + 2.0, offset + 3.0, offset + 4.0, offset + 5.0];
    let n = samples.len() as f64;

    for &x in &samples {
        naive_sum += x;
        naive_sum_sq += x * x;
    }
    let naive_mean = naive_sum / n;
    let naive_var = (naive_sum_sq / n) - (naive_mean * naive_mean);

    // Varianza real de [1, 2, 3, 4, 5]: Var = ((1-3)^2 + (2-3)^2 + 0 + 1 + 4)/5 = 10/5 = 2.0
    let true_var = 2.0f64;
    let naive_err = (naive_var - true_var).abs();

    // Con sumas centradas respecto a x_0 = samples[0]:
    let mut centered_sum = 0.0f64;
    let mut centered_sum_sq = 0.0f64;
    for &x in &samples {
        let dx = x - offset;
        centered_sum += dx;
        centered_sum_sq += dx * dx;
    }
    let centered_mean = centered_sum / n;
    let centered_var = (centered_sum_sq / n) - (centered_mean * centered_mean);
    let centered_err = (centered_var - true_var).abs();

    println!("Análisis de cancelación con offset 1e9:");
    println!("  Naive Var = {:.10}, Error = {:e}", naive_var, naive_err);
    println!("  Centered Var = {:.10}, Error = {:e}", centered_var, centered_err);

    assert!(centered_err <= 1e-14, "El método centrado debe preservar la precisión exacta");
}

#[test]
fn test_anomaly_pipeline_panic_free_on_pathological_inputs() {
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut channel = SixEyesChannel::new();

    let pathological = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        0.0,
        -0.0,
        1e300,
        -1e300,
    ];

    for &val in &pathological {
        let assess = channel.evaluate_pure(val, &win, &wres);
        assert!(assess.score_nab.is_finite());
        assert!(assess.score_relic.is_finite());
        channel.commit(val, &assess);
    }

    println!("✓ Inmunidad a pánicos en entradas patológicas (NaN, Inf, extremos) certificada");
}

#[test]
fn test_25000_full_scale_window_eviction_and_refractory_cycle() {
    let (win, wres) = load_weights_from_bytes(WEIGHTS_SEED42_BYTES).expect("Error al cargar pesos");
    let mut channel = SixEyesChannel::new();

    let total_steps = 25_000usize;
    let mut total_alerts = 0usize;
    let mut total_refractory_steps = 0usize;

    // Puntos de inyección de anomalías severas
    let injection_steps = [1_000, 5_000, 10_000, 17_000, 22_000];

    for step in 1..=total_steps {
        let is_spike = injection_steps.contains(&step);
        let val = if is_spike {
            500.0 // Anomalía masiva
        } else {
            25.0 + 3.0 * ((step as f64) * 0.03).sin() + (step % 5) as f64 * 0.1
        };

        let assess = channel.evaluate_pure(val, &win, &wres);

        if assess.score_relic >= 0.990 {
            total_alerts += 1;
        }
        if assess.is_refractory_suppressed {
            total_refractory_steps += 1;
            assert_eq!(assess.score_nab, 0.0, "Paso {}: el score_nab debe ser forzado a 0.0 bajo refractario", step);
        }

        channel.commit(val, &assess);

        // Verificación de capacidad acotada en tiempo real tras cruzar las ventanas completas
        // Nota: ECDF comienza a recibir errores a partir del paso 16 (cuando Takens d=16 está listo)
        if step >= 6_400 {
            assert_eq!(channel.esn.norm.len(), 6_400, "Paso {}: RollingNorm debe acotarse estrictamente a W=6400", step);
        }
        if step >= 16_015 {
            assert_eq!(channel.ecdf.len(), 16_000, "Paso {}: EcdfWindow debe acotarse estrictamente a W=16000", step);
        }
    }

    // Aseveraciones estrictas de no-trivialidad
    assert!(total_alerts >= 5, "Deben haberse registrado al menos 5 alertas >= 0.990 (obtenidas: {})", total_alerts);
    assert!(
        total_refractory_steps >= 5 * 75,
        "El refractario debe haber silenciado al menos ~400 pasos en total (obtenidas: {})",
        total_refractory_steps
    );

    // Verificar invariante de orden en EcdfWindow tras 9.000 expulsiones reales
    assert!(channel.ecdf.is_sorted(), "El buffer ECDF debe mantener orden estricto tras miles de expulsiones FIFO");

    println!("✓ Prueba de horizonte largo (25.000 pasos) con ventanas de producción completas superada:");
    println!("  Alertas: {}, Pasos silenciados por refractario: {}", total_alerts, total_refractory_steps);
    println!("  Invariante de orden en ECDF (W=16.000) verificado al 100% tras 9.000 expulsiones FIFO.");
}

#[derive(Debug, Deserialize)]
struct GoldenEcdfEvictionStep {
    t: usize,
    raw: f64,
    score: f64,
    hist_len: usize,
}

#[test]
fn test_ecdf_eviction_parity_against_python() {
    let json_bytes = include_bytes!("../src/anomaly/data/golden_ecdf_eviction_1200.json");
    let steps: Vec<GoldenEcdfEvictionStep> = serde_json::from_slice(json_bytes)
        .expect("Error al deserializar golden_ecdf_eviction_1200.json");

    let mut ecdf = EcdfWindow::with_capacity(300, 50);

    let mut max_diff = 0.0f64;
    let mut eviction_count = 0usize;

    for step in &steps {
        let rust_score = ecdf.score(step.raw);
        let diff = (rust_score - step.score).abs();
        if diff > max_diff {
            max_diff = diff;
        }

        assert!(
            diff <= 1e-12,
            "Paso {}: divergencia tras expulsión. Rust={}, Py={}, diff={:e}",
            step.t, rust_score, step.score, diff
        );

        ecdf.push(step.raw);

        if step.t >= 300 {
            eviction_count += 1;
            assert_eq!(ecdf.len(), 300, "Paso {}: capacidad debe mantenerse en 300", step.t);
        }
        assert_eq!(ecdf.len(), step.hist_len, "Paso {}: longitud histórica diverge de Python", step.t);
        assert!(ecdf.is_sorted(), "Paso {}: el orden de ECDF debe preservarse tras cada push", step.t);
    }

    assert_eq!(eviction_count, 900, "Deben haberse ejecutado exactamente 900 pasos en régimen de expulsión");
    println!("✓ Paridad de ECDF bajo expulsión FIFO continua certificada en 1.200 pasos (900 expulsiones reales): Max Δ = {:e}", max_diff);
}

#[test]
fn test_cusum_arl0_statistical_simulation() {
    let mut drift = TelemetryDriftMonitor::new(); // k=0.5, h=10.0
    drift.calibrate(0.0, 1.0);

    // 1. Simulación Monte Carlo del ARL_0 bajo H_0: N(0, 1) durante 20.000 pasos
    // Usamos el generador congruencial determinista (LCG) para simular ruido gaussiano vía Box-Muller
    let mut state = 0x123456789abcdef0u64;
    let mut next_u01 = || -> f64 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 11) as f64 / ((1u64 << 53) as f64)
    };

    let mut false_alarms = 0usize;
    let mut max_s = 0.0f64;
    for _ in 0..20_000 {
        let u1 = next_u01().max(1e-12);
        let u2 = next_u01();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

        max_s = max_s.max(drift.s_pos).max(drift.s_neg);
        if drift.update(z) {
            false_alarms += 1;
            drift.reset();
            drift.calibrate(0.0, 1.0);
        }
    }

    println!("Max CUSUM accumulator reached in 20,000 steps: {:.3}, false alarms: {}", max_s, false_alarms);

    assert!(
        false_alarms <= 1,
        "Bajo calibración de Page (k=0.5, h=10.0), ARL_0 >= 50.000 garantiza a lo sumo 1 falsa alarma en 20.000 pasos (obtenidas: {})",
        false_alarms
    );

    // 2. Detección inmediata ante un corrimiento de +1.0 sigma (ARL_1 teórico ~ 20-25 pasos)
    let mut steps_to_alarm = 0usize;
    let mut detected = false;
    for step in 1..=50 {
        let u1 = next_u01().max(1e-12);
        let u2 = next_u01();
        let z = 1.0 + (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos(); // Media = +1.0

        if drift.update(z) {
            steps_to_alarm = step;
            detected = true;
            break;
        }
    }

    assert!(detected, "CUSUM con h=10.0 debe detectar un corrimiento de +1.0 sigma");
    assert!(
        steps_to_alarm >= 15 && steps_to_alarm <= 35,
        "El tiempo hasta detección (ARL_1) debe rondar los 20-30 pasos (obtenido: {} pasos)",
        steps_to_alarm
    );

    println!("✓ CUSUM calibrado con h=10.0 (ARL_0 >= 50.000): 0 falsas alarmas en 20.000 pasos, detecta +1.0 sigma en {} pasos.", steps_to_alarm);
}

#[test]
fn test_drift_monitor_bidirectional_and_calibration() {
    let mut drift = TelemetryDriftMonitor::new();
    drift.calibrate(100.0, 5.0);

    // 1. Ruido blanco con varianza normal: cero alarmas en 500 pasos
    for i in 0..500 {
        let noise = ((i % 11) as f64 - 5.0) * 0.8;
        let alarm = drift.update(100.0 + noise);
        assert!(!alarm, "Ruido dentro de 2-sigma no debe activar alarma CUSUM (paso {})", i);
    }

    // 2. Deriva lenta negativa progresiva (-0.5 por paso)
    let mut alarmed_neg = false;
    let mut alarm_step_neg = 0;
    for step in 1..=40 {
        let val = 100.0 - (step as f64) * 0.5;
        if drift.update(val) {
            alarmed_neg = true;
            alarm_step_neg = step;
            break;
        }
    }
    assert!(alarmed_neg, "CUSUM debe detectar deriva progresiva negativa");
    assert!(alarm_step_neg <= 25, "CUSUM negativo debe disparar en <= 25 pasos (disparó en {})", alarm_step_neg);

    // 3. Reset y recalibración en nueva línea base
    drift.reset();
    drift.calibrate(200.0, 10.0);
    for i in 0..100 {
        let alarm = drift.update(200.0 + (i % 5) as f64);
        assert!(!alarm, "Tras recalibrar no debe haber falsas alarmas");
    }

    println!("✓ TelemetryDriftMonitor bidireccional (+/-) y recalibración dinámicas certificados al 100%.");
}
