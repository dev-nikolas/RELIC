
use relic_cli::{answer_intent, parse_dispatch_intent_with_lexicon};
use relic_lexicon::LexiconView;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionCase {
    pub id: String,
    pub query: String,
    pub expected_substring: String,
    pub expected_route: String,
    pub expected_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionFailure {
    pub case_id: String,
    pub query: String,
    pub reason: String,
}

fn rc(id: &str, query: &str, expected_substring: &str, expected_route: &str, expected_status: &str) -> RegressionCase {
    RegressionCase {
        id: id.to_string(),
        query: query.to_string(),
        expected_substring: expected_substring.to_string(),
        expected_route: expected_route.to_string(),
        expected_status: expected_status.to_string(),
    }
}

pub fn canonical_regression_suite() -> Vec<RegressionCase> {
    vec![
        rc("reg_math_direct_01", "1 + 1", "2", "deterministic_arithmetic", "VALID"),
        rc("reg_math_verbal_02", "dos más dos", "4", "deterministic_arithmetic", "VALID"),
        rc("reg_math_precedence_03", "calcula (10 + 5) * 2", "10 + 5 = 15; 15 * 2 = 30", "deterministic_arithmetic", "VALID"),
        rc("reg_inv_basic_04", "tengo 10 gatos y recibo 5", "15 gatos", "deterministic_arithmetic", "VALID"),
        rc("reg_inv_sub_05", "tengo 12 lápices y regalo 7", "5 lápices", "deterministic_arithmetic", "VALID"),
        rc("reg_epistemic_incomplete_06", "tengo algunas manzanas y luego compro 5", "cantidad inicial", "epistemic_rejection", "INCOMPLETE"),
        rc("reg_graph_spatial_07", "¿Es posible ir de Bogotá a Lima?", "Bogotá -> Lima", "deterministic_graph", "VALID"),
        rc("reg_graph_modal_via_08", "¿Puede Marta llegar a la biblioteca pasando por la estación?", "Marta -> estación -> biblioteca", "deterministic_graph", "VALID"),
        rc("reg_graph_crate_dep_09", "¿Depende el crate relic_cli del paquete memmap2?", "relic_cli -> relic_core -> memmap2", "deterministic_graph", "VALID"),
        rc("reg_relational_order_10", "Si Ana es más alta que Luis y Luis es más alto que Pedro, ¿quién es más alto, Ana o Pedro?", "Ana", "deterministic_relational", "VALID"),
    ]
}

pub fn run_regression_suite(
    sandbox_view: &mut LexiconView,
    extra_cases: &[RegressionCase],
) -> Result<usize, RegressionFailure> {
    let base_cases = canonical_regression_suite();
    let total = base_cases.len() + extra_cases.len();

    for case in base_cases.iter().chain(extra_cases.iter()) {
        let intent = parse_dispatch_intent_with_lexicon(&case.query, Some(sandbox_view));
        let answer = answer_intent(&intent, &mut sandbox_view.episodic);

        let text_ok = answer.text.to_lowercase().contains(&case.expected_substring.to_lowercase());
        let route_ok = answer.route == case.expected_route;
        let status_ok = answer.status == case.expected_status;

        if !text_ok || !route_ok || !status_ok {
            return Err(RegressionFailure {
                case_id: case.id.clone(),
                query: case.query.clone(),
                reason: format!(
                    "Esperaba [status={}, route={}, texto~='{}'], obtuvo [status={}, route={}, texto='{}']",
                    case.expected_status, case.expected_route, case.expected_substring,
                    answer.status, answer.route, answer.text
                ),
            });
        }
    }

    Ok(total)
}

pub fn compute_consolidation_fidelity(
    before_view: &mut LexiconView,
    after_view: &mut LexiconView,
    extra_cases: &[RegressionCase],
) -> (f64, Vec<String>) {
    let base_cases = canonical_regression_suite();
    let total = base_cases.len() + extra_cases.len();
    if total == 0 { return (1.0, Vec::new()); }

    let (mut matched, mut degraded) = (0usize, Vec::new());
    for case in base_cases.iter().chain(extra_cases.iter()) {
        let intent_before = parse_dispatch_intent_with_lexicon(&case.query, Some(before_view));
        let ans_before = answer_intent(&intent_before, &mut before_view.episodic);

        let intent_after = parse_dispatch_intent_with_lexicon(&case.query, Some(after_view));
        let ans_after = answer_intent(&intent_after, &mut after_view.episodic);

        let text_ok = ans_after.text.to_lowercase().contains(&case.expected_substring.to_lowercase());
        let route_ok = ans_after.route == case.expected_route;
        let status_ok = ans_after.status == case.expected_status;
        let same_as_before = ans_after.text == ans_before.text && ans_after.status == ans_before.status;

        if text_ok && route_ok && status_ok && same_as_before {
            matched += 1;
        } else {
            degraded.push(case.id.clone());
        }
    }

    (matched as f64 / total as f64, degraded)
}
