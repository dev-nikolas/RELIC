//! Módulo de generación y verificación curricular para Niveles 1 a 3.
//! Menos de 180 líneas.

pub mod types;
pub mod templates_l01;
pub mod templates_l02;
pub mod templates_l03;
pub mod templates;
pub mod engine;
pub mod export;

pub use types::{CurriculumLevel, CurriculumSample, DatasetSplit};
pub use engine::generate_sample;
pub use export::generate_and_export_curriculum;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determinism_same_seed_gives_identical_samples() {
        let s1 = generate_sample(42, CurriculumLevel::L01DirectArithmetic, DatasetSplit::Train, 0);
        let s2 = generate_sample(42, CurriculumLevel::L01DirectArithmetic, DatasetSplit::Train, 0);
        assert_eq!(s1.prompt, s2.prompt);
        assert_eq!(s1.gold_rir, s2.gold_rir);
        assert_eq!(s1.oracle_answer, s2.oracle_answer);
    }

    #[test]
    fn test_holdout_template_separation() {
        let train_sample = generate_sample(100, CurriculumLevel::L01DirectArithmetic, DatasetSplit::Train, 0);
        let holdout_sample = generate_sample(100, CurriculumLevel::L01DirectArithmetic, DatasetSplit::HoldoutTemplate, 0);
        assert!(train_sample.template_id.starts_with("T_"));
        assert!(holdout_sample.template_id.starts_with("H_"));
        assert_ne!(train_sample.template_id, holdout_sample.template_id);
    }

    #[test]
    fn test_epistemic_rejection_semantic_status() {
        let sample = generate_sample(200, CurriculumLevel::L03EpistemicRejection, DatasetSplit::Train, 0);
        assert!(sample.gold_rir.status != relic_core::RirValidityStatus::Valid);
        assert!(sample.expected_status == "INCOMPLETE" || sample.expected_status == "UNSUPPORTED");
    }

    #[test]
    fn test_inventory_semantic_concordance() {
        let sample = generate_sample(300, CurriculumLevel::L02InventoryUnits, DatasetSplit::Train, 1);
        assert_eq!(sample.expected_status, "VALID");
        assert!(sample.gold_rir.entity.is_some());
    }

    #[test]
    fn test_permanent_gold_operands_strictly_match_prompt_numbers() {
        // Prueba permanente obligatoria: los números de la frase deben ser exactamente los operandos del gold
        for seed in [42, 999, 2026] {
            for level in [CurriculumLevel::L01DirectArithmetic, CurriculumLevel::L02InventoryUnits] {
                for split in [DatasetSplit::Train, DatasetSplit::HoldoutTemplate] {
                    for idx in 0..50 {
                        let s = generate_sample(seed, level, split, idx);
                        if s.expected_status != "VALID" { continue; }
                        let initial = s.gold_rir.initial_quantity.expect("Debe tener cantidad inicial");
                        let op_val = s.gold_rir.operations[0].operand;
                        let prompt_lower = s.prompt.to_lowercase();

                        // Verificar que 'initial' está en la frase (en dígitos o en palabras)
                        let init_dig = format!("{}", initial as i64);
                        let init_word = templates::num_to_text(initial as i64).unwrap_or_default();
                        assert!(
                            prompt_lower.contains(&init_dig) || (!init_word.is_empty() && prompt_lower.contains(&init_word)),
                            "Falla de concordancia en sample '{}': initial {} no encontrado en prompt '{}'",
                            s.id, initial, s.prompt
                        );

                        // Verificar que 'op_val' está en la frase (en dígitos o en palabras)
                        let op_dig = format!("{}", op_val as i64);
                        let op_word = templates::num_to_text(op_val as i64).unwrap_or_default();
                        assert!(
                            prompt_lower.contains(&op_dig) || (!op_word.is_empty() && prompt_lower.contains(&op_word)),
                            "Falla de concordancia en sample '{}': operand {} no encontrado en prompt '{}'",
                            s.id, op_val, s.prompt
                        );
                    }
                }
            }
        }
    }
}
