//! Implementación de las 8 herramientas de RELIC MCP bajo custodia de `BlackwallGate`.
//! Menos de 180 líneas.

use relic_blackwall::BlackwallGate;
use relic_cli::DispatchIntent;
use relic_core::RirDocumentV1;
use relic_lexicon::{to_dot_diff, to_mermaid_diff, LexiconPayload};
use serde_json::{json, Value};
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
];

pub fn list_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "relic_interpret".to_string(),
            description: "Interpreta lenguaje natural en español hacia RIR estructurado con estado epistemológico.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": { "text": { "type": "string", "description": "Texto en lenguaje natural a interpretar" } },
                "required": ["text"]
            }),
        },
        ToolDefinition {
            name: "relic_verify".to_string(),
            description: "Valida semánticamente un documento RIR v1 contra invariantes de transitividad y aritmética.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": { "rir": { "type": "object", "description": "Documento RIR v1 en formato JSON" } },
                "required": ["rir"]
            }),
        },
        ToolDefinition {
            name: "relic_retrieve".to_string(),
            description: "Recupera hechos, vecinos y alias en la memoria episódica estructurada.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": { "entity": { "type": "string", "description": "Nombre de la entidad a consultar" } },
                "required": ["entity"]
            }),
        },
        ToolDefinition {
            name: "relic_propose".to_string(),
            description: "Propone una mutación léxica o relacional al BlackwallGate (Oráculo + Regresión Sandbox + Commit SHA-256).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "payload": { "type": "object", "description": "LexiconPayload serializado (graph_fact, op_alias, phrase_macro, entity_alias)" },
                    "timestamp_ms": { "type": "integer" }
                },
                "required": ["payload"]
            }),
        },
        ToolDefinition {
            name: "relic_rollback".to_string(),
            description: "Revierte el estado activo hacia target_version mediante una cicatriz append-only sin borrar historia.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target_version": { "type": "integer" },
                    "reason": { "type": "string" },
                    "timestamp_ms": { "type": "integer" }
                },
                "required": ["target_version", "reason"]
            }),
        },
        ToolDefinition {
            name: "relic_consolidate".to_string(),
            description: "Consolida la memoria ('Sueño') podando episodios de bajo peso si la fidelidad cumple min_fidelity.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "min_weight_threshold": { "type": "number" },
                    "min_fidelity": { "type": "number" },
                    "timestamp_ms": { "type": "integer" }
                },
                "required": ["min_weight_threshold"]
            }),
        },
        ToolDefinition {
            name: "relic_activate".to_string(),
            description: "Ejecuta Activación Propagada determinista con cálculo de entropía de Shannon y abstención por incertidumbre.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "seeds": { "type": "array", "items": { "type": "string" } },
                    "gamma": { "type": "number" },
                    "steps": { "type": "integer" },
                    "max_entropy_threshold": { "type": "number" }
                },
                "required": ["seeds"]
            }),
        },
        ToolDefinition {
            name: "relic_diff".to_string(),
            description: "Calcula el diff estructurado entre dos versiones del Engrama y exporta grafos Mermaid y DOT.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "v_from": { "type": "integer" },
                    "v_to": { "type": "integer" }
                },
                "required": ["v_from", "v_to"]
            }),
        },
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
        Ok(()) => CallToolResult::ok(format!(
            "Verificación: VÁLIDA\nEstado: VALID\nIntento: {:?}\nDominio: {:?}",
            doc.intent, doc.domain
        )),
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
            "Memoria Episódica: '{}'\nEstado: INCOMPLETE\nDetalle: No se encontraron relaciones para esta entidad.",
            entity
        ))
    } else {
        let relations: Vec<String> = neighbors.iter().map(|dest| format!("--> {dest}")).collect();
        CallToolResult::ok(format!(
            "Memoria Episódica: '{}'\nEstado: VALID\nVecinos ({}/{}):\n{}",
            entity,
            neighbors.len(),
            neighbors.len(),
            relations.join("\n")
        ))
    }
}

pub fn handle_propose(args: &Value, blackwall: &mut BlackwallGate) -> CallToolResult {
    let Some(payload_val) = args.get("payload") else {
        return CallToolResult::err("Error: Falta parámetro 'payload'.");
    };
    let payload: LexiconPayload = match serde_json::from_value(payload_val.clone()) {
        Ok(p) => p,
        Err(e) => return CallToolResult::err(format!("Error en 'payload': {e}")),
    };
    let ts = args.get("timestamp_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let outcome = blackwall.propose_mutation(payload, ts);
    CallToolResult::ok(serde_json::to_string(&outcome).unwrap_or_default())
}

pub fn handle_rollback(args: &Value, blackwall: &mut BlackwallGate) -> CallToolResult {
    let Some(target_version) = args.get("target_version").and_then(|v| v.as_u64()) else {
        return CallToolResult::err("Error: Falta 'target_version'.");
    };
    let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("Rollback MCP");
    let ts = args.get("timestamp_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let outcome = blackwall.rollback_to(target_version, reason, ts);
    CallToolResult::ok(serde_json::to_string(&outcome).unwrap_or_default())
}

pub fn handle_consolidate(args: &Value, blackwall: &mut BlackwallGate) -> CallToolResult {
    let Some(min_w) = args.get("min_weight_threshold").and_then(|v| v.as_f64()) else {
        return CallToolResult::err("Error: Falta 'min_weight_threshold'.");
    };
    let min_f = args.get("min_fidelity").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let ts = args.get("timestamp_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let outcome = blackwall.consolidate_memory(min_w, min_f, ts);
    CallToolResult::ok(serde_json::to_string(&outcome).unwrap_or_default())
}

pub fn handle_activate(args: &Value, blackwall: &BlackwallGate) -> CallToolResult {
    let Some(seeds_arr) = args.get("seeds").and_then(|v| v.as_array()) else {
        return CallToolResult::err("Error: Falta arreglo 'seeds'.");
    };
    let seeds: Vec<&str> = seeds_arr.iter().filter_map(|v| v.as_str()).collect();
    let gamma = args.get("gamma").and_then(|v| v.as_f64()).unwrap_or(0.8);
    let steps = args.get("steps").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
    let max_h = args.get("max_entropy_threshold").and_then(|v| v.as_f64()).unwrap_or(1.5);
    let res = blackwall.propagate_activation(&seeds, gamma, steps, max_h);
    CallToolResult::ok(serde_json::to_string(&res).unwrap_or_default())
}

pub fn handle_diff(args: &Value, blackwall: &BlackwallGate) -> CallToolResult {
    let Some(v_from) = args.get("v_from").and_then(|v| v.as_u64()) else {
        return CallToolResult::err("Error: Falta 'v_from'.");
    };
    let Some(v_to) = args.get("v_to").and_then(|v| v.as_u64()) else {
        return CallToolResult::err("Error: Falta 'v_to'.");
    };
    match blackwall.diff_versions(v_from, v_to) {
        Ok(diff) => {
            let out = json!({
                "diff": diff,
                "mermaid": to_mermaid_diff(&diff),
                "dot": to_dot_diff(&diff),
            });
            CallToolResult::ok(out.to_string())
        }
        Err(e) => CallToolResult::err(format!("Error de integridad en diff: {e}")),
    }
}
