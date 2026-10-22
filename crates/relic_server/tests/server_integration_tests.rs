//! Pruebas de integración de motor y endpoints de relic_server.
//! Menos de 180 líneas.

use relic_server::conversations::{add_message_and_respond, create_conversation, get_conversation, list_conversations};
use relic_server::engine::execute_query;
use relic_server::state::ServerState;

#[test]
fn test_execute_query_simple_arithmetic() {
    let state = ServerState::new();
    let res = execute_query("1 + 1", &state);

    assert_eq!(res.status, "VALID");
    assert_eq!(res.route, "deterministic_arithmetic");
    assert!(res.text.contains("2"), "La respuesta debe contener 2, recibido: {}", res.text);
    assert!(res.latency_ms >= 0.0);
    assert!(res.rss_mb > 0.0);
}

#[test]
fn test_execute_query_graph_reachability() {
    let state = ServerState::new();
    let res = execute_query("¿Depende el crate relic_cli del paquete memmap2?", &state);

    assert_eq!(res.status, "VALID");
    assert_eq!(res.route, "deterministic_graph");
    assert!(res.text.contains("memmap2") || res.text.contains("relic_cli"));
}

#[test]
fn test_execute_query_incomplete_epistemic_rejection() {
    let state = ServerState::new();
    let res = execute_query("tengo algunas manzanas y luego compro 5", &state);

    assert_eq!(res.status, "INCOMPLETE");
    assert_eq!(res.route, "epistemic_rejection");
    assert!(res.text.contains("desconoce la cantidad inicial") || res.text.contains("incompleta"));
}

#[test]
fn test_execute_query_empty_rejection() {
    let state = ServerState::new();
    let res = execute_query("   ", &state);

    assert_eq!(res.status, "INVALID");
    assert_eq!(res.route, "validation_error");
}

#[test]
fn test_execute_query_free_language_degraded_or_neural() {
    let state = ServerState::new();
    let res = execute_query("Explica brevemente la teoría del universo", &state);

    assert_eq!(res.status, "VALID");
    assert!(res.route == "neural_model" || res.route == "degraded_mode");
}

#[test]
fn test_conversation_lifecycle_and_assistant_reply() {
    let state = ServerState::new();

    // 1. Crear conversación
    let conv = create_conversation(Some("Test Chat".to_string()), &state);
    assert_eq!(conv.title, "Test Chat");
    assert_eq!(conv.messages.len(), 0);

    // 2. Enviar mensaje del usuario con operación aritmética
    let reply = add_message_and_respond(&conv.id, "cuánto es 5 * 4", &state).expect("Debe generar respuesta");
    assert_eq!(reply.role, "assistant");
    assert!(reply.content.contains("20"), "Debe resolver 5 * 4 = 20, recibido: {}", reply.content);

    // 3. Verificar que la conversación ahora contiene 2 mensajes (usuario + asistente)
    let fetched = get_conversation(&conv.id, &state).expect("Conversación debe existir");
    assert_eq!(fetched.messages.len(), 2);
    assert_eq!(fetched.messages[0].role, "user");
    assert_eq!(fetched.messages[0].content, "cuánto es 5 * 4");
    assert_eq!(fetched.messages[1].role, "assistant");
    assert!(fetched.messages[1].content.contains("20"));

    // 4. Listar conversaciones
    let list = list_conversations(&state);
    assert!(!list.is_empty());
}
