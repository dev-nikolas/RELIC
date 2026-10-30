//! Implementación de los manejadores transaccionales Blackwall para RELIC MCP.
use relic_blackwall::BlackwallGate;
use relic_core::{ArithmeticOp, KuramotoNetworkSpec, NetworkAlertSpec};
use relic_lexicon::{to_dot_diff, to_mermaid_diff, EngramCapsule, LexiconPayload};
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

pub fn handle_synchronize(args: &Value, blackwall: &BlackwallGate) -> CallToolResult {
    let Some(spec_val) = args.get("spec") else {
        return CallToolResult::err("Error: Falta 'spec' (KuramotoNetworkSpec).");
    };
    let spec: KuramotoNetworkSpec = match serde_json::from_value(spec_val.clone()) {
        Ok(s) => s,
        Err(e) => return CallToolResult::err(format!("Error en 'spec': {e}")),
    };
    let outcome = blackwall.synchronize_kuramoto(&spec);
    CallToolResult::ok(serde_json::to_string(&outcome).unwrap_or_default())
}

pub fn handle_assimilate(args: &Value, blackwall: &mut BlackwallGate) -> CallToolResult {
    let Some(raw_query) = args.get("raw_query").and_then(|v| v.as_str()) else {
        return CallToolResult::err("Error: Falta 'raw_query'.");
    };
    let Some(op_val) = args.get("verified_op") else {
        return CallToolResult::err("Error: Falta 'verified_op'.");
    };
    let verified_op: ArithmeticOp = match serde_json::from_value(op_val.clone()) {
        Ok(o) => o,
        Err(e) => return CallToolResult::err(format!("Error en 'verified_op': {e}")),
    };
    let ts = args.get("timestamp_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let rep = blackwall.assimilate_verified_example(raw_query, verified_op, ts);
    CallToolResult::ok(serde_json::to_string(&rep).unwrap_or_default())
}

pub fn handle_capsule(args: &Value, blackwall: &mut BlackwallGate) -> CallToolResult {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("export");
    if action.eq_ignore_ascii_case("import") {
        let Some(cap_val) = args.get("capsule") else {
            return CallToolResult::err("Error: Falta 'capsule' para import.");
        };
        let cap: EngramCapsule = match serde_json::from_value(cap_val.clone()) {
            Ok(c) => c,
            Err(e) => return CallToolResult::err(format!("Error en 'capsule': {e}")),
        };
        let ts = args.get("timestamp_ms").and_then(|v| v.as_u64()).unwrap_or(0);
        let outcome = blackwall.import_engram_capsule(&cap, ts);
        return CallToolResult::ok(serde_json::to_string(&outcome).unwrap_or_default());
    }
    let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("relic_mcp");
    let v_from = args.get("v_from").and_then(|v| v.as_u64()).unwrap_or(1);
    let v_to = args.get("v_to").and_then(|v| v.as_u64()).unwrap_or(blackwall.current_version());
    match blackwall.export_engram_capsule(domain, v_from, v_to) {
        Ok(cap) => CallToolResult::ok(serde_json::to_string(&cap).unwrap_or_default()),
        Err(e) => CallToolResult::err(format!("Error exportando cápsula: {e}")),
    }
}

