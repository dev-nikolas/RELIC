//! Demostración y medición de las propiedades singulares de originalidad de RELIC.
use std::time::Instant;
use relic_core::certificate::{verify_negative, SearchOutcome};
use relic_core::episodic_memory::EpisodicMemory;
use relic_core::ms_bfs::solve_ms_bfs_counterfactual;
use relic_core::reasoning_engine::ReasoningEngine;

fn test_1_epistemic_certificates_and_tamper_resistance() {
    println!("\n--- [PRUEBA 1] CERTIFICADOS EPISTÉMICOS INDUCTIVOS Y RESISTENCIA A MANIPULACIÓN ---");
    let mut mem = EpisodicMemory::new();
    mem.insert_edge("Cluster_A1", "Cluster_A2");
    mem.insert_edge("Cluster_A2", "Gateway");
    mem.insert_edge("Gateway", "Cluster_B1");
    mem.insert_edge("Cluster_B1", "Cluster_B2");

    // Consulta con "Gateway" en avoid (corte inducido)
    let t0 = Instant::now();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Cluster_A1", "Cluster_B2", 50, &["Gateway"]);
    let search_us = t0.elapsed().as_micros();

    let closed_set = match &res.outcome {
        SearchOutcome::Unreachable { closed_set, .. } => closed_set,
        _ => panic!("Debe ser inalcanzable"),
    };

    let t1 = Instant::now();
    let res_valid = verify_negative(&mem, "Cluster_A1", "Cluster_B2", closed_set, &["Gateway"]);
    let verify_us = t1.elapsed().as_micros();

    println!("* Búsqueda BFS con corte inductivo: {} µs [MEDIDO]", search_us);
    println!("* Verificación formal del certificado inductivo S: {} µs [MEDIDO]", verify_us);
    println!("* Validez matemática del testigo negativo: {} [MEDIDO]", res_valid.is_ok());
    assert!(res_valid.is_ok(), "El certificado negativo debe ser matemáticamente válido");

    // Intento de ataque adversarial: manipular el corte S agregando el nodo destino t
    let mut tampered_set = closed_set.clone();
    tampered_set.push("Cluster_B2".to_string());
    let res_tampered = verify_negative(&mem, "Cluster_A1", "Cluster_B2", &tampered_set, &["Gateway"]);
    println!("* Detección y rechazo de certificado manipulado/falsificado: {} [MEDIDO]", res_tampered.is_err());
    assert!(res_tampered.is_err(), "Un certificado adulterado debe ser rechazado formalmente");
}

fn test_2_64_counterfactual_hypotheses_simultaneous_divergence() {
    println!("\n--- [PRUEBA 2] PARALELISMO DE 64 HIPÓTESIS CONTRAFACTUALES (ALU BITWISE u64) ---");
    let mut mem = EpisodicMemory::new();
    mem.insert_edge("Root", "Branch_0");
    mem.insert_edge("Root", "Branch_1");
    mem.insert_edge("Branch_0", "Mid_0");
    mem.insert_edge("Branch_1", "Mid_1");
    mem.insert_edge("Mid_0", "Target");
    mem.insert_edge("Mid_1", "Target");

    // En mundos pares Mid_0->Target activo; en mundos alternados Mid_1->Target activo
    let mask_mid0 = 0x5555_5555_5555_5555u64;
    let mask_mid1 = 0x3333_3333_3333_3333u64;
    mem.set_edge_world_mask("Mid_0", "Target", mask_mid0);
    mem.set_edge_world_mask("Mid_1", "Target", mask_mid1);

    let root_id = mem.interner.get_id("Root").unwrap();
    let target_id = mem.interner.get_id("Target").unwrap();

    let t0 = Instant::now();
    let res = solve_ms_bfs_counterfactual(&mem, root_id, target_id, 64);
    let us_cf = t0.elapsed().as_micros();

    let reachable_count = res.reachable_mask.count_ones();
    let unreachable_count = 64 - reachable_count;
    let expected_mask = mask_mid0 | mask_mid1;

    println!("* Tiempo de evaluación simultánea de 64 hipótesis contrafactuales: {} µs [MEDIDO]", us_cf);
    println!("* Mundos contrafactuales con ruta viable: {}/64 [MEDIDO]", reachable_count);
    println!("* Mundos contrafactuales con corte estricto: {}/64 [MEDIDO]", unreachable_count);
    println!("* Coincidencia exacta con álgebra booleana de mundos: {} [MEDIDO]", res.reachable_mask == expected_mask);
    assert_eq!(res.reachable_mask, expected_mask);
}

fn test_3_mquake_causal_replacement_vs_ghost_edges() {
    println!("\n--- [PRUEBA 3] REEMPLAZO ATÓMICO CAUSAL MQuAKE VS ARISTAS FANTASMA ---");
    let mut mem = EpisodicMemory::new();
    mem.insert_edge("París", "Francia");
    mem.insert_edge("Francia", "Europa");
    mem.insert_edge("Europa", "PlanetaTierra");

    let r1 = ReasoningEngine::solve_reachability_bfs(&mut mem, "París", "PlanetaTierra", 10);
    println!("* Consulta inicial París -> PlanetaTierra: Alcanzable = {} (distancia: {}) [MEDIDO]", r1.is_reachable, r1.path_length);
    assert!(r1.is_reachable);

    // Mutación estilo MQuAKE: Francia fue reasignada a un nuevo continente "NeoEuropa"
    mem.remove_edge("Francia", "Europa");
    mem.insert_edge("Francia", "NeoEuropa");
    mem.insert_edge("NeoEuropa", "ColoniaEspacial");

    let t0 = Instant::now();
    let r_old = ReasoningEngine::solve_reachability_bfs(&mut mem, "París", "PlanetaTierra", 10);
    let us_check = t0.elapsed().as_micros();

    let r_new = ReasoningEngine::solve_reachability_bfs(&mut mem, "París", "ColoniaEspacial", 10);

    println!("* Verificación arista fantasma París -> PlanetaTierra tras mutación: Alcanzable = {} (Tiempo: {} µs) [MEDIDO]", r_old.is_reachable, us_check);
    println!("* Consulta París -> ColoniaEspacial tras mutación: Alcanzable = {} (distancia: {}) [MEDIDO]", r_new.is_reachable, r_new.path_length);
    assert!(!r_old.is_reachable, "Cero aristas fantasma: el hecho reemplazado no debe filtrar caminos obsoletos");
    assert!(r_new.is_reachable, "La nueva ruta causal debe ser inmediatamente activa");
}

fn main() {
    println!("========================================================================");
    println!("  RELIC: SUITE DE DEMOSTRACIÓN DE ORIGINALIDAD Y CAPACIDADES SINGULARES");
    println!("========================================================================");

    test_1_epistemic_certificates_and_tamper_resistance();
    test_2_64_counterfactual_hypotheses_simultaneous_divergence();
    test_3_mquake_causal_replacement_vs_ghost_edges();

    println!("\n========================================================================");
    println!("  TODAS LAS PRUEBAS DE ORIGINALIDAD COMPLETADAS SATISFACTORIAMENTE [OK]");
    println!("========================================================================");
}
