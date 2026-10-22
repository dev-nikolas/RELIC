//! Banco de Falsación y Evaluación Semántica de Roles Composicionales (Opción 1).
//!
//! Separa explícitamente:
//! 1. Conjunto de Desarrollo: Verifica la extracción granular de roles (polaridad, dimensión, negación).
//! 2. Conjunto de Evaluación Ciega / Reservada: Paráfrasis, entidades y órdenes no utilizados en la implementación.

use relic_cli::relational::{
    extract_semantic_role, parse_relational_query, InterpretationStatus, Polarity,
};

// ============================================================================
// CONJUNTO 1: DESARROLLO (Verificación Estructural Intermedia)
// ============================================================================

#[test]
fn test_dev_role_extraction_affirmative_greater() {
    let role = extract_semantic_role("Pedro es más alto que Juan").expect("Debe extraer");
    assert_eq!(role.left_entity, "Pedro");
    assert_eq!(role.right_entity, "Juan");
    assert_eq!(role.dimension.as_deref(), Some("altura"));
    assert_eq!(role.polarity, Polarity::Greater);
    assert!(!role.is_negated);
    assert_eq!(role.status, InterpretationStatus::Valid);

    // Verificación de normalización: (Pedro, altura_mayor, Juan)
    assert_eq!(
        role.to_normalized_edge(),
        Some(("Pedro".to_string(), "altura_mayor".to_string(), "Juan".to_string()))
    );
}

#[test]
fn test_dev_role_extraction_inverse_polarity() {
    let role = extract_semantic_role("Juan es más bajo que Pedro").expect("Debe extraer");
    assert_eq!(role.left_entity, "Juan");
    assert_eq!(role.right_entity, "Pedro");
    assert_eq!(role.dimension.as_deref(), Some("altura"));
    assert_eq!(role.polarity, Polarity::Lesser);
    assert_eq!(role.status, InterpretationStatus::Valid);

    // Debe invertir automáticamente la arista: (Pedro, altura_mayor, Juan)
    assert_eq!(
        role.to_normalized_edge(),
        Some(("Pedro".to_string(), "altura_mayor".to_string(), "Juan".to_string()))
    );
}

#[test]
fn test_dev_role_extraction_negation_does_not_create_edge() {
    let role = extract_semantic_role("Pedro no es más alto que Juan").expect("Debe extraer");
    assert!(role.is_negated);
    assert_eq!(role.status, InterpretationStatus::Negated);

    // Distinción lógica crítica: NO debe generar arista afirmativa ni directa ni inversa
    assert_eq!(role.to_normalized_edge(), None);
}

#[test]
fn test_dev_role_extraction_ambiguous_dimension() {
    let role = extract_semantic_role("Pedro supera a Juan").expect("Debe extraer");
    assert_eq!(role.dimension, None);
    assert_eq!(role.status, InterpretationStatus::AmbiguousDimension);

    // Sin dimensión conocida, la arista no puede ser normalizada
    assert_eq!(role.to_normalized_edge(), None);
}

// ============================================================================
// CONJUNTO 2: EVALUACIÓN CIEGA / RESERVADA (Falsación de Generalización)
// ============================================================================

#[test]
fn test_blind_eval_case_1_paraphrase_aventaja_altura() {
    // Caso 1: Paráfrasis con entidades nunca vistas
    let q = "Si Mateo aventaja a Tomás en altura y Tomás supera a Lucas en estatura, ¿quién es el más alto?";
    let spec = parse_relational_query(q).expect("Debe reconocer paráfrasis de ventaja de altura");
    assert_eq!(
        spec.graph.find_maximal("altura_mayor"),
        Some("Mateo".to_string()),
        "El elemento maximal debe ser Mateo"
    );
    assert!(spec.graph.is_transitively_related("Mateo", "Lucas", "altura_mayor"));
}

#[test]
fn test_blind_eval_case_2_inverse_polarity_transitive() {
    // Caso 2: Inversión en cascada (Lucía más baja que Sofía; Sofía más baja que Elena)
    // Orden real: Elena > Sofía > Lucía -> Maximal: Elena
    let q = "Lucía es más baja que Sofía. Sofía tiene menor altura que Elena. ¿Quién tiene mayor estatura?";
    let spec = parse_relational_query(q).expect("Debe componer relaciones inversas");
    println!("SPEC GRAPH NODES: {:?}, EDGES: {:?}", spec.graph.node_count(), spec.graph);
    assert_eq!(
        spec.graph.find_maximal("altura_mayor"),
        Some("Elena".to_string()),
        "Inversión de polaridad debe coronar a Elena como la más alta"
    );
    assert!(spec.graph.is_transitively_related("Elena", "Lucía", "altura_mayor"));
}

#[test]
fn test_blind_eval_case_3_free_syntactic_order() {
    // Caso 3: Orden sintáctico no estándar "Más alto que X es Y"
    let q = "Más alto que Juan es Pedro. Pedro tiene menor altura que Andrés. ¿Quién es el más alto?";
    let spec = parse_relational_query(q).expect("Debe procesar orden sintáctico libre");
    assert_eq!(
        spec.graph.find_maximal("altura_mayor"),
        Some("Andrés".to_string()),
        "Andrés debe ser el maximal independientemente del orden superficial"
    );
}

#[test]
fn test_blind_eval_case_4_negation_rejection_in_query() {
    // Caso 4: Una premisa negada NO debe completar la cadena de transitividad
    let q = "Pedro no es más alto que Juan. Juan es más alto que Diego. ¿Quién es el más alto?";
    let spec = parse_relational_query(q);
    // Como Pedro no es más alto no genera arista, solo existe la arista Juan -> Diego.
    // Pedro no entra en el grafo, impidiendo falsos positivos sobre Pedro.
    if let Some(s) = spec {
        assert_ne!(
            s.graph.find_maximal("altura_mayor"),
            Some("Pedro".to_string()),
            "Pedro jamás debe ser inferido como más alto bajo premisa negada"
        );
    }
}

#[test]
fn test_blind_eval_case_5_composition_4_entities_mixed() {
    // Caso 5: 4 entidades con paráfrasis mixtas
    // Gabriel > Rafael > Daniel > Samuel
    let q = "Gabriel supera a Rafael en altura. Daniel es más bajo que Rafael. Daniel aventaja a Samuel en estatura. ¿Quién es el más alto?";
    let spec = parse_relational_query(q).expect("Debe soportar composición con mezcla de polaridades");
    assert_eq!(
        spec.graph.find_maximal("altura_mayor"),
        Some("Gabriel".to_string())
    );
    assert!(spec.graph.is_transitively_related("Gabriel", "Samuel", "altura_mayor"));
}

#[test]
fn test_blind_eval_case_6_contradiction_detection() {
    // Caso 6: Premisas contradictorias incompatibles con orden estricto (A > B y B > A)
    let q = "Pedro es más alto que Juan. Juan es más alto que Pedro. ¿Quién es el más alto?";
    let spec = parse_relational_query(q).expect("Debe procesar la sintaxis de las premisas");
    assert!(spec.graph.has_cycle("altura_mayor"), "Debe detectar el ciclo contradictorio");
    assert_eq!(spec.graph.find_maximal("altura_mayor"), None, "No debe producir máximo en grafo cíclico");
}

#[test]
fn test_blind_eval_case_7_disconnected_chains_ambiguity() {
    // Caso 7: Dos cadenas desconectadas independientes (A > B y C > D).
    // Existen dos máximos locales (A y C), por lo que no hay un único máximo global.
    let q = "Pedro es más alto que Juan. Carlos es más alto que Diego. ¿Quién es el más alto?";
    let spec = parse_relational_query(q).expect("Debe procesar las dos cadenas");
    assert_eq!(spec.graph.find_maximal("altura_mayor"), None, "Debe abstenerse al haber dos máximos incomparables");
}

#[test]
fn test_blind_eval_case_8_indirect_contradiction_cycle() {
    // Caso 8: Contradicción indirecta en 3 pasos (A > B, B > C, C > A)
    // Ninguna pareja directa está repetida; solo se detecta mediante cierre topológico cíclico
    let q = "Marcos es más alto que Lucas. Lucas es más alto que Mateo. Mateo es más alto que Marcos. ¿Quién es el más alto?";
    let spec = parse_relational_query(q).expect("Debe procesar la sintaxis de las 3 premisas");
    assert!(spec.graph.has_cycle("altura_mayor"), "Debe detectar el ciclo indirecto Marcos -> Lucas -> Mateo -> Marcos");
    assert_eq!(spec.graph.find_maximal("altura_mayor"), None, "No debe producir ningún máximo bajo ciclo indirecto");
}

#[test]
fn test_blind_eval_case_9_mixed_dimensions_isolation() {
    // Caso 9: Dimensiones mezcladas (A supera a B en fuerza; B supera a C en altura).
    // NO debe mezclar fuerza con altura para inferir que A es más alto que C
    let q = "Marcos supera a Lucas en fuerza. Lucas supera a Mateo en altura. ¿Quién es el más alto?";
    let spec = parse_relational_query(q).expect("Debe procesar las premisas");
    assert!(!spec.graph.is_transitively_related("Marcos", "Mateo", "altura_mayor"), "Fuerza y altura no deben combinarse en la misma relación");
    assert_eq!(spec.graph.find_maximal("altura_mayor"), None, "Debe abstenerse de coronar un máximo global al existir entidades con dimensiones mezcladas sin datos de altura");
}
