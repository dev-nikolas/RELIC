//! Implementación de los manejadores transaccionales Blackwall para RELIC MCP.
//! Menos de 180 líneas.

use relic_blackwall::BlackwallGate;
use relic_core::NetworkAlertSpec;
use relic_lexicon::{to_dot_diff, to_mermaid_diff, LexiconPayload};
use serde_json::{json, Value};
use crate::types::CallToolResult;

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
        Ok(diff) => CallToolResult::ok(
            json!({
                "diff": diff,
                "mermaid": to_mermaid_diff(&diff),
                "dot": to_dot_diff(&diff),
            })
            .to_string(),
        ),
        Err(e) => CallToolResult::err(format!("Error de integridad en diff: {e}")),
    }
}

pub fn handle_spectral(args: &Value, blackwall: &BlackwallGate) -> CallToolResult {
    if let Some(alert_val) = args.get("network_alert") {
        let spec: NetworkAlertSpec = match serde_json::from_value(alert_val.clone()) {
            Ok(s) => s,
            Err(e) => return CallToolResult::err(format!("Error en 'network_alert': {e}")),
        };
        let report = blackwall.verify_network_alert(&spec);
        return CallToolResult::ok(serde_json::to_string(&report).unwrap_or_default());
    }
    let Some(sig_arr) = args.get("signal").and_then(|v| v.as_array()) else {
        return CallToolResult::err("Error: Falta 'signal' o 'network_alert'.");
    };
    let signal: Vec<f64> = sig_arr.iter().filter_map(|v| v.as_f64()).collect();
    let eps = args.get("support_epsilon").and_then(|v| v.as_f64()).unwrap_or(0.05);
    let min_c = args.get("min_cycles").and_then(|v| v.as_f64()).unwrap_or(3.0);
    let hint = args.get("expected_period_hint").and_then(|v| v.as_f64());
    let outcome = blackwall.analyze_spectrum(&signal, eps, min_c, hint);
    CallToolResult::ok(serde_json::to_string(&outcome).unwrap_or_default())
}

