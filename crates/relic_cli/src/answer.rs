//! Ejecutor verificado de intenciones: produce (texto, ruta, estado) sin efectos secundarios.
use relic_core::{ArithmeticOp, ArithmeticPlan, ArithmeticStep, EpisodicMemory, NodeResolution,
    ReasoningEngine, RirArithmeticOp, RirDocumentV1, RirDomainKind};
use crate::dispatch::{DispatchIntent, QuerySpec};
use crate::relational::{RelationalQuestionType, RelationalQuerySpec};

#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub text: String,
    pub route: String,
    pub status: String,
}

impl Answer {
    fn new(text: String, route: &str, status: &str) -> Self {
        Self { text, route: route.to_string(), status: status.to_string() }
    }
}

pub fn is_generation_request(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.starts_with("genera") || lower.starts_with("inventa") || lower.starts_with("escribe")
        || lower.contains("cuento") || lower.contains("historia")
}

pub fn answer_intent(intent: &DispatchIntent, mem: &mut EpisodicMemory) -> Answer {
    match intent {
        DispatchIntent::Arithmetic(ref plan) => answer_arithmetic(plan),
        DispatchIntent::GraphReachability(ref spec) => answer_graph(spec, mem),
        DispatchIntent::RelationalReasoning(ref rel) => answer_relational(rel),
        DispatchIntent::Incomplete { ref reason, .. } =>
            Answer::new(format!("Consulta incompleta: {}", reason), "epistemic_rejection", "INCOMPLETE"),
        DispatchIntent::FreeLanguage(ref text) => {
            if is_generation_request(text) {
                Answer::new("RELIC [Modo degradado]: Modelo neuronal en espera de pesos entrenados.".to_string(), "degraded_mode", "UNVERIFIED")
            } else {
                Answer::new("Consulta fuera del dominio estructurado soportado. RELIC no puede verificar esta afirmación todavía.".to_string(), "unsupported", "UNSUPPORTED")
            }
        }
    }
}

fn answer_arithmetic(plan: &ArithmeticPlan) -> Answer {
    let rir_steps: Vec<ArithmeticStep> = plan.steps.iter().map(|&(op, val)| ArithmeticStep {
        operator: match op {
            ArithmeticOp::Add => RirArithmeticOp::Add,
            ArithmeticOp::Subtract => RirArithmeticOp::Subtract,
            ArithmeticOp::Multiply => RirArithmeticOp::Multiply,
            ArithmeticOp::Divide => RirArithmeticOp::Divide,
        },
        operand: val,
    }).collect();

    let doc = RirDocumentV1::new_inventory(plan.unit.clone(), plan.initial, rir_steps);
    if let Err(e) = doc.validate_semantic() {
        return Answer::new(format!("Rechazo semántico RIR: {}", e), "deterministic_arithmetic", "INCOMPLETE");
    }
    match plan.execute() {
        Ok(val) => {
            let fmt = plan.format_result(val);
            let form = plan.format_formula();
            let msg = if plan.unit.is_some() {
                format!("Entendí: {}. Ahora tienes {}.", form, fmt)
            } else if form.contains('=') {
                format!("Entendí: {}. El resultado es {}.", form, fmt)
            } else {
                format!("Entendí: {} = {}. El resultado es {}.", form, fmt, fmt)
            };
            Answer::new(msg, "deterministic_arithmetic", "VALID")
        }
        Err(e) => Answer::new(format!("Error aritmético: {}", e), "deterministic_arithmetic", "INVALID"),
    }
}

/// Resolución inviolable: un nodo solo es utilizabe si es Exact o SingleAlias.
/// La ambigüedad o la inexistencia devuelven una abstención explícita, nunca un silencio.
fn resolve_strict(mem: &mut EpisodicMemory, node: &str, role: &str) -> Result<String, Answer> {
    match mem.resolve_node(node) {
        NodeResolution::Exact(s) | NodeResolution::SingleAlias(s) => Ok(s.to_string()),
        NodeResolution::Ambiguous(c) => Err(Answer::new(
            format!("Consulta ambigua: La entidad {} '{}' coincide con múltiples nodos: {:?}.", role, node, c),
            "deterministic_graph", "AMBIGUOUS")),
        NodeResolution::NotFound => Err(Answer::new(
            format!("Consulta irresoluble: La entidad {} '{}' no existe en la memoria episódica.", role, node),
            "deterministic_graph", "INVALID")),
    }
}

fn answer_graph(spec: &QuerySpec, mem: &mut EpisodicMemory) -> Answer {
    let orig = spec.initial.as_deref().unwrap_or("").to_string();
    let dest = spec.target.as_deref().unwrap_or("").to_string();
    let doc = RirDocumentV1::new_graph(RirDomainKind::SpatialGraph, orig.clone(), dest.clone(),
        spec.via.clone(), spec.avoid.clone(), spec.is_negated);
    if let Err(e) = doc.validate_semantic() {
        return Answer::new(format!("Rechazo semántico RIR: {}", e), "deterministic_graph", "INCOMPLETE");
    }

    let c_orig = match resolve_strict(mem, &orig, "de origen") { Ok(s) => s, Err(a) => return a };
    let c_dest = match resolve_strict(mem, &dest, "de destino") { Ok(s) => s, Err(a) => return a };
    let c_via = match spec.via.as_deref() { Some(v) => match resolve_strict(mem, v, "de paso") { Ok(s) => Some(s), Err(a) => return a }, None => None };
    let c_avoid = match spec.avoid.as_deref() { Some(a) => match resolve_strict(mem, a, "restringida") { Ok(s) => Some(s), Err(e) => return e }, None => None };

    let avoid_arr = c_avoid.as_deref().map(|s| [s]);
    let avoid_slice: &[&str] = avoid_arr.as_ref().map(|a| a.as_slice()).unwrap_or(&[]);
    let budget = ReasoningEngine::default_budget(mem);
    let (outcome, path, len) = if let Some(ref v) = c_via {
        // Semántica de RECORRIDO: dos llamadas concatenadas (s -> via, via -> t)
        let r1 = ReasoningEngine::solve_reachability_bidirectional(mem, &c_orig, v, spec.max_depth, avoid_slice, budget);
        let r2 = ReasoningEngine::solve_reachability_bidirectional(mem, v, &c_dest, spec.max_depth, avoid_slice, budget);
        if r1.is_reachable && r2.is_reachable {
            let mut full = r1.path;
            full.extend(r2.path.into_iter().skip(1));
            let total_len = full.len() - 1;
            (relic_core::SearchOutcome::Reachable { path: full.clone(), path_ids: Vec::new() }, full, total_len)
        } else if matches!(r1.outcome, relic_core::SearchOutcome::BudgetExceeded { .. }) {
            (r1.outcome, Vec::new(), 0)
        } else if matches!(r2.outcome, relic_core::SearchOutcome::BudgetExceeded { .. }) {
            (r2.outcome, Vec::new(), 0)
        } else {
            (r1.outcome, Vec::new(), 0)
        }
    } else {
        let res = ReasoningEngine::solve_reachability_bidirectional(mem, &c_orig, &c_dest, spec.max_depth, avoid_slice, budget);
        let l = res.path_length;
        (res.outcome, res.path, l)
    };

    match outcome {
        relic_core::SearchOutcome::BudgetExceeded { levels_forward, levels_backward, frontier_size, operations_spent, .. } => {
            let msg = if let Some(limit) = spec.max_depth {
                format!("Indeterminado: Se alcanzó el límite solicitado ({} saltos) entre {} y {}.", limit, c_orig, c_dest)
            } else {
                format!("Indeterminado: Presupuesto de búsqueda agotado ({} operaciones gastadas, niveles: fwd={}/bwd={}, frontera={}) entre {} y {}.", operations_spent, levels_forward, levels_backward, frontier_size, c_orig, c_dest)
            };
            Answer::new(msg, "deterministic_graph", "INCOMPLETE")
        }
        relic_core::SearchOutcome::Unreachable { .. } => {
            if let Some(limit) = spec.max_depth {
                if path.is_empty() {
                    return Answer::new(format!("Indeterminado: Se alcanzó el límite de profundidad ({} saltos) entre {} y {} sin hallar conexión.", limit, c_orig, c_dest), "deterministic_graph", "INCOMPLETE");
                }
            }
            let msg = if spec.is_negated {
                format!("Correcto: No existe ninguna ruta viable entre {} y {}.", c_orig, c_dest)
            } else {
                format!("No, no existe ninguna ruta viable desde {} hasta {}.", c_orig, c_dest)
            };
            Answer::new(msg, "deterministic_graph", "VALID")
        }
        relic_core::SearchOutcome::Reachable { .. } => {
            let pstr = path.join(" -> ");
            let extra = c_via.as_ref().map(|v| format!(" pasando por {}", v)).unwrap_or_default();
            let msg = if spec.is_negated {
                format!("Incorrecto: Sí existe conexión viable entre {} y {} (Ruta: {}, {} saltos).", c_orig, c_dest, pstr, len)
            } else {
                format!("Sí, es posible alcanzar {} desde {}{} (Ruta: {}, {} saltos).", c_dest, c_orig, extra, pstr, len)
            };
            Answer::new(msg, "deterministic_graph", "VALID")
        }
    }
}

fn answer_relational(rel: &RelationalQuerySpec) -> Answer {
    let has_cycle = rel.graph.has_cycle("altura_mayor");
    if has_cycle {
        return Answer::new("Inconsistencia lógica: Se detectaron premisas contradictorias en la comparación.".to_string(), "deterministic_relational", "INVALID");
    }
    let ans = match &rel.question_type { RelationalQuestionType::Maximal(ref r) => rel.graph.find_maximal(r) };
    match ans {
        Some(n) => {
            let note = if rel.unparsed_clauses.is_empty() { "" } else { " (Basado exclusivamente en las premisas interpretadas)" };
            Answer::new(format!("El más alto es {}.{}", n, note), "deterministic_relational",
                if rel.unparsed_clauses.is_empty() { "VALID" } else { "PARTIAL_VALID" })
        }
        None => Answer::new("Evidencia insuficiente o ambigüedad: Existen múltiples elementos máximos locales o cadenas desconectadas.".to_string(), "deterministic_relational", "AMBIGUOUS"),
    }
}
