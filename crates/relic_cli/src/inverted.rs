//! Extractor de clausulas invertidas para inventario (etapa 4).
//!
//! Patron general: cuantificacion inicial embebida en una NP precedida por
//! preposicion ("de los/las/del ... N unidad(es) que <clausula relativa>")
//! + operaciones en el segmento previo a la NP y/o posterior a la coma.
//!
//! Fallback de parse_inventory_problem: se invoca SOLO si el camino canónico
//! fallo. No se inventan operaciones ni cantidades; lexicos ESPEJO del
//! original (dispatch.rs:64-74, snapshot ff64fc2).
use relic_core::{ArithmeticOp, ArithmeticPlan};
use crate::normalize::normalize_query;

pub const ADD_WORDS: [&str; 19] = [
    "recibo", "recibe", "me dan", "me da", "compro", "compré", "compre",
    "agrego", "más", "mas", "sumo", "adquirí", "adquiri", "recolecté",
    "recolecte", "incorporé", "incorpore", "encuentro", "encontré",
];
pub const SUB_WORDS: [&str; 38] = [
    "regalo", "regala", "doy", "di", "le di", "les di", "me quitan", "pierdo", "perdí", "perdi",
    "vendo", "vendí", "vendi", "como", "comí", "comi", "me comí", "me comi", "zampé", "zampe",
    "consumo", "consumí", "consumi", "gasto", "gasté", "gaste", "resto", "menos",
    "extravié", "extravie", "presté", "preste", "dono", "doné", "done",
    "cedo", "cedí", "cedi",
];
pub const CONECTORES: [&str; 6] = [" y ", " pero ", " luego ", " después ", " despues ", ","];

struct Cuantifica {
    initial: f64,
    unit: String,
    pre: String,
    post: String,
}

/// Localiza la NP cuantificada: "de los/las/del [<=3 tokens] N unidad que ..."
/// Devuelve segmentos externos (pre-NP y post-coma) SIN los numeros internos.
fn cuantificacion(texto_norm: &str, lower: &str) -> Option<Cuantifica> {
    for ancla in ["de los ", "de las ", "del "] {
        let Some(a) = lower.find(ancla) else { continue };
        if a > 0 && !lower.as_bytes().get(a - 1).is_some_and(|c| *c == b' ') { continue; }
        let resto = &lower[a + ancla.len()..];
        let Some(q) = resto.find(" que ") else { continue };
        let np = &resto[..q];
        let tokens: Vec<&str> = np.split_whitespace().collect();
        if tokens.len() > 4 { continue; }
        let mut ni = None;
        for (i, t) in tokens.iter().enumerate() {
            if t.parse::<f64>().is_ok() {
                ni = Some(i);
                break;
            }
        }
        let Some(i) = ni else { continue };
        if tokens.len() <= i + 1 { continue; }
        let initial = tokens[i].parse::<f64>().ok()?;
        let unidad = tokens[i + 1].trim_matches(|c: char| !c.is_alphanumeric());
        if unidad.is_empty() { continue; }
        let rel_fin = lower[q + a + ancla.len() + 5..].find(',').map(|f| f + q + a + ancla.len() + 5);
        let pre = texto_norm[..a].trim().to_string();
        let post = match rel_fin {
            Some(f) => texto_norm[f + 1..].trim().to_string(),
            None => String::new(),
        };
        if pre.is_empty() && post.is_empty() { continue; }
        return Some(Cuantifica { initial, unit: unidad.to_string(), pre, post });
    }
    None
}

fn clasifica(clause: &str) -> Option<(ArithmeticOp, f64)> {
    // '+' y '-' son las formas que normalize_query emite desde 'más'/'menos'.
    let is_add = ADD_WORDS.iter().any(|w| clause.contains(w)) || clause.contains('+');
    let is_sub = SUB_WORDS.iter().any(|w| clause.contains(w)) || clause.contains('-');
    if is_add == is_sub { return None; }
    let found = clause.split_whitespace().find(|w| w.trim_matches(|c: char| !c.is_numeric() && c != '.').parse::<f64>().is_ok());
    Some((if is_add { ArithmeticOp::Add } else { ArithmeticOp::Subtract },
          found?.trim_matches(|c: char| !c.is_numeric() && c != '.').parse().ok()?))
}

fn ops_de_segmento(clause: &str, plan: &mut ArithmeticPlan) -> bool {
    let mut norm = clause.to_lowercase().replace(',', " | ");
    for c in [" luego ", " después ", " despues ", " pero ", " y "] { norm = norm.replace(c, " | "); }
    let mut ok = true;
    for sub in norm.split('|') {
        let t = sub.trim();
        if t.is_empty() { continue; }
        match clasifica(t) {
            Some((op, v)) => plan.add_step(op, v),
            None => { ok = false; }
        }
    }
    ok && !plan.steps.is_empty()
}

/// Devuelve Some(plan) solo si cada subclausa con verbo de op existe y aporta
/// numero; cualquier verbo desconocido con contenido -> None (no se inventan).
pub fn parse_inverted_inventory(text: &str) -> Option<ArithmeticPlan> {
    let norm = normalize_query(text);
    let lower = norm.to_lowercase();
    let c = cuantificacion(&norm, &lower)?;
    let mut plan = ArithmeticPlan::new(c.initial, Some(c.unit.clone()));
    if !ops_de_segmento(&c.pre, &mut plan) && !ops_de_segmento(&c.post, &mut plan) { return None; }
    if plan.steps.is_empty() { None } else { Some(plan) }
}
