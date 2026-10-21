//! # relic_cli
//!
//! Binario de ejecución local y CLI unificado para RELIC en Rust nativo.
//!
//! Subcomandos:
//! - `query "<pregunta>"`: Compilación a RIR, validación episódica y razonamiento determinista.
//! - `bench`: Benchmark de rendimiento, latencia en microsegundos y memoria física en Windows.
//! - `demo`: Demostración de generación autoregresiva con KV-cache.

use std::path::Path;
use std::time::Instant;
use relic_core::{
    ArithmeticOp, ArithmeticPlan,
    EpisodicMemory, KvCache, ReasoningEngine, RelicConfig,
    RelicModel, SafetensorsLoader, Sampler,
};
use relic_tokenizer::{BpeTokenizer, Vocab};

// ==============================================================================
// MONITOR NATIVO DE MEMORIA FÍSICA (RSS) PARA WINDOWS (KERNEL32 / PSAPI)
// ==============================================================================
#[cfg(windows)]
fn get_current_rss_kb() -> usize {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            h_process: *mut std::ffi::c_void,
            ppsmc: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }
    unsafe {
        let mut pmc = std::mem::zeroed::<ProcessMemoryCounters>();
        pmc.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
        if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) != 0 {
            pmc.working_set_size / 1024
        } else {
            0
        }
    }
}

#[cfg(not(windows))]
fn get_current_rss_kb() -> usize {
    0
}


// ==============================================================================
// INICIALIZADOR DE HECHOS DEL MUNDO EN LA MEMORIA EPISÓDICA
// ==============================================================================
fn load_default_knowledge(memory: &mut EpisodicMemory) {
    // 1. Dominio Espacial
    memory.insert_edge("Madrid", "Barcelona");
    memory.insert_edge("Barcelona", "París");
    memory.insert_edge("Madrid", "Toledo");
    memory.insert_edge("Bogotá", "Lima");
    memory.insert_edge("Lima", "Buenos Aires");
    memory.insert_edge("Marta", "estación");
    memory.insert_edge("estación", "biblioteca");
    memory.insert_edge("nodo_A", "nodo_B");

    // 2. Dominio Dependencias de Software (RELIC Crates)
    memory.insert_edge("relic_cli", "relic_core");
    memory.insert_edge("relic_core", "memmap2");
    memory.insert_edge("relic_cli", "relic_tokenizer");
    memory.insert_edge("relic_core", "serde");
    memory.insert_edge("serde", "serde_json");
    memory.insert_edge("app_service", "auth_gateway");
    memory.insert_edge("auth_gateway", "database_connector");
}


// ==============================================================================
// CAPA DE DESPACHO UNIFICADA Y PARSER RIR
// ==============================================================================
#[derive(Debug, PartialEq)]
enum DispatchIntent {
    Arithmetic(ArithmeticPlan),
    GraphReachability(QuerySpec),
    FreeLanguage(String),
    Incomplete { reason: String, missing: String },
}

#[derive(Debug, PartialEq, Eq)]
struct QuerySpec {
    initial: Option<String>,
    target: Option<String>,
    via: Option<String>,
    avoid: Option<String>,
    is_negated: bool,
}

fn parse_inventory_problem(text: &str) -> Option<ArithmeticPlan> {
    let lower = text.to_lowercase();
    let idx_tengo = lower.find("tengo ").or_else(|| lower.find("hay "))?;
    let prefix_len = if lower[idx_tengo..].starts_with("tengo ") { 6 } else { 4 };
    let after_tengo = lower[idx_tengo + prefix_len..].trim();

    let parts: Vec<&str> = after_tengo.split_whitespace().collect();
    if parts.is_empty() { return None; }

    let initial_val: f64 = parts[0].parse().ok()?;
    let unit = if parts.len() > 1 {
        let u = parts[1].trim_matches(|c: char| !c.is_alphanumeric());
        if !["y", "pero", "luego", "que", "de"].contains(&u) {
            Some(u.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let mut plan = ArithmeticPlan::new(initial_val, unit);

    let normalized = lower
        .replace(',', " | ")
        .replace(" luego ", " | ")
        .replace(" después ", " | ")
        .replace(" y ", " | ");

    for clause in normalized.split('|').skip(1) {
        let trimmed = clause.trim();
        if trimmed.is_empty() { continue; }

        let is_add = ["recibo", "recibe", "me dan", "me da", "compro", "agrego", "más", "sumo"]
            .iter().any(|&w| trimmed.contains(w));
        let is_sub = ["regalo", "regala", "doy", "me quitan", "pierdo", "vendo", "como", "resto", "menos"]
            .iter().any(|&w| trimmed.contains(w));

        if is_add || is_sub {
            for word in trimmed.split_whitespace() {
                let clean_word = word.trim_matches(|c: char| !c.is_numeric() && c != '.' && c != '-');
                if let Ok(val) = clean_word.parse::<f64>() {
                    let op = if is_add { ArithmeticOp::Add } else { ArithmeticOp::Subtract };
                    plan.add_step(op, val);
                    break;
                }
            }
        }
    }

    if !plan.steps.is_empty() {
        Some(plan)
    } else {
        None
    }
}

fn parse_math_expression(text: &str) -> Option<ArithmeticPlan> {
    let clean = text.trim_matches(|c| c == '¿' || c == '?' || c == '¡' || c == '!' || c == '.');
    let lower = clean.to_lowercase();

    let mut expr = clean;
    for prefix in ["¿cuánto es ", "cuánto es ", "cuanto es ", "¿cuanto es ", "¿cuánto da ", "cuánto da ", "calcula ", "resultado de "] {
        if let Some(idx) = lower.find(prefix) {
            expr = &clean[idx + prefix.len()..];
            break;
        }
    }
    let expr = expr.trim();

    let mut tokens = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == ' ' {
            if !cur.is_empty() {
                tokens.push(cur.clone());
                cur.clear();
            }
        } else if matches!(c, '+' | '*' | '/' | 'x' | 'X') {
            if !cur.is_empty() {
                tokens.push(cur.clone());
                cur.clear();
            }
            tokens.push(c.to_string());
        } else if c == '-' {
            let is_unary = cur.is_empty() && (tokens.is_empty() || matches!(tokens.last().map(|s| s.as_str()), Some("+") | Some("-") | Some("*") | Some("/")));
            if is_unary {
                cur.push('-');
            } else {
                if !cur.is_empty() {
                    tokens.push(cur.clone());
                    cur.clear();
                }
                tokens.push("-".to_string());
            }
        } else if c.is_numeric() || c == '.' {
            cur.push(c);
        }
        i += 1;
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }

    if tokens.len() < 3 {
        return None;
    }

    let initial: f64 = tokens[0].parse().ok()?;
    let mut plan = ArithmeticPlan::new(initial, None);

    let mut j = 1;
    while j + 1 < tokens.len() {
        let op_str = &tokens[j];
        let val: f64 = tokens[j + 1].parse().ok()?;
        let op = match op_str.as_str() {
            "+" => ArithmeticOp::Add,
            "-" => ArithmeticOp::Subtract,
            "*" | "x" | "X" => ArithmeticOp::Multiply,
            "/" => ArithmeticOp::Divide,
            _ => return None,
        };
        plan.add_step(op, val);
        j += 2;
    }

    if !plan.steps.is_empty() {
        Some(plan)
    } else {
        None
    }
}

fn parse_query_to_spec(text: &str) -> QuerySpec {
    let clean = text.trim_matches(|c| c == '¿' || c == '?' || c == '¡' || c == '!' || c == '.');
    let lower = clean.to_lowercase();

    let is_negated = if lower.contains("no es imposible") {
        false
    } else {
        lower.contains("es imposible") || lower.contains("no es posible") || lower.contains("no se puede")
    };

    let avoid = if let Some(idx) = lower.find("sin pasar por ") {
        let rest = &clean[idx + 14..];
        let token = rest.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '?' || c == '.' || c == ',');
        Some(token.to_string())
    } else {
        None
    };

    let via = if let Some(idx) = lower.find("pasando por ") {
        let rest = &clean[idx + 12..];
        let token = rest.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '?' || c == '.' || c == ',');
        Some(token.to_string())
    } else {
        None
    };

    if lower.contains("depende") && lower.contains("del") {
        let parts: Vec<&str> = clean.split_whitespace().collect();
        let mut initial = None;
        let mut target = None;
        for (i, &word) in parts.iter().enumerate() {
            if (word.eq_ignore_ascii_case("crate") || word.eq_ignore_ascii_case("componente")) && i + 1 < parts.len() {
                initial = Some(parts[i + 1].trim_matches(|c| c == '?' || c == ',').to_string());
            }
            if (word.eq_ignore_ascii_case("del") || word.eq_ignore_ascii_case("paquete")) && i + 1 < parts.len() {
                let candidate = parts[i + 1].trim_matches(|c| c == '?' || c == ',');
                if candidate != "paquete" && candidate != "crate" {
                    target = Some(candidate.to_string());
                } else if i + 2 < parts.len() {
                    target = Some(parts[i + 2].trim_matches(|c| c == '?' || c == ',').to_string());
                }
            }
        }
        if initial.is_some() && target.is_some() {
            return QuerySpec { initial, target, via, avoid, is_negated };
        }
    }

    let mut initial = None;
    for prefix in ["partiendo de ", "desde ", "salir de ", "de "] {
        if let Some(idx) = lower.find(prefix) {
            let rest = &clean[idx + prefix.len()..];
            let token = rest.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '?' || c == '.' || c == ',');
            if !["la", "el", "los", "las", "un", "una", "ir", "salir", "llegar"].contains(&token.to_lowercase().as_str()) {
                initial = Some(token.to_string());
                break;
            }
        }
    }

    if initial.is_none() && lower.contains("marta") && lower.contains("llegar a") {
        initial = Some("Marta".to_string());
    }

    let mut target = None;
    for prefix in ["hasta ", "hacia ", "llegar a ", "alcanzar ", " a "] {
        if let Some(idx) = lower.find(prefix) {
            let rest = &clean[idx + prefix.len()..];
            let token = rest.split_whitespace().next().unwrap_or("").trim_matches(|c| c == '?' || c == '.' || c == ',');
            if !["la", "el", "los", "las", "un", "una"].contains(&token.to_lowercase().as_str()) {
                if Some(token) != initial.as_deref() {
                    target = Some(token.to_string());
                    break;
                }
            }
        }
    }

    QuerySpec { initial, target, via, avoid, is_negated }
}

fn parse_dispatch_intent(text: &str) -> DispatchIntent {
    let clean = text.trim();
    let lower = clean.to_lowercase();

    // 1. Detectar incompletitud explícita en expresiones matemáticas o de inventario
    if lower.contains("+ ?") || lower.contains("- ?") || lower.contains("* ?") || lower.contains("/ ?")
        || clean.ends_with('+') || clean.ends_with('-') || clean.ends_with('*') || clean.ends_with('/')
    {
        return DispatchIntent::Incomplete {
            reason: "Falta el segundo operando para completar la operación solicitada.".to_string(),
            missing: "segundo operando".to_string(),
        };
    }

    if lower.contains("tengo algunas ") || lower.contains("tengo algunos ") || lower.contains("hay algunas ") {
        return DispatchIntent::Incomplete {
            reason: "Se desconoce la cantidad inicial disponible. Por favor especifica cuántas tienes al inicio.".to_string(),
            missing: "cantidad inicial".to_string(),
        };
    }

    // 2. Detectar problemas verbales de inventario ("Tengo X [unidad] y recibo Y más...")
    if lower.contains("tengo ") || lower.contains("hay ") {
        if let Some(plan) = parse_inventory_problem(clean) {
            return DispatchIntent::Arithmetic(plan);
        }
    }

    // 3. Detectar expresiones matemáticas estándar ("1 + 1", "¿cuánto es 2 + 3?", etc.)
    if let Some(plan) = parse_math_expression(clean) {
        return DispatchIntent::Arithmetic(plan);
    }

    // 4. Detectar consultas de grafos / dependencias
    let graph_keywords = ["depende", "llegar a", "alcanzar", "camino de", "ruta de", "ir de", "partiendo de"];
    if graph_keywords.iter().any(|k| lower.contains(k)) {
        let spec = parse_query_to_spec(clean);
        if spec.initial.is_none() || spec.target.is_none() {
            return DispatchIntent::Incomplete {
                reason: "Consulta de ruta incompleta: falta especificar el origen o el destino requerido.".to_string(),
                missing: "extremo de ruta".to_string(),
            };
        }
        return DispatchIntent::GraphReachability(spec);
    }

    // 5. Por defecto: lenguaje libre derivado a RELIC-10M
    DispatchIntent::FreeLanguage(clean.to_string())
}

// ==============================================================================
// EJECUCIÓN DEL DESPACHO CON MEDICIÓN SEPARADA DE LATENCIAS
// ==============================================================================
fn handle_query(query: &str) {
    println!("============================================================");
    println!("  RELIC — Capa de Despacho y Razonamiento Verificado");
    println!("============================================================");
    println!("Petición Recibida: \"{}\"", query);

    let t0 = Instant::now();
    let rss_before = get_current_rss_kb();

    // 1. Fase de Parser & Despacho
    let t_parse_start = Instant::now();
    let intent = parse_dispatch_intent(query);
    let t_parse = t_parse_start.elapsed();

    match intent {
        DispatchIntent::Arithmetic(ref plan) => {
            println!("\n[1. RIR INTENCIÓN]: ARITHMETIC_OPERATION");
            println!("    Valor Inicial: {} {:?}", plan.initial, plan.unit);
            println!("    Operaciones:   {:?}", plan.steps);

            let t_engine_start = Instant::now();
            let result = plan.execute();
            let t_engine = t_engine_start.elapsed();

            println!("\n[2. ESTADO]: VALID (Ejecución Aritmética Determinista)");
            println!("\n[3. RESPUESTA VERIFICADA]:");
            match result {
                Ok(val) => {
                    let formatted = plan.format_result(val);
                    if plan.unit.is_some() {
                        println!("  Ahora tienes {}.", formatted);
                    } else {
                        println!("  El resultado es {}.", formatted);
                    }
                }
                Err(e) => {
                    println!("  Error aritmético determinista: {}", e);
                }
            }

            print_granular_performance(t_parse, t_engine, t0.elapsed(), rss_before);
        }

        DispatchIntent::GraphReachability(ref spec) => {
            println!("\n[1. RIR INTENCIÓN]: GRAPH_REACHABILITY");
            println!("    Estado Inicial:  {:?}", spec.initial);
            println!("    Estado Objetivo: {:?}", spec.target);
            println!("    Vía / Escala:    {:?}", spec.via);
            println!("    Evitar / Avoid:  {:?}", spec.avoid);

            let orig = spec.initial.as_ref().unwrap();
            let dest = spec.target.as_ref().unwrap();

            let mut memory = EpisodicMemory::new();
            load_default_knowledge(&mut memory);

            let t_engine_start = Instant::now();
            let (reachable, path, len) = if let Some(ref via_node) = spec.via {
                let r1 = ReasoningEngine::solve_reachability_bfs(&mut memory, orig, via_node, 8);
                let r2 = ReasoningEngine::solve_reachability_bfs(&mut memory, via_node, dest, 8);
                if r1.is_reachable && r2.is_reachable {
                    let mut full = r1.path;
                    full.extend(r2.path.into_iter().skip(1));
                    let total_len = full.len() - 1;
                    (true, full, total_len)
                } else {
                    (false, Vec::new(), 0)
                }
            } else {
                let res = ReasoningEngine::solve_reachability_bfs(&mut memory, orig, dest, 8);
                (res.is_reachable, res.path, res.path_length)
            };
            let t_engine = t_engine_start.elapsed();

            println!("\n[2. ESTADO]: VALID (Ejecución BFS)");
            println!("\n[3. RESPUESTA VERIFICADA]:");
            if reachable {
                let path_str = path.join(" -> ");
                if spec.is_negated {
                    println!("  Incorrecto: Sí existe conexión viable entre {} y {} (Ruta: {}, {} saltos).", orig, dest, path_str, len);
                } else {
                    let extra = if let Some(ref v) = spec.via { format!(" pasando por {}", v) } else { "".to_string() };
                    println!("  Sí, es posible alcanzar {} desde {}{} (Ruta: {}, {} saltos).", dest, orig, extra, path_str, len);
                }
            } else {
                if spec.is_negated {
                    println!("  Correcto: No existe ninguna ruta viable entre {} y {}.", orig, dest);
                } else {
                    println!("  No, no existe ninguna ruta viable desde {} hasta {}.", orig, dest);
                }
            }

            print_granular_performance(t_parse, t_engine, t0.elapsed(), rss_before);
        }

        DispatchIntent::Incomplete { ref reason, ref missing } => {
            println!("\n[1. RIR INTENCIÓN]: INCOMPLETE (Rechazo Epistémico)");
            println!("    Campo Faltante:  {}", missing);
            println!("\n[2. ESTADO]: INCOMPLETE");
            println!("\n[3. RESPUESTA VERIFICADA]:\n  Consulta incompleta: {}", reason);
            print_granular_performance(t_parse, std::time::Duration::from_nanos(0), t0.elapsed(), rss_before);
        }

        DispatchIntent::FreeLanguage(ref _text) => {
            // Ruta de ejecución sin validación: se rechaza explícitamente
            println!("\n[1. RIR INTENCIÓN]: FREE_LANGUAGE (Rechazo seguro)");
            println!("    Consulta fuera del dominio soportado o no estructurada. Rechazando.");
            // Devolver un estado de intento incompleto (simulado vía salida estándar)
            println!("[RESULTADO]: ValidityStatus::UNSUPPORTED");
            // No se llama a handle_demo, evitando la generación autoregresiva
        }
    }
}

fn print_granular_performance(
    t_parse: std::time::Duration,
    t_engine: std::time::Duration,
    t_total: std::time::Duration,
    rss_before: usize,
) {
    let rss_after = get_current_rss_kb();
    let delta = if rss_after >= rss_before { rss_after - rss_before } else { 0 };
    println!("------------------------------------------------------------");
    println!("Latencia del Parser:        {:.2?} ({:.1} µs)", t_parse, t_parse.as_nanos() as f64 / 1000.0);
    println!("Latencia del Motor:         {:.2?} ({:.1} µs)", t_engine, t_engine.as_nanos() as f64 / 1000.0);
    println!("Latencia Total End-to-End:  {:.2?} ({:.1} µs)", t_total, t_total.as_nanos() as f64 / 1000.0);
    println!("Memoria Física Activa RSS:  {:.2} MB (Delta: +{} KB)", rss_after as f64 / 1024.0, delta);
    println!("============================================================");
}


// ==============================================================================
// BENCHMARK DE RENDIMIENTO INTEGRAL EN RUST
// ==============================================================================
fn handle_bench() {
    println!("============================================================");
    println!("  RELIC — Benchmark Oficial de Memoria y Razonamiento");
    println!("============================================================");

    let val_id_path = Path::new("data/benchmarks/graph/val_id.json");
    if !val_id_path.exists() {
        eprintln!("[ERROR] No se encontró data/benchmarks/graph/val_id.json");
        return;
    }

    let file = std::fs::File::open(val_id_path).expect("Abrir archivo");
    let samples: Vec<serde_json::Value> = serde_json::from_reader(std::io::BufReader::new(file)).expect("Parsear JSON");
    println!("Muestras cargadas: {}", samples.len());

    let mut memory = EpisodicMemory::new();
    let start = Instant::now();
    let mut total_correct = 0;

    for s in &samples {
        memory.clear();
        let prompt = s["prompt"].as_str().unwrap();
        let target = s["target"].as_str().unwrap();

        memory.load_from_text(prompt);
        if let Some(idx) = prompt.find(". ¿Camino ") {
            let q = &prompt[idx + 10..];
            if let Some(end_idx) = q.find("? ->") {
                let clean_q = &q[..end_idx];
                if let Some((u, v)) = clean_q.split_once(" a ") {
                    let res = ReasoningEngine::solve_reachability_bfs(&mut memory, u.trim(), v.trim(), 8);
                    let pred = if res.is_reachable { "SI" } else { "NO" };
                    if pred == target {
                        total_correct += 1;
                    }
                }
            }
        }
    }

    let elapsed = start.elapsed();
    let us_per_query = (elapsed.as_nanos() as f64 / samples.len() as f64) / 1000.0;
    println!("Exactitud Determinista (BFS): {:.2}%", (total_correct as f64 / samples.len() as f64) * 100.0);
    println!("Latencia Promedio por Muestra: {:.2} µs", us_per_query);
    println!("Throughput:                   {:.0} muestras/segundo", 1_000_000.0 / us_per_query);
    println!("Memoria Física Activa RSS:    {:.2} MB", get_current_rss_kb() as f64 / 1024.0);
    println!("============================================================");
}


// ==============================================================================
// DEMOSTRACIÓN DE GENERACIÓN AUTOREGRESIVA CON EL LLM EN RUST
// ==============================================================================
fn handle_demo(prompt_text: &str) {
    println!("============================================================");
    println!("  RELIC-10M — Inferencia Autoregresiva (Rust Nativo)");
    println!("============================================================");

    let config = RelicConfig::default();
    let _params = config.count_parameters();

    let mut model = RelicModel::new(config.clone());
    let weights_path = Path::new("artifacts/weights/relic_10m_fp16.safetensors");
    if weights_path.exists() {
        if let Ok(loader) = SafetensorsLoader::open(weights_path) {
            let _ = loader.load_into_model(&mut model);
            println!("[INFO] Pesos Safetensors cargados.");
        }
    } else {
        println!("[WARN] No se encontró artifacts/weights/relic_10m_fp16.safetensors, usando inicialización");
    }

    let tok_path = Path::new("artifacts/tokenizer/tokenizer.json");
    let tokenizer = if tok_path.exists() {
        Vocab::from_file(tok_path).ok().map(|v| {
            BpeTokenizer::new(v)
        })
    } else {
        None
    };

    println!("Prompt: \"{}\"", prompt_text);

    let mut token_ids = if let Some(ref tok) = tokenizer {
        tok.encode(prompt_text)
    } else {
        vec![12u32, 45, 99, 1024]
    };

    let mut kv_cache = KvCache::new(config.num_hidden_layers, config.max_train_seq_len, config.num_key_value_heads, config.head_dim);
    let mut step_logits = vec![0.0f32; config.vocab_size];

    let t0 = Instant::now();
    for (pos, &tid) in token_ids.iter().enumerate() {
        model.forward_decode_step(tid, pos, &mut kv_cache, &mut step_logits);
    }

    let tokens_to_gen = 16;
    for _ in 0..tokens_to_gen {
        let next_token = Sampler::greedy(&step_logits);
        token_ids.push(next_token);
        let pos = kv_cache.current_len;
        if pos >= config.max_train_seq_len { break; }
        model.forward_decode_step(next_token, pos, &mut kv_cache, &mut step_logits);
    }

    let elapsed = t0.elapsed();
    let gen_rate = (tokens_to_gen as f64) / elapsed.as_secs_f64();

    if let Some(ref tok) = tokenizer {
        if let Ok(text) = tok.decode(&token_ids) {
            println!("\n[GENERACIÓN]:\n\"{}\"", text);
        }
    }
    println!("\nLatencia: {:.2?} ({:.1} tok/s) | RSS: {:.2} MB", elapsed, gen_rate, get_current_rss_kb() as f64 / 1024.0);
    println!("============================================================");
}


fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "query" => {
                let query = if args.len() > 2 {
                    args[2..].join(" ")
                } else {
                    "¿Depende el crate relic_cli del paquete memmap2?".to_string()
                };
                handle_query(&query);
            }
            "bench" => handle_bench(),
            "demo" => {
                let prompt = if args.len() > 2 {
                    args[2..].join(" ")
                } else {
                    "El pingüino encontró una señal en Bogotá".to_string()
                };
                handle_demo(&prompt);
            }
            _ => {
                let query = args[1..].join(" ");
                handle_query(&query);
            }
        }
    } else {
        handle_demo("El pingüino encontró una señal en Bogotá");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_simple_arithmetic() {
        let intent = parse_dispatch_intent("1 + 1");
        if let DispatchIntent::Arithmetic(plan) = intent {
            assert_eq!(plan.execute(), Ok(2.0));
            assert_eq!(plan.format_result(2.0), "2");
        } else {
            panic!("Esperaba intención aritmética");
        }
    }

    #[test]
    fn test_dispatch_addition_question() {
        let intent = parse_dispatch_intent("¿Cuánto es 2 + 3?");
        if let DispatchIntent::Arithmetic(plan) = intent {
            assert_eq!(plan.execute(), Ok(5.0));
            assert_eq!(plan.format_result(5.0), "5");
        } else {
            panic!("Esperaba intención aritmética");
        }
    }

    #[test]
    fn test_dispatch_inventory_addition() {
        let intent = parse_dispatch_intent("Tengo 2 manzanas y recibo 1 más");
        if let DispatchIntent::Arithmetic(plan) = intent {
            let res = plan.execute().unwrap();
            assert_eq!(res, 3.0);
            assert_eq!(plan.format_result(res), "3 manzanas");
        } else {
            panic!("Esperaba intención de inventario");
        }
    }

    #[test]
    fn test_dispatch_inventory_multi_step() {
        let intent = parse_dispatch_intent("Tengo 2 manzanas y recibo 1 más, luego regalo 2");
        if let DispatchIntent::Arithmetic(plan) = intent {
            let res = plan.execute().unwrap();
            assert_eq!(res, 1.0);
            assert_eq!(plan.format_result(res), "1 manzana");
        } else {
            panic!("Esperaba intención de inventario multi-paso");
        }
    }

    #[test]
    fn test_dispatch_incomplete_missing_initial() {
        let intent = parse_dispatch_intent("Tengo algunas manzanas y recibo 1 más");
        assert!(matches!(intent, DispatchIntent::Incomplete { .. }));
    }

    #[test]
    fn test_dispatch_incomplete_missing_operand() {
        let intent = parse_dispatch_intent("¿Cuánto es 2 + ?");
        assert!(matches!(intent, DispatchIntent::Incomplete { .. }));
    }

    #[test]
    fn test_dispatch_decimals_and_negatives() {
        let intent = parse_dispatch_intent("¿Cuánto es -5 + 3?");
        if let DispatchIntent::Arithmetic(plan) = intent {
            assert_eq!(plan.execute(), Ok(-2.0));
        } else {
            panic!("Esperaba aritmética negativa");
        }

        let intent2 = parse_dispatch_intent("2.5 + 1.5");
        if let DispatchIntent::Arithmetic(plan) = intent2 {
            assert_eq!(plan.execute(), Ok(4.0));
            assert_eq!(plan.format_result(4.0), "4");
        } else {
            panic!("Esperaba aritmética decimal");
        }
    }

    #[test]
    fn test_dispatch_graph_query() {
        let intent = parse_dispatch_intent("¿Depende el crate relic_cli del paquete memmap2?");
        assert!(matches!(intent, DispatchIntent::GraphReachability(_)));
    }
}

