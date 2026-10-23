//! Transparencia de interpretación ("Entendí: 3 - 10 = -7") y abstención honesta
//! ante ambigüedades de alias y entidades fuera de la memoria. Menos de 180 líneas.

use relic_cli::answer::answer_intent;
use relic_cli::dispatch::{parse_dispatch_intent, DispatchIntent, QuerySpec};
use relic_cli::knowledge::load_default_knowledge;
use relic_core::EpisodicMemory;

fn answer(query: &str) -> relic_cli::Answer {
    answer_default_intent(&parse_dispatch_intent(query))
}

fn answer_default_intent(intent: &DispatchIntent) -> relic_cli::Answer {
    let mut mem = EpisodicMemory::new();
    load_default_knowledge(&mut mem);
    answer_intent(intent, &mut mem)
}

fn ambiguous_memory() -> EpisodicMemory {
    let mut mem = EpisodicMemory::new();
    load_default_knowledge(&mut mem);
    // 'Bogota' (sin tilde) colisiona canónicamente con 'Bogotá' ya registrada.
    mem.insert_edge("Bogota", "Medellín");
    mem
}

#[test]
fn test_negative_result_declares_its_interpretation() {
    // "resta 3 y 10" es ambiguo en lenguaje natural: la lectura elegida debe mostrarse.
    let a = answer("resta 3 y 10");
    assert_eq!(a.route, "deterministic_arithmetic");
    assert_eq!(a.status, "VALID");
    assert_eq!(a.text, "Entendí: 3 - 10 = -7. El resultado es -7.");
}

#[test]
fn test_simple_expression_formula_matches_input_pair() {
    let a = answer("cuánto es 5 - 9");
    assert_eq!(a.text, "Entendí: 5 - 9 = -4. El resultado es -4.");
    let b = answer("suma 8 e 11");
    assert_eq!(b.text, "Entendí: 8 + 11 = 19. El resultado es 19.");
}

#[test]
fn test_inventory_declares_units_formula() {
    let a = answer("tengo 12 lápices y regalo 7");
    assert_eq!(a.text, "Entendí: 12 lápices - 7 lápices. Ahora tienes 5 lápices.");
}

#[test]
fn test_graph_alias_collision_abstains_as_ambiguous() {
    let spec = QuerySpec { initial: Some("bogota".to_string()), target: Some("Lima".to_string()), via: None, avoid: None, is_negated: false };
    let mut mem = ambiguous_memory();
    let a = answer_intent(&DispatchIntent::GraphReachability(spec), &mut mem);
    assert_eq!(a.status, "AMBIGUOUS");
    assert!(a.text.contains("Consulta ambigua"), "Respuesta real: {}", a.text);
    assert!(a.text.contains("Bogotá") && a.text.contains("Bogota"));
}

#[test]
fn test_graph_unknown_entity_is_invalid_not_fake_route() {
    let spec = QuerySpec { initial: Some("relic_cli".to_string()), target: Some("kodex".to_string()), via: None, avoid: None, is_negated: false };
    let mut mem = EpisodicMemory::new();
    load_default_knowledge(&mut mem);
    let a = answer_intent(&DispatchIntent::GraphReachability(spec), &mut mem);
    assert_eq!(a.status, "INVALID");
    assert!(a.text.contains("no existe en la memoria episódica"), "Respuesta real: {}", a.text);
}

#[test]
fn test_graph_ambiguous_via_or_avoid_abstains_silently_never() {
    // Evitar una entidad ambigua no debe degradarse a BFS sin restricciones en silencio.
    let spec = QuerySpec { initial: Some("Madrid".to_string()), target: Some("Barcelona".to_string()), via: None, avoid: Some("bogota".to_string()), is_negated: false };
    let mut mem = ambiguous_memory();
    let a = answer_intent(&DispatchIntent::GraphReachability(spec), &mut mem);
    assert_eq!(a.status, "AMBIGUOUS");
    assert!(a.text.contains("restringida"), "Debe señalar el rol 'restringida': {}", a.text);

    // Via ambigua tampoco se ignora.
    let spec2 = QuerySpec { initial: Some("Madrid".to_string()), target: Some("Paris".to_string()), via: Some("bogota".to_string()), avoid: None, is_negated: false };
    let mut mem2 = ambiguous_memory();
    let a2 = answer_intent(&DispatchIntent::GraphReachability(spec2), &mut mem2);
    assert_eq!(a2.status, "AMBIGUOUS");
    assert!(a2.text.contains("de paso"), "Debe señalar el rol 'de paso': {}", a2.text);
}

#[test]
fn test_nested_expression_falls_back_to_evaluated_value() {
    let a = answer("cuánto es (10 + 5) * 2");
    assert_eq!(a.route, "deterministic_arithmetic");
    assert!(a.text.contains("30"), "Respuesta real: {}", a.text);
}
