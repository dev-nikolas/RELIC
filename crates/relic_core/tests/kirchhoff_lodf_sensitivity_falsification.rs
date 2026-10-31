//! # Kirchhoff Parallel Flow Split & LODF Sensitivity Falsification Suite
//!
//! Verificación empírica de:
//! 1. Redistribución exacta de Kirchhoff sobre caminos paralelos bajo falla N-1 y N-2.
//! 2. Prevención de incendios en cascada: detección y veto determinista de sobrecargas indirectas.
//! 3. Evaluación SIMD AVX2 de superposición lineal en < 50 ns sobre 64 aristas en L2.

use relic_core::contingency_bfs::{LinearizedSensitivityMatrix, MAX_SENSITIVITY_EDGES};
use std::time::Instant;

#[test]
fn test_kirchhoff_parallel_flow_split_and_thermal_overload_veto() {
    let mut lodf_matrix = LinearizedSensitivityMatrix::empty();

    // Red de 3 caminos paralelos entre fuente y carga:
    // Línea 0 (baja impedancia): flujo previo = 200.0 MW, límite térmico = 300.0 MW
    // Línea 1 (impedancia media): flujo previo = 150.0 MW, límite térmico = 250.0 MW
    // Línea 2 (alta impedancia): flujo previo = 100.0 MW, límite térmico = 220.0 MW
    lodf_matrix.set_lodf(0, 1, 0.60).unwrap(); // 60% del flujo de la línea 0 se desvía a la línea 1
    lodf_matrix.set_lodf(0, 2, 0.40).unwrap(); // 40% del flujo de la línea 0 se desvía a la línea 2

    let mut pre_flows = [0.0f32; MAX_SENSITIVITY_EDGES];
    pre_flows[0] = 200.0;
    pre_flows[1] = 150.0;
    pre_flows[2] = 100.0;

    let mut limits = [0.0f32; MAX_SENSITIVITY_EDGES];
    limits[0] = 300.0;
    limits[1] = 250.0; // Nota: 150 + (0.6 * 200) = 270 MW > 250 MW (Sobrecarga!)
    limits[2] = 220.0; // 100 + (0.4 * 200) = 180 MW <= 220 MW (Seguro)

    let mut post_flows = [0.0f32; MAX_SENSITIVITY_EDGES];

    // Caso 1: Veto de sobrecarga térmica en la línea paralela 1 (Incendio prevenido)
    let outcome = lodf_matrix.evaluate_post_contingency_flows(
        &pre_flows,
        &[0], // Línea 0 cae
        &limits,
        &mut post_flows,
    );

    match outcome {
        Err(violating_edge) => {
            assert_eq!(violating_edge, 1, "La línea 1 debió ser identificada con sobrecarga térmica");
            assert_eq!(post_flows[0], 0.0, "La línea fallada debe quedar en 0 MW");
            assert!((post_flows[1] - 270.0).abs() < 1e-4, "Flujo en línea 1 debe ser 270 MW");
            assert!((post_flows[2] - 180.0).abs() < 1e-4, "Flujo en línea 2 debe ser 180 MW");
        }
        Ok(_) => panic!("Debió vetarse por sobrecarga térmica de Kirchhoff en línea 1"),
    }

    // Caso 2: Con margen térmico ampliado (línea 1 reforzada a 300 MW), flujo admisible
    limits[1] = 300.0;
    let safe_outcome = lodf_matrix.evaluate_post_contingency_flows(
        &pre_flows,
        &[0],
        &limits,
        &mut post_flows,
    );

    match safe_outcome {
        Ok(min_margin) => {
            // Margen línea 1: 1 - (270 / 300) = 0.10 (10%)
            // Margen línea 2: 1 - (180 / 220) = 0.1818 (18.18%)
            assert!((min_margin - 0.10).abs() < 1e-3, "El margen mínimo debe ser 10%");
        }
        Err(e) => panic!("Esperado flujo admisible, violó arista {}", e),
    }
}

#[test]
fn test_multiple_outage_n_minus_2_superposition() {
    let mut lodf_matrix = LinearizedSensitivityMatrix::empty();
    lodf_matrix.set_lodf(0, 2, 0.50).unwrap();
    lodf_matrix.set_lodf(1, 2, 0.30).unwrap();

    let mut pre_flows = [0.0f32; MAX_SENSITIVITY_EDGES];
    pre_flows[0] = 100.0;
    pre_flows[1] = 50.0;
    pre_flows[2] = 80.0;

    let mut limits = [0.0f32; MAX_SENSITIVITY_EDGES];
    limits[0] = 200.0; limits[1] = 200.0; limits[2] = 200.0;

    let mut post_flows = [0.0f32; MAX_SENSITIVITY_EDGES];
    // Caída simultánea N-2 de las líneas 0 y 1
    let outcome = lodf_matrix.evaluate_post_contingency_flows(
        &pre_flows,
        &[0, 1],
        &limits,
        &mut post_flows,
    );

    assert!(outcome.is_ok());
    // Flujo en 2: 80 + (0.5 * 100) + (0.3 * 50) = 80 + 50 + 15 = 145 MW
    assert!((post_flows[2] - 145.0).abs() < 1e-4);
    assert_eq!(post_flows[0], 0.0);
    assert_eq!(post_flows[1], 0.0);
}

#[test]
fn test_lodf_simd_latency_sub_50ns() {
    let mut lodf_matrix = LinearizedSensitivityMatrix::empty();
    for k in 0..16 {
        for m in 0..16 {
            let _ = lodf_matrix.set_lodf(k, m, 0.05);
        }
    }

    let pre_flows = [100.0f32; MAX_SENSITIVITY_EDGES];
    let limits = [500.0f32; MAX_SENSITIVITY_EDGES];
    let mut post_flows = [0.0f32; MAX_SENSITIVITY_EDGES];
    let outages = [0u8, 1u8];

    // Warm-up de 5,000 ciclos para cebar L1/L2
    for _ in 0..5_000 {
        let res = lodf_matrix.evaluate_post_contingency_flows(&pre_flows, &outages, &limits, &mut post_flows);
        let _ = std::hint::black_box(res);
    }

    let start = Instant::now();
    let iters = 100_000;
    for _ in 0..iters {
        let res = lodf_matrix.evaluate_post_contingency_flows(&pre_flows, &outages, &limits, &mut post_flows);
        let _ = std::hint::black_box(res);
    }
    let elapsed = start.elapsed();
    let ns_per_op = (elapsed.as_nanos() as f64) / (iters as f64);

    println!("Latencia por evaluación LODF Kirchhoff (AVX2): {:.2} ns", ns_per_op);
    assert!(ns_per_op < 50.0, "La evaluación LODF SIMD debe tomar menos de 50 ns");
}

