//! Banco de falsación TDD (Fases 1 y 2 del plan de dirección de flujo).
//!
//! - Controles: pasan hoy y fijan el comportamiento que NO debe regresar.
//! - Falsadores (`#[ignore]`): estado ROJO esperado. Ejecutar con
//!   `cargo test -p relic_cli --test flow_direction_falsification -- --ignored`
//!   Al implementar el clasificador Inflow/Outflow se retira el `#[ignore]`.
//!
//! Criterio de rechazo: ninguna prueba puede pasar añadiendo verbos a listas;
//! los casos usan léxico fuera de las listas actuales de `dispatch.rs`.

use relic_cli::{parse_dispatch_intent, DispatchIntent};

/// Resultado numérico final si el sistema logra un plan aritmético; `None` si se abstiene.
fn eval(text: &str) -> Option<f64> {
    match parse_dispatch_intent(text) {
        DispatchIntent::Arithmetic(plan) => plan.execute().ok(),
        _ => None,
    }
}

fn approx(a: Option<f64>, b: f64) -> bool {
    a.is_some_and(|v| (v - b).abs() < 1e-9)
}

// ---------------------------------------------------------------------------
// Controles (verdes hoy)
// ---------------------------------------------------------------------------

#[test]
fn control_outflow_basico() {
    assert!(approx(eval("Tengo 10 caramelos y regalo 2"), 8.0));
}

#[test]
fn control_inflow_basico() {
    assert!(approx(eval("Tengo 10 caramelos y compro 2"), 12.0));
}

#[test]
fn control_abstencion_sin_estado_inicial() {
    // Cuádrupla manca: sin estado inicial no debe fabricarse un resultado.
    assert_eq!(eval("Regalé 2 caramelos"), None);
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------

#[test]
#[ignore = "TDD rojo: requiere clasificador de dirección por rol"]
fn falsador_voz_pasiva_me_regalaron_es_inflow() {
    // Misma raíz "regal-" que "regalé", dirección opuesta (receptor = yo).
    assert!(approx(eval("Tenía 10 caramelos y me regalaron 2"), 12.0));
}

#[test]
#[ignore = "TDD rojo: requiere clasificador de dirección por rol"]
fn falsador_voz_activa_regale_es_outflow() {
    assert!(approx(eval("Tenía 10 caramelos y regalé 2"), 8.0));
}

#[test]
#[ignore = "TDD rojo: requiere clasificador de dirección por rol"]
fn falsador_par_minimo_misma_raiz_direccion_opuesta() {
    let sale = eval("Tenía 10 caramelos y regalé 2");
    let entra = eval("Tenía 10 caramelos y me regalaron 2");
    assert!(sale.is_some() && entra.is_some());
    assert!(entra.unwrap() > sale.unwrap());
}

#[test]
#[ignore = "TDD rojo: sinónimo abierto fuera de listas"]
fn falsador_sinonimo_abierto_se_me_extraviaron() {
    assert!(approx(eval("Tengo 9 peras y se me extraviaron 4"), 5.0));
}

#[test]
#[ignore = "TDD rojo: sinónimo abierto fuera de listas"]
fn falsador_sinonimo_abierto_inflow_me_obsequiaron() {
    assert!(approx(eval("Tengo 3 peras y me obsequiaron 4"), 7.0));
}

#[test]
#[ignore = "TDD rojo: subcadena 'di' no debe decidir dirección"]
fn falsador_subcadena_espuria_no_decide() {
    // "comida"/"medida" contienen 'di'/'como'/'mas' como subcadena; no son verbos de salida.
    assert_eq!(eval("Tengo 5 manzanas y me dieron una medida de 3"), None);
}

#[test]
#[ignore = "TDD rojo: negación invierte/anula el evento"]
fn falsador_negacion_no_regale_no_resta() {
    assert!(approx(eval("Tenía 10 caramelos y no regalé 2"), 10.0));
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------

#[test]
#[ignore = "TDD rojo: unidades jerárquicas/contenedores (Fase 1-2)"]
fn falsador_contenedor_cajas_misma_dimension() {
    // Ambas operaciones en cajas: 3 - 1 = 2 (no debe mezclar con "caramelos").
    assert!(approx(eval("Tenía 3 cajas de caramelos y regalé 1 caja"), 2.0));
}

#[test]
#[ignore = "TDD rojo: multiplicador de contenedor"]
fn falsador_contenedor_con_multiplicador() {
    // 2 cajas de 10 caramelos => 20 unidades; 20 - 20 = 0.
    assert!(approx(eval("Tengo 20 caramelos y regalo 2 cajas de 10"), 0.0));
}

#[test]
#[ignore = "TDD rojo: invariante física, se anticipa aquí"]
fn falsador_no_se_puede_regalar_mas_de_lo_que_se_tiene() {
    // Resultado negativo para objetos contables debe ser abstención, no -3.
    assert_eq!(eval("Tengo 2 caramelos y regalo 5"), None);
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------

#[test]
#[ignore = "TDD rojo: EntityDescriptor aún no existe"]
fn falsador_flexion_misma_entidad() {
    // Contrato futuro: "caramelo" / "caramelos" / "caramelito" => mismo entity_id.
    // Hoy no hay API; la prueba se completará al exponer EntityDescriptor.
    unimplemented!("exponer relic_core::rir::EntityDescriptor y resolver_entidad()");
}
