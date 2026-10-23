//! eval_dump: harness de evaluación (SOLO medición, no afecta el motor).
//! Lee uno o varios archivos JSON con casos {"input"/"text", "id"?, "category"?,
//! "expected_output"?, "expected_route"?}, ejecuta el pipeline completo de RELIC
//! (normalizador -> despacho -> RIR -> motor determinista) N veces por caso y
//! vuelca salidas crudas con latencia p50/p95, RSS y determinismo a reports/eval/.
//!
//! Uso: cargo run -p relic_cli --example eval_dump -- --repeats 5 --out reports/eval/dump.json <archivo.json...>

use std::time::Instant;

use relic_cli::answer::answer_intent;
use relic_cli::dispatch::{parse_dispatch_intent, DispatchIntent};
use relic_cli::knowledge::load_default_knowledge;
use relic_cli::monitor::{get_current_rss_kb};
use relic_core::{ArithmeticOp, EpisodicMemory, NodeResolution, ReasoningEngine};

fn intent_name(intent: &DispatchIntent) -> &'static str {
    match intent {
        DispatchIntent::Arithmetic(_) => "arithmetic",
        DispatchIntent::GraphReachability(_) => "graph",
        DispatchIntent::RelationalReasoning(_) => "relational",
        DispatchIntent::Incomplete { .. } => "incomplete",
        DispatchIntent::FreeLanguage(_) => "free_language",
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn resolve_summary(mem: &mut EpisodicMemory, node: &str) -> String {
    match mem.resolve_node(node) {
        NodeResolution::Exact(_) => "exact",
        NodeResolution::SingleAlias(_) => "single_alias",
        NodeResolution::Ambiguous(_) => "ambiguous",
        NodeResolution::NotFound => "not_found",
    }.to_string()
}

fn graph_spec_json(mem: &mut EpisodicMemory, spec: &relic_cli::dispatch::QuerySpec) -> String {
    let mut nodo = |n: &Option<String>| match n {
        Some(s) => {
            let res = resolve_summary(mem, s);
            format!("{{\"value\":\"{}\",\"resolution\":\"{}\"}}", json_escape(s), res)
        }
        None => "null".into(),
    };
    format!(
        "{{\"initial\":{},\"target\":{},\"via\":{},\"avoid\":{},\"is_negated\":{}}}",
        nodo(&spec.initial), nodo(&spec.target), nodo(&spec.via), nodo(&spec.avoid),
        spec.is_negated
    )
}

fn plan_json(intent: &DispatchIntent) -> String {
    if let DispatchIntent::Arithmetic(plan) = intent {
        let steps = plan.steps.iter().map(|&(op, v)| {
            let sym = match op { ArithmeticOp::Add => "+", ArithmeticOp::Subtract => "-", ArithmeticOp::Multiply => "*", ArithmeticOp::Divide => "/" };
            format!("\"{}{}\"", sym, v)
        }).collect::<Vec<_>>().join(",");
        return format!("{{\"initial\":{},\"unit\":{},\"steps\":[{}]}}", plan.initial,
            plan.unit.as_ref().map(|u| format!("\"{}\"", json_escape(u))).unwrap_or_else(|| "null".into()), steps);
    }
    "null".to_string()
}

fn fmt(v: f64) -> String { format!("{}", v) }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut repeats = 5usize;
    let mut inputs: Vec<String> = Vec::new();
    let mut out_path = String::from("reports/eval/dump.json");
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--repeats" => { i += 1; repeats = args[i].parse().unwrap_or(5); }
            "--out" => { i += 1; out_path = args[i].clone(); }
            p => inputs.push(p.to_string()),
        }
        i += 1;
    }
    if inputs.is_empty() { eprintln!("sin archivos de entrada"); std::process::exit(1); }

    let mut cases_json: Vec<String> = Vec::new();
    for path in &inputs {
        let raw = std::fs::read_to_string(path).expect("leer archivo de casos");
        let data: serde_json::Value = serde_json::from_str(&raw).expect("JSON valido");
        let empty = vec![];
        let arr = data.as_array().or_else(|| data.get("cases").and_then(|c| c.as_array())).unwrap_or(&empty);
        for case in arr {
            let input = case.get("input").or_else(|| case.get("text")).and_then(|v| v.as_str()).unwrap_or("");
            let id = case.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let category = case.get("category").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let exp_out = case.get("expected_output").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let mut texts: Vec<String> = Vec::new();
            let mut routes: Vec<String> = Vec::new();
            let mut statuses: Vec<String> = Vec::new();
            let mut lats: Vec<f64> = Vec::new();
            let mut rss_kb_max = 0usize;
            let mut intent_s = String::new();
            let mut spec_s = String::new();
            let mut plan_s = String::new();

            for _ in 0..repeats {
                let t0 = Instant::now();
                let intent = parse_dispatch_intent(input);
                let (t_str, r_str, st_str, meta_s) = {
                    match &intent {
                        DispatchIntent::GraphReachability(spec) => {
                            let mut mem = EpisodicMemory::new();
                            load_default_knowledge(&mut mem);
                            let s = graph_spec_json(&mut mem, spec);
                            let a = answer_intent(&intent, &mut mem);
                            (a.text, a.route.clone(), a.status.clone(), s)
                        }
                        _ => {
                            let mut mem = EpisodicMemory::new();
                            load_default_knowledge(&mut mem);
                            let a = answer_intent(&intent, &mut mem);
                            (a.text, a.route.clone(), a.status.clone(), String::new())
                        }
                    }
                };
                let dt = t0.elapsed().as_nanos() as f64 / 1000.0;
                texts.push(t_str); routes.push(r_str.clone()); statuses.push(st_str);
                lats.push(dt);
                rss_kb_max = rss_kb_max.max(get_current_rss_kb());
                intent_s = intent_name(&intent).to_string();
                spec_s = if meta_s.is_empty() { "null".to_string() } else { meta_s };
                plan_s = plan_json(&intent);
            }
            let route0 = routes[0].clone();
            let status0 = statuses[0].clone();
            let deterministic = routes.iter().all(|r| r == &route0) && statuses.iter().all(|s| s == &status0)
                && texts.windows(2).all(|w| w[0] == w[1]);
            lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p50 = lats[lats.len() / 2];
            let p95 = lats[((lats.len() as f64) * 0.95) as usize % lats.len()];

            cases_json.push(format!(
                "{{\"ofile\":\"{}\",\"id\":\"{}\",\"category\":\"{}\",\"input\":\"{}\",\"expected_output\":\"{}\",\"intent\":\"{}\",\"plan\":{},\"spec\":{},\"repeats\":[{}],\"latency_us\":{{\"min\":{},\"p50\":{},\"p95\":{},\"max\":{}}},\"rss_kb_max\":{},\"deterministic\":{}}}",
                json_escape(path), json_escape(&id), json_escape(&category), json_escape(input), json_escape(&exp_out),
                intent_s, plan_s, spec_s,
                texts.iter().zip(routes.iter()).zip(statuses.iter())
                    .map(|((t, r), s)| format!("{{\"text\":\"{}\",\"route\":\"{}\",\"status\":\"{}\"}}", json_escape(t), r, s))
                    .collect::<Vec<_>>().join(","),
                fmt(lats[0]), fmt(p50), fmt(p95), fmt(*lats.last().unwrap()),
                rss_kb_max, deterministic
            ));
        }
        println!("[dump] {}: {} casos procesados ({} repeticiones c/u)", path, arr.len(), repeats);
    }

    let json_out = format!("{{\"harness\":\"eval_dump_rust\",\"pipeline\":\"rules+normalizer+RIR+engine\",\"created\":\"{}\",\"repeats\":{},\"cases\":[{}]}}",
        chrono_stub(), repeats, cases_json.join(","));
    if let Some(dir) = std::path::Path::new(&out_path).parent() { let _ = std::fs::create_dir_all(dir); }
    std::fs::write(&out_path, json_out).expect("escribir reporte");
    println!("[dump] escrito: {}", out_path);
}

fn chrono_stub() -> String {
    // Timestamp simple sin dependencias: segundos desde epoch.
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);
    format!("epoch_{}_s", now)
}

#[allow(dead_code)]
fn _unused_marker(_e: &ReasoningEngine) {}
