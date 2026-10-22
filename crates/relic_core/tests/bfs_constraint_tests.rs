use relic_core::episodic_memory::EpisodicMemory;
use relic_core::reasoning_engine::ReasoningEngine;

fn build_sample_graph() -> EpisodicMemory {
    let mut memory = EpisodicMemory::new();
    // Path 1: Madrid -> Barcelona -> Paris
    memory.insert_edge("Madrid", "Barcelona");
    memory.insert_edge("Barcelona", "Paris");
    // Path 2: Madrid -> Valencia -> Lyon -> Paris
    memory.insert_edge("Madrid", "Valencia");
    memory.insert_edge("Valencia", "Lyon");
    memory.insert_edge("Lyon", "Paris");
    // Disconnected / Dead ends
    memory.insert_edge("Madrid", "Toledo");
    memory.insert_edge("Berlin", "Warsaw");
    memory
}

#[test]
fn test_case_1_alternative_route_when_avoided() {
    let mut mem = build_sample_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Paris", 10, &["Barcelona"]);
    assert!(res.is_reachable, "Debe encontrar la ruta alternativa vía Valencia/Lyon");
    assert_eq!(res.path, vec!["Madrid", "Valencia", "Lyon", "Paris"]);
    assert!(!res.path.contains(&"Barcelona".to_string()), "La ruta no debe contener Barcelona");
}

#[test]
fn test_case_2_all_routes_blocked_by_avoid() {
    let mut mem = EpisodicMemory::new();
    mem.insert_edge("Madrid", "Barcelona");
    mem.insert_edge("Barcelona", "Paris");
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Paris", 10, &["Barcelona"]);
    assert!(!res.is_reachable, "No debe ser alcanzable si la única ruta pasa por Barcelona");
    assert!(res.path.is_empty());
}

#[test]
fn test_case_3_origin_node_is_avoided() {
    let mut mem = build_sample_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Paris", 10, &["Madrid"]);
    assert!(!res.is_reachable, "Si el origen está prohibido, la búsqueda debe fallar inmediatamente");
    assert!(res.path.is_empty());
}

#[test]
fn test_case_4_destination_node_is_avoided() {
    let mut mem = build_sample_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Paris", 10, &["Paris"]);
    assert!(!res.is_reachable, "Si el destino está prohibido, la búsqueda debe fallar inmediatamente");
    assert!(res.path.is_empty());
}

#[test]
fn test_case_5_unconstrained_query_preserves_shortest_path() {
    let mut mem = build_sample_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Paris", 10, &[]);
    assert!(res.is_reachable);
    assert_eq!(res.path, vec!["Madrid", "Barcelona", "Paris"], "Sin restricciones, debe tomar el camino más corto (2 saltos)");
}

#[test]
fn test_case_6_direct_route_not_crossing_avoided() {
    let mut mem = build_sample_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Toledo", 10, &["Barcelona", "Paris"]);
    assert!(res.is_reachable);
    assert_eq!(res.path, vec!["Madrid", "Toledo"]);
}

#[test]
fn test_case_7_multiple_avoided_nodes() {
    let mut mem = build_sample_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Paris", 10, &["Barcelona", "Valencia"]);
    assert!(!res.is_reachable, "Bloqueando ambas rutas (Barcelona y Valencia), no hay camino admisible");
}

#[test]
fn test_case_8_avoid_node_not_in_graph() {
    let mut mem = build_sample_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Paris", 10, &["Atlantida"]);
    assert!(res.is_reachable, "Restricción sobre nodo no existente en el camino no afecta rutas válidas");
    assert_eq!(res.path, vec!["Madrid", "Barcelona", "Paris"]);
}

#[test]
fn test_case_9_disconnected_nodes_in_graph() {
    let mut mem = build_sample_graph();
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "Madrid", "Warsaw", 10, &[]);
    assert!(!res.is_reachable, "No existe conexión entre Madrid y Varsovia en el grafo");
}

#[test]
fn test_case_10_cycle_with_avoid() {
    let mut mem = EpisodicMemory::new();
    mem.insert_edge("A", "B");
    mem.insert_edge("B", "A");
    mem.insert_edge("B", "C");
    mem.insert_edge("C", "D");
    let res = ReasoningEngine::solve_reachability_bfs_constrained(&mut mem, "A", "D", 10, &["C"]);
    assert!(!res.is_reachable, "El ciclo A-B no debe colapsar y al bloquear C no debe llegar a D");
}
