//! ETAPA 3 del protocolo H1->RIR: binario puente de PRUEBA (aislado).
//! NO modifica despacho ni ejecutor de produccion: SOLO elige, por caso,
//! entre caminos ya existentes (parse_inventory_problem / parse_dispatch_intent /
//! intentos de abstencion canonica) y emite el mismo formato de dump que eval_dump.
//! Los textos de abstencion provienen SIEMPRE de la maquinaria existente (0 frases nuevas).
//! Menos de 180 lineas.
//!
//! Uso: cargo run -p relic_cli --example bridge_dump -- --cases reports/eval/prot_h1rir_bridge_cases.json
//!           --preds reports/eval/prot_h1rir_bridge_preds.json --out reports/eval/dump_bridge_seedN.json

use std::collections::HashMap;
use std::time::Instant;
use relic_cli::dispatch::{parse_dispatch_intent, parse_inventory_problem, DispatchIntent};
use relic_cli::answer::{answer_intent, Answer};
use relic_core::{ArithmeticOp, EpisodicMemory};
use relic_cli::knowledge::load_default_knowledge;
use relic_cli::normalize::normalize_query;
use relic_cli::monitor::get_current_rss_kb;
use serde_json::Value;

fn respuesta_de_intento(intent: &DispatchIntent) -> Answer {
    let mut mem = EpisodicMemory::new();
    load_default_knowledge(&mut mem);
    answer_intent(intent, &mut mem)
}

fn abstencion_canonica(texto: &str, motivo: &str) -> DispatchIntent {
    // Reutiliza EXISTENTE: rechazo de cantidad inicial (texto canonico del engine).
    let _ = texto;
    DispatchIntent::Incomplete {
        reason: motivo.to_string(),
        missing: "cantidad inicial".to_string(),
    }
}

type PlanInfo = (f64, String, Vec<(u8, f64)>); // initial, unit, [(0=add|1=sub, v)]

fn por_caso(texto: &str, pi: &str, ps: &str) -> (Answer, Option<PlanInfo>) {
    let vacio = || (respuesta_de_intento(&DispatchIntent::FreeLanguage(texto.to_string())), None);
    match ps {
        "VALID" => match pi {
            "inventory_update" => {
                let norm = normalize_query(texto);
                let plan = parse_inventory_problem(&norm).or_else(|| parse_inventory_problem(texto));
                match plan {
                    Some(p) => {
                        let info = (p.initial, p.unit.clone().unwrap_or_default(),
                            p.steps.iter().map(|(op, v)| (match op { ArithmeticOp::Add => 0u8, _ => 1u8 }, *v)).collect());
                        (respuesta_de_intento(&DispatchIntent::Arithmetic(p)), Some(info))
                    }
                    None => (respuesta_de_intento(&abstencion_canonica(texto, "Se desconoce la cantidad inicial disponible. Por favor especifica cuantas tienes al inicio.")), None),
                }
            }
            "graph_query" | "arithmetic_query" => (respuesta_de_intento(&parse_dispatch_intent(texto)), None),
            _ => vacio(),
        },
        "INCOMPLETE" | "AMBIGUOUS" => (respuesta_de_intento(&abstencion_canonica(texto, "Se desconoce la cantidad inicial disponible. Por favor especifica cuantas tienes al inicio.")), None),
        _ => vacio(),
    }
}

fn dump(cases: &Value, preds: &HashMap<String, (String, String)>) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in cases.as_array().expect("array de casos") {
        let input = c["input"].as_str().unwrap_or("").to_string();
        let id = c["id"].as_str().unwrap_or("").to_string();
        let (pi, ps) = preds.get(&id).cloned().unwrap_or(("unknown".into(), "UNSUPPORTED".into()));
        let mut lats: Vec<f64> = Vec::new();
        let mut texto = String::new();
        let mut rutas = String::new();
        let mut estados = String::new();
        let mut rss = 0usize;
        let mut plan: Option<PlanInfo> = None;
        for _ in 0..5 {
            let t0 = Instant::now();
            let (a, p) = por_caso(&input, &pi, &ps);
            let dt = t0.elapsed().as_nanos() as f64 / 1000.0;
            texto = a.text.clone(); rutas = a.route.clone(); estados = a.status.clone();
            plan = p;
            lats.push(dt);
            rss = rss.max(get_current_rss_kb());
        }
        lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = lats.len();
        let plan_json = match plan {
            Some((ini, unit, ops)) => {
                let ops_s = ops.iter().map(|(o, v)| format!("[{},{}]", o, v)).collect::<Vec<_>>().join(",");
                format!(",\"plan\":{{\"initial\":{},\"unit\":\"{}\",\"ops\":[{}]}}", ini, escape(&unit), ops_s)
            }
            None => ",\"plan\":null".to_string(),
        };
        parts.push(format!("{{\"id\":\"{}\",\"input\":\"{}\",\"pred\":\"{}|{}\",\"text\":\"{}\",\"route\":\"{}\",\"status\":\"{}\",\"lat_us_p50\":{},\"lat_us_min\":{:.1},\"rss_kb_max\":{}{}}}",
            id, escape(&input), escape(&pi), escape(&ps), escape(&texto), escape(&rutas), escape(&estados),
            lats[n/2].round(), lats[0], rss, plan_json));
    }
    format!("{{\"harness\":\"bridge_dump_etapa3\",\"cases\":[{}]}}", parts.join(","))
}

fn escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut cases_p = String::new(); let mut preds_p = String::new(); let mut out_p = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--cases" => { cases_p = args[i + 1].clone(); i += 2; }
            "--preds" => { preds_p = args[i + 1].clone(); i += 2; }
            "--out" => { out_p = args[i + 1].clone(); i += 2; }
            _ => i += 1,
        }
    }
    let cases: Value = serde_json::from_str(&std::fs::read_to_string(&cases_p).expect("leer cases")).expect("json cases");
    let preds_raw: HashMap<String, Value> = serde_json::from_str(&std::fs::read_to_string(&preds_p).expect("leer preds")).expect("json preds");
    let mut mapa = HashMap::new();
    for (semilla, v) in preds_raw.iter() {
        if let Value::Array(arr) = v {
            for c in arr {
                mapa.insert(format!("{}|s{}", c["id"].as_str().unwrap_or(""), semilla),
                    (c["pred_intent"].as_str().unwrap_or("unknown").to_string(),
                     c["pred_status"].as_str().unwrap_or("UNSUPPORTED").to_string()));
            }
        }
    }
    // una semilla por ejecucion (--seed), flows separados por semilla
    let seed = args.iter().position(|a| a == "--seed").and_then(|p| args.get(p + 1).cloned())
        .unwrap_or("42".into());
    let solo: HashMap<String, (String, String)> = mapa.iter()
        .filter_map(|(k, v)| k.rsplit_once("|s").and_then(|(id, s)| (s == seed).then(|| (id.to_string(), v.clone()))))
        .collect();
    let json = dump(&cases, &solo);
    std::fs::write(&out_p, json).expect("escribir dump");
    println!("[bridge_dump] seed={} | escrito {}", seed, out_p);
}
