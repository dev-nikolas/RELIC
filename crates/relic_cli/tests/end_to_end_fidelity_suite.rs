//! Suite de Fidelidad de Extremo a Extremo (E2E Fidelity & Failure Decoupling).
//! Evalúa: (1) Texto, (2) Aristas extraídas, (3) Cláusulas sin resolver, (4) Conclusión, (5) Derivabilidad.
//! Menos de 180 líneas.

use relic_cli::relational::{parse_relational_query, RelationalQuestionType};

#[derive(Debug, PartialEq, Eq)]
enum FailureClass {
    ParserMisinterpretation,
    EngineReasoningFault,
}

fn assert_e2e_fidelity(
    text: &str,
    expected_edges: &[(&str, &str, &str)],
    expected_unresolved_count: usize,
    expected_cycle: bool,
    expected_maximal: Option<&str>,
    expected_partial: bool,
) {
    // 1. Análisis del texto y extracción (Parser)
    let spec = parse_relational_query(text).expect("El texto debe contener una consulta relacional parseable");

    // 2. Comprobación de relaciones extraídas
    let actual_edges: Vec<(&str, &str, &str)> = spec.graph.edges().iter()
        .map(|e| (e.source.as_str(), e.relation.as_str(), e.target.as_str()))
        .collect();
    for exp in expected_edges {
        if !actual_edges.contains(exp) {
            panic!("[{:?}] Arista esperada {:?} no extraída por el parser. Aristas reales: {:?}",
                FailureClass::ParserMisinterpretation, exp, actual_edges);
        }
    }
    assert_eq!(actual_edges.len(), expected_edges.len(),
        "[{:?}] Discrepancia en cantidad de relaciones extraídas", FailureClass::ParserMisinterpretation);

    // 3. Comprobación de información rechazada o sin resolver
    assert_eq!(spec.unparsed_clauses.len(), expected_unresolved_count,
        "[{:?}] Discrepancia en cláusulas sin resolver", FailureClass::ParserMisinterpretation);

    // 4. Conclusión del motor (Kahn y búsqueda maximal)
    let has_cycle = spec.graph.has_cycle("altura_mayor");
    assert_eq!(has_cycle, expected_cycle,
        "[{:?}] Detección de ciclo inconsistente con premisas", FailureClass::EngineReasoningFault);

    let conclusion = match &spec.question_type {
        RelationalQuestionType::Maximal(r) => spec.graph.find_maximal(r),
    };
    assert_eq!(conclusion.as_deref(), expected_maximal,
        "[{:?}] La conclusión del motor no coincide con lo derivable de las premisas", FailureClass::EngineReasoningFault);

    // 5. Estado final: debe advertir parcialidad si quedaron cláusulas sin resolver
    let is_partial = !spec.unparsed_clauses.is_empty();
    assert_eq!(is_partial, expected_partial, "El estado final debe reflejar si la interpretación fue parcial");
}

#[test]
fn test_e2e_fidelity_complete_and_sound() {
    // Caso 1: Interpretación completa, acíclica, 100% derivable.
    let text = "Mateo es más alto que Lucas. Lucas es más alto que Tomás. ¿Quién es el más alto?";
    assert_e2e_fidelity(
        text,
        &[("Mateo", "altura_mayor", "Lucas"), ("Lucas", "altura_mayor", "Tomás")],
        0, // 0 no resueltas (completo)
        false, // sin ciclo
        Some("Mateo"),
        false, // no es parcial
    );
}

#[test]
fn test_e2e_fidelity_partial_interpretation_preserves_validity() {
    // Caso 2: 3 cláusulas: 2 de comparación válidas y 1 no interpretable.
    // RELIC no debe asumir que interpretó todo ni fallar completamente: produce Mateo con advertencia parcial.
    let text = "Mateo es más alto que Lucas. Lucas es más alto que Tomás. El clima estaba frío ayer. ¿Quién es el más alto?";
    assert_e2e_fidelity(
        text,
        &[("Mateo", "altura_mayor", "Lucas"), ("Lucas", "altura_mayor", "Tomás")],
        1, // 1 cláusula no interpretable ("El clima estaba frío ayer")
        false,
        Some("Mateo"),
        true, // estado final debe ser PARCIAL
    );
}

#[test]
fn test_e2e_fidelity_negation_not_inverted_into_positive_edge() {
    // Caso 3: Negación no debe transformarse en menor ni en mayor.
    // "Lucas no es más alto que Mateo" -> No genera arista positiva Mateo > Lucas ni Lucas > Mateo.
    let text = "Lucas no es más alto que Mateo. Mateo es más alto que Tomás. ¿Quién es el más alto?";
    assert_e2e_fidelity(
        text,
        &[("Mateo", "altura_mayor", "Tomás")],
        1, // "Lucas no es más alto que Mateo" registrada como no asertada
        false,
        Some("Mateo"), // Mateo es el maximal de las premisas interpretadas
        true, // Parcial
    );
}

#[test]
fn test_e2e_fidelity_cycle_contradiction_engine_abstained() {
    // Caso 4: Contradicción directa A > B y B > A.
    let text = "Alberto es más alto que Bernardo. Bernardo es más alto que Alberto. ¿Quién es el más alto?";
    assert_e2e_fidelity(
        text,
        &[("Alberto", "altura_mayor", "Bernardo"), ("Bernardo", "altura_mayor", "Alberto")],
        0,
        true, // Ciclo detectado
        None, // Abstención por contradicción formal
        false,
    );
}

#[test]
fn test_e2e_fidelity_incomparable_components_abstention() {
    // Caso 5: Cadenas independientes (Andrés > Boris) y (Camilo > David).
    // Dos máximos locales -> Abstención fundamentada de máximo global.
    let text = "Andrés es más alto que Boris. Camilo es más alto que David. ¿Quién es el más alto?";
    assert_e2e_fidelity(
        text,
        &[("Andrés", "altura_mayor", "Boris"), ("Camilo", "altura_mayor", "David")],
        0,
        false,
        None, // Ningún maximal domina a todos los nodos
        false,
    );
}

#[test]
fn test_e2e_fidelity_mixed_dimensions_abstention() {
    // Caso 6: Dimensiones mezcladas (Fuerza vs Altura).
    // Andrés supera a Boris en fuerza. Boris es más alto que Camilo.
    let text = "Andrés supera a Boris en fuerza. Boris es más alto que Camilo. ¿Quién es el más alto?";
    assert_e2e_fidelity(
        text,
        &[("Andrés", "fuerza_mayor", "Boris"), ("Boris", "altura_mayor", "Camilo")],
        0,
        false,
        None, // Andrés carece de arista en altura -> Boris no domina a Andrés -> Abstención
        false,
    );
}
