//! Implementación de las 3 herramientas nucleares de RELIC MCP.
//! Menos de 180 líneas.

use relic_core::{EpisodicMemory, RirDocumentV1};
use relic_cli::{answer_intent, normalize_query, parse_dispatch_intent, DispatchIntent};
use serde_json::{json, Value};
use crate::types::{CallToolResult, ToolDefinition};

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
            description: "Valida semánticamente un documento RIR v1 contra invariantes de transitividad y arimética.".to_string(),
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
    ]
}

pub fn handle_interpret(args: &Value, mem: &mut EpisodicMemory) -> CallToolResult {
    let text = match args.get("text").and_then(|v| v.as_str()) {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => return CallToolResult::err("Error: Parámetro 'text' faltante o vacío."),
    };

    let normalized = normalize_query(text);
    let intent = parse_dispatch_intent(&normalized);
    let answer = answer_intent(&intent, mem);

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

    let rir_str = rir_val.to_string();
    let doc = match RirDocumentV1::from_json_str(&rir_str) {
        Ok(d) => d,
        Err(e) => return CallToolResult::err(format!("Error de sintaxis RIR: {e}")),
    };

    match doc.validate_semantic() {
        Ok(()) => {
            let res = format!(
                "Verificación: VÁLIDA\nEstado: VALID\nIntento: {:?}\nDominio: {:?}",
                doc.intent, doc.domain
            );
            CallToolResult::ok(res)
        }
        Err(err) => {
            let res = format!("Verificación: RECHAZADA\nEstado: INCOMPLETE\nDetalle: {err}");
            CallToolResult::ok(res)
        }
    }
}

pub fn handle_retrieve(args: &Value, mem: &mut EpisodicMemory) -> CallToolResult {
    let entity = match args.get("entity").and_then(|v| v.as_str()) {
        Some(e) if !e.trim().is_empty() => e.trim(),
        _ => return CallToolResult::err("Error: Parámetro 'entity' faltante o vacío."),
    };

    let neighbors: Vec<String> = mem.get_neighbors(entity).to_vec();
    if neighbors.is_empty() {
        CallToolResult::ok(format!(
            "Memoria Episódica: '{}'\nEstado: INCOMPLETE\nDetalle: No se encontraron relaciones para esta entidad.",
            entity
        ))
    } else {
        let relations: Vec<String> = neighbors
            .iter()
            .map(|dest| format!("--> {dest}"))
            .collect();
        CallToolResult::ok(format!(
            "Memoria Episódica: '{}'\nEstado: VALID\nVecinos ({}/{}):\n{}",
            entity,
            neighbors.len(),
            neighbors.len(),
            relations.join("\n")
        ))
    }
}
