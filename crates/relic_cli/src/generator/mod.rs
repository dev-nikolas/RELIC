//! Módulo de generación y verificación curricular para Niveles 1 a 3.
//! Menos de 180 líneas.

pub mod types;
pub mod templates;
pub mod engine;
pub mod sha256;
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
}
