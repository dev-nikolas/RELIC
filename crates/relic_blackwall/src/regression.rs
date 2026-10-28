//! Paso 2 del Blackwall: Suite de Regresión sobre Sandbox Efímero.
//! Verifica que ninguna mutación propuesta degrade el comportamiento canónico de RELIC.
//! Menos de 180 líneas.

use relic_cli::{answer_intent, parse_dispatch_intent_with_lexicon};
use relic_lexicon::LexiconView;
use serde::{Deserialize, Serialize};

/// Caso individual de verificación dentro de la batería de regresión del Blackwall.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionCase {
    pub id: String,
    pub query: String,
    pub expected_substring: String,
    pub expected_route: String,
    pub expected_status: String,
}

/// Diagnóstico detallado cuando un caso de regresión falla en el Sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionFailure {
    pub case_id: String,
    pub query: String,
    pub reason: String,
}

/// Banco canónico de regresión base para proteger aritmética, inventarios, abstención y grafos.
pub fn canonical_regression_suite() -> Vec<RegressionCase> {
    vec![
        RegressionCase {
            id: "reg_math_direct_01".to_string(),
            query: "1 + 1".to_string(),
            expected_substring: "2".to_string(),
            expected_route: "deterministic_arithmetic".to_string(),
            expected_status: "VALID".to_string(),
        },
        RegressionCase {
            id: "reg_math_verbal_02".to_string(),
            query: "dos más dos".to_string(),
            expected_substring: "4".to_string(),
            expected_route: "deterministic_arithmetic".to_string(),
            expected_status: "VALID".to_string(),
        },
        RegressionCase {
            id: "reg_math_precedence_03".to_string(),
            query: "calcula (10 + 5) * 2".to_string(),
            expected_substring: "10 + 5 = 15; 15 * 2 = 30".to_string(),
            expected_route: "deterministic_arithmetic".to_string(),
            expected_status: "VALID".to_string(),
        },
        RegressionCase {
            id: "reg_inv_basic_04".to_string(),
            query: "tengo 10 gatos y recibo 5".to_string(),
            expected_substring: "15 gatos".to_string(),
            expected_route: "deterministic_arithmetic".to_string(),
            expected_status: "VALID".to_string(),
        },
        RegressionCase {
            id: "reg_inv_sub_05".to_string(),
            query: "tengo 12 lápices y regalo 7".to_string(),
            expected_substring: "5 lápices".to_string(),
            expected_route: "deterministic_arithmetic".to_string(),
            expected_status: "VALID".to_string(),
        },
        RegressionCase {
            id: "reg_epistemic_incomplete_06".to_string(),
            query: "tengo algunas manzanas y luego compro 5".to_string(),
            expected_substring: "cantidad inicial".to_string(),
            expected_route: "epistemic_rejection".to_string(),
            expected_status: "INCOMPLETE".to_string(),
        },
        RegressionCase {
            id: "reg_graph_spatial_07".to_string(),
            query: "¿Es posible ir de Bogotá a Lima?".to_string(),
            expected_substring: "Bogotá -> Lima".to_string(),
            expected_route: "deterministic_graph".to_string(),
            expected_status: "VALID".to_string(),
        },
        RegressionCase {
            id: "reg_graph_modal_via_08".to_string(),
            query: "¿Puede Marta llegar a la biblioteca pasando por la estación?".to_string(),
            expected_substring: "Marta -> estación -> biblioteca".to_string(),
            expected_route: "deterministic_graph".to_string(),
            expected_status: "VALID".to_string(),
        },
        RegressionCase {
            id: "reg_graph_crate_dep_09".to_string(),
            query: "¿Depende el crate relic_cli del paquete memmap2?".to_string(),
            expected_substring: "relic_cli -> relic_core -> memmap2".to_string(),
            expected_route: "deterministic_graph".to_string(),
            expected_status: "VALID".to_string(),
        },
        RegressionCase {
            id: "reg_relational_order_10".to_string(),
            query: "Si Ana es más alta que Luis y Luis es más alto que Pedro, ¿quién es más alto, Ana o Pedro?".to_string(),
            expected_substring: "Ana".to_string(),
            expected_route: "deterministic_relational".to_string(),
            expected_status: "VALID".to_string(),
        },
    ]
}

/// Ejecuta la batería canónica + casos adicionales sobre una vista de Sandbox en RAM.
/// Si un solo caso falla, devuelve `Err(RegressionFailure)` inmediatamente.
pub fn run_regression_suite(
    sandbox_view: &mut LexiconView,
    extra_cases: &[RegressionCase],
) -> Result<usize, RegressionFailure> {
    let base_cases = canonical_regression_suite();
    let total = base_cases.len() + extra_cases.len();

    for case in base_cases.iter().chain(extra_cases.iter()) {
        let intent = parse_dispatch_intent_with_lexicon(&case.query, Some(sandbox_view));
        let answer = answer_intent(&intent, &mut sandbox_view.episodic);

        let text_ok = answer
            .text
            .to_lowercase()
            .contains(&case.expected_substring.to_lowercase());
        let route_ok = answer.route == case.expected_route;
        let status_ok = answer.status == case.expected_status;

        if !text_ok || !route_ok || !status_ok {
            return Err(RegressionFailure {
                case_id: case.id.clone(),
                query: case.query.clone(),
                reason: format!(
                    "Esperaba [status={}, route={}, texto~='{}'], obtuvo [status={}, route={}, texto='{}']",
                    case.expected_status,
                    case.expected_route,
                    case.expected_substring,
                    answer.status,
                    answer.route,
                    answer.text
                ),
            });
        }
    }

    Ok(total)
}

/// Calcula la Fidelidad de Consolidación $F \in [0.0, 1.0]$ midiendo la proporción exacta
/// de consultas protegidas (canónicas + custom) que mantienen equivalencia antes vs. después de podar.
pub fn compute_consolidation_fidelity(
    before_view: &mut LexiconView,
    after_view: &mut LexiconView,
    extra_cases: &[RegressionCase],
) -> (f64, Vec<String>) {
    let base_cases = canonical_regression_suite();
    let total = base_cases.len() + extra_cases.len();
    if total == 0 {
        return (1.0, Vec::new());
    }

    let mut matched = 0usize;
    let mut degraded = Vec::new();

    for case in base_cases.iter().chain(extra_cases.iter()) {
        let intent_before = parse_dispatch_intent_with_lexicon(&case.query, Some(before_view));
        let ans_before = answer_intent(&intent_before, &mut before_view.episodic);

        let intent_after = parse_dispatch_intent_with_lexicon(&case.query, Some(after_view));
        let ans_after = answer_intent(&intent_after, &mut after_view.episodic);

        let text_ok = ans_after
            .text
            .to_lowercase()
            .contains(&case.expected_substring.to_lowercase());
        let route_ok = ans_after.route == case.expected_route;
        let status_ok = ans_after.status == case.expected_status;
        let same_as_before = ans_after.text == ans_before.text && ans_after.status == ans_before.status;

        if text_ok && route_ok && status_ok && same_as_before {
            matched += 1;
        } else {
            degraded.push(case.id.clone());
        }
    }

    let fidelity = matched as f64 / total as f64;
    (fidelity, degraded)
}


