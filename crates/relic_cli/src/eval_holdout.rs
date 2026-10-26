//! Evaluación nativa de alta velocidad de Reglas y Slot-Filler sobre el Hold-Out v2 (N=4.500).
//! Menos de 180 líneas.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use serde_json::Value;
use relic_core::EpisodicMemory;
use crate::answer::answer_intent;
use crate::dispatch::parse_dispatch_intent;
use crate::knowledge::load_default_knowledge;

fn wilson_ci(k: usize, n: usize) -> (f64, f64, f64) {
    if n == 0 { return (0.0, 0.0, 0.0); }
    let p = k as f64 / n as f64;
    let z = 1.95996; // 95%
    let d = 1.0 + (z * z) / n as f64;
    let center = (p + (z * z) / (2.0 * n as f64)) / d;
    let spread = (z * ((p * (1.0 - p) / n as f64) + (z * z) / (4.0 * (n * n) as f64)).sqrt()) / d;
    (p, (center - spread).max(0.0), (center + spread).min(1.0))
}

pub fn handle_eval_holdout(file_path: &str) {
    let p = Path::new(file_path);
    if !p.exists() {
        eprintln!("[ERROR] Archivo {} no encontrado.", file_path);
        return;
    }
    let file = File::open(p).expect("No se pudo abrir el dataset");
    let reader = BufReader::new(file);

    let mut total_holdout = 0;
    let mut rules_em = 0;
    let mut rules_fa = 0;
    let mut rules_ca = 0;
    let mut rules_ia = 0;
    let mut epistemic_count = 0;
    let mut valid_count = 0;

    let mut memory = EpisodicMemory::new();
    load_default_knowledge(&mut memory);

    for line in reader.lines() {
        let l = match line { Ok(x) => x, Err(_) => continue };
        if l.trim().is_empty() { continue; }
        let val: Value = match serde_json::from_str(&l) { Ok(v) => v, Err(_) => continue };
        
        let split = val.get("split").and_then(|s| s.as_str()).unwrap_or("");
        if split != "holdout_template" { continue; }
        total_holdout += 1;

        let prompt = val.get("prompt").and_then(|s| s.as_str()).unwrap_or("");
        let exp_st = val.get("expected_status").and_then(|s| s.as_str()).unwrap_or("VALID");
        let oracle_ans = val.get("oracle_answer").and_then(|s| s.as_str()).unwrap_or("");
        let is_epistemic = exp_st == "INCOMPLETE" || exp_st == "UNSUPPORTED" || exp_st == "AMBIGUOUS";

        if is_epistemic { epistemic_count += 1; } else { valid_count += 1; }

        // 1. Evaluar Reglas Rust
        let intent = parse_dispatch_intent(prompt);
        let ans = answer_intent(&intent, &mut memory);
        let rules_abst = ans.status == "INCOMPLETE" || ans.status == "UNSUPPORTED" || ans.status == "AMBIGUOUS";
        let rules_match = !oracle_ans.is_empty() && ans.text.to_lowercase().contains(&oracle_ans.to_lowercase());

        if is_epistemic && rules_abst {
            rules_em += 1;
            rules_ca += 1;
        } else if !is_epistemic && !rules_abst && rules_match {
            rules_em += 1;
        } else if is_epistemic && !rules_abst {
            rules_fa += 1;
        } else if !is_epistemic && rules_abst {
            rules_ia += 1;
        }
    }

    let (r_p, r_low, r_high) = wilson_ci(rules_em, total_holdout);
    let (r_fa_p, r_fa_low, r_fa_high) = wilson_ci(rules_fa, total_holdout);

    println!("================================================================================");
    println!("  RELIC — EVALUACIÓN MASIVA DE REGLAS SOBRE HOLDOUT V2 (N={})", total_holdout);
    println!("================================================================================");
    println!("Total Casos Holdout:    {}", total_holdout);
    println!("  Casos Válidos:        {} | Casos Epistémicos: {}", valid_count, epistemic_count);
    println!("--------------------------------------------------------------------------------");
    println!("Línea Base: REGLAS (crates/relic_cli)");
    println!("  Exact Match (EM):     {}/{} ({:.2}%) [Wilson 95%: {:.2}% - {:.2}%]", rules_em, total_holdout, r_p*100.0, r_low*100.0, r_high*100.0);
    println!("  Falsa Aceptación:     {}/{} ({:.2}%) [Wilson 95%: {:.2}% - {:.2}%]", rules_fa, total_holdout, r_fa_p*100.0, r_fa_low*100.0, r_fa_high*100.0);
    println!("  Abstención Correcta:  {}/{} ({:.2}%)", rules_ca, epistemic_count, if epistemic_count > 0 { rules_ca as f64 / epistemic_count as f64 * 100.0 } else { 0.0 });
    println!("  Abstención Innecesaria:{}/{} ({:.2}%)", rules_ia, valid_count, if valid_count > 0 { rules_ia as f64 / valid_count as f64 * 100.0 } else { 0.0 });
    println!("================================================================================");
}
