//! Catálogo de herramientas MCP de RELIC y manejadores base (`interpret`, `verify`, `retrieve`).
//! Menos de 180 líneas.

use relic_blackwall::BlackwallGate;
use relic_cli::DispatchIntent;
use relic_core::RirDocumentV1;
use serde_json::{json, Value};
pub use crate::tools_blackwall::{
    handle_activate, handle_assimilate, handle_capsule, handle_consolidate, handle_diff,
    handle_propose, handle_rollback, handle_spectral, handle_synchronize,
};
use crate::types::{CallToolResult, ToolDefinition};

pub const VALID_MCP_TOOLS: &[&str] = &[
    "relic_interpret",
    "relic_verify",
    "relic_retrieve",
    "relic_propose",
    "relic_rollback",
    "relic_consolidate",
    "relic_activate",
    "relic_diff",
    "relic_spectral",
    "relic_synchronize",
    "relic_assimilate",
    "relic_capsule",
];

fn tool_def(name: &str, description: &str, input_schema: Value) -> ToolDefinition {
    ToolDefinition { name: name.to_string(), description: description.to_string(), input_schema }
}

pub fn list_tools() -> Vec<ToolDefinition> {
    vec![
        tool_def(
            "relic_interpret",
            "Interpreta lenguaje natural en español hacia RIR estructurado con estado epistemológico.",
            json!({ "type": "object", "properties": { "text": { "type": "string", "description": "Texto en lenguaje natural a interpretar" } }, "required": ["text"] }),
        ),
        tool_def(
            "relic_verify",
            "Valida semánticamente un documento RIR v1 contra invariantes de transitividad y aritmética.",
            json!({ "type": "object", "properties": { "rir": { "type": "object", "description": "Documento RIR v1 en formato JSON" } }, "required": ["rir"] }),
        ),
        tool_def(
            "relic_retrieve",
            "Recupera hechos, vecinos y alias en la memoria episódica estructurada.",
            json!({ "type": "object", "properties": { "entity": { "type": "string", "description": "Nombre de la entidad a consultar" } }, "required": ["entity"] }),
        ),
        tool_def(
            "relic_propose",
            "Propone una mutación léxica o relacional al BlackwallGate (Oráculo + Regresión Sandbox + Commit SHA-256).",
            json!({ "type": "object", "properties": { "payload": { "type": "object" }, "timestamp_ms": { "type": "integer" } }, "required": ["payload"] }),
        ),
        tool_def(
            "relic_rollback",
            "Revierte el estado activo hacia target_version mediante una cicatriz append-only sin borrar historia.",
            json!({ "type": "object", "properties": { "target_version": { "type": "integer" }, "reason": { "type": "string" }, "timestamp_ms": { "type": "integer" } }, "required": ["target_version", "reason"] }),
        ),
        tool_def(
            "relic_consolidate",
            "Consolida la memoria ('Sueño') podando episodios de bajo peso si la fidelidad cumple min_fidelity.",
            json!({ "type": "object", "properties": { "min_weight_threshold": { "type": "number" }, "min_fidelity": { "type": "number" }, "timestamp_ms": { "type": "integer" } }, "required": ["min_weight_threshold"] }),
        ),
        tool_def(
            "relic_activate",
            "Ejecuta Activación Propagada determinista con cálculo de entropía de Shannon y abstención por incertidumbre.",
            json!({ "type": "object", "properties": { "seeds": { "type": "array", "items": { "type": "string" } }, "gamma": { "type": "number" }, "steps": { "type": "integer" }, "max_entropy_threshold": { "type": "number" } }, "required": ["seeds"] }),
        ),
        tool_def(
            "relic_diff",
            "Calcula el diff estructurado entre dos versiones del Engrama y exporta grafos Mermaid y DOT.",
            json!({ "type": "object", "properties": { "v_from": { "type": "integer" }, "v_to": { "type": "integer" } }, "required": ["v_from", "v_to"] }),
        ),
        tool_def(
            "relic_spectral",
            "Ejecuta el verificador espectral (DFT exacta + Teorema de Donoho-Stark) o auditoría de alerta de red C2 + BFS.",
            json!({ "type": "object", "properties": { "signal": { "type": "array", "items": { "type": "number" } }, "support_epsilon": { "type": "number" }, "min_cycles": { "type": "number" }, "expected_period_hint": { "type": "number" }, "network_alert": { "type": "object" } } }),
        ),
        tool_def(
            "relic_synchronize",
            "Ejecuta dinámica de sincronización de osciladores acoplados de Kuramoto con abstención física si r < r*.",
            json!({ "type": "object", "properties": { "spec": { "type": "object" } }, "required": ["spec"] }),
        ),
        tool_def(
            "relic_assimilate",
            "Destilación en la Sombra: extrae y asimila un término de una consulta incompleta, auto-calibra fase y registra regresión.",
            json!({ "type": "object", "properties": { "raw_query": { "type": "string" }, "verified_op": { "type": "string" }, "timestamp_ms": { "type": "integer" } }, "required": ["raw_query", "verified_op"] }),
        ),
        tool_def(
            "relic_capsule",
            "Importa o exporta cápsulas portátiles .engram con verificación criptográfica SHA-256 y Sandbox de Regresión.",
            json!({ "type": "object", "properties": { "action": { "type": "string" }, "domain": { "type": "string" }, "v_from": { "type": "integer" }, "v_to": { "type": "integer" }, "capsule": { "type": "object" } } }),
        ),
    ]
}

pub fn handle_interpret(args: &Value, blackwall: &mut BlackwallGate) -> CallToolResult {
    let text = match args.get("text").and_then(|v| v.as_str()) {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => return CallToolResult::err("Error: Parámetro 'text' faltante o vacío."),
    };
    let (intent, answer) = blackwall.evaluate_query(text);
    let status_str = match answer.status.as_str() {
        "VALID" => "VALID",
        "INCOMPLETE" => "INCOMPLETE",
        "AMBIGUOUS" => "AMBIGUOUS",
        _ => "UNSUPPORTED",
    };
    let summary = format!(
        "Entendí: {}\nEstado: {}\nRuta: {}\nRespuesta Verificada: {}",
        match &intent {
            DispatchIntent::Arithmetic(_) => "Operación de inventario / aritmética determinista.",
            DispatchIntent::GraphReachability(_) => "Consulta de alcanzabilidad o relaciones en grafo.",
            DispatchIntent::RelationalReasoning(_) => "Razonamiento relacional transitivo / comparativo.",
            DispatchIntent::Incomplete { reason, .. } => reason.as_str(),
            DispatchIntent::FreeLanguage(_) => "Lenguaje libre no determinista o fuera de dominio.",
        },
        status_str,
        answer.route,
        answer.text
    );
    CallToolResult::ok(summary)
}

pub fn handle_verify(args: &Value) -> CallToolResult {
    let rir_val = match args.get("rir") {
        Some(v) if v.is_object() => v,
        _ => return CallToolResult::err("Error: Parámetro 'rir' faltante o no es un objeto JSON."),
    };
    let doc = match RirDocumentV1::from_json_str(&rir_val.to_string()) {
        Ok(d) => d,
        Err(e) => return CallToolResult::err(format!("Error de sintaxis RIR: {e}")),
    };
    match doc.validate_semantic() {
        Ok(()) => CallToolResult::ok(format!("Verificación: VÁLIDA\nEstado: VALID\nIntento: {:?}\nDominio: {:?}", doc.intent, doc.domain)),
        Err(err) => CallToolResult::ok(format!("Verificación: RECHAZADA\nEstado: INCOMPLETE\nDetalle: {err}")),
    }
}

pub fn handle_retrieve(args: &Value, blackwall: &mut BlackwallGate) -> CallToolResult {
    let entity = match args.get("entity").and_then(|v| v.as_str()) {
        Some(e) if !e.trim().is_empty() => e.trim(),
        _ => return CallToolResult::err("Error: Parámetro 'entity' faltante o vacío."),
    };
    let neighbors = blackwall.retrieve_neighbors(entity);
    if neighbors.is_empty() {
        CallToolResult::ok(format!(
            "Memoria Episódica: '{entity}'\nEstado: INCOMPLETE\nDetalle: No se encontraron relaciones para esta entidad."
        ))
    } else {
        let relations: Vec<String> = neighbors.iter().map(|dest| format!("--> {dest}")).collect();
        CallToolResult::ok(format!(
            "Memoria Episódica: '{entity}'\nEstado: VALID\nVecinos ({0}/{0}):\n{1}",
            neighbors.len(),
            relations.join("\n")
        ))
    }
}
