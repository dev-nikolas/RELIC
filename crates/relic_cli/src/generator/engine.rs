//! Motor determinista por semilla y verificador independiente.
//! Menos de 180 líneas.

use relic_core::{ArithmeticOp, ArithmeticPlan, ArithmeticStep, RirArithmeticOp, RirDocumentV1, RirValidityStatus};
use super::types::{CurriculumLevel, CurriculumSample, DatasetSplit};
use super::templates::{num_to_text, NOUN_UNITS, TemplateDef, L01_TEMPLATES, L02_TEMPLATES, L03_TEMPLATES};

pub struct Rng64(u64);

impl Rng64 {
    pub fn new(seed: u64) -> Self { Self(if seed == 0 { 0x517cc1b727220a95 } else { seed }) }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn gen_range(&mut self, min: i64, max: i64) -> i64 {
        let span = (max - min + 1) as u64;
        min + (self.next_u64() % span) as i64
    }
}

pub fn generate_sample(seed: u64, level: CurriculumLevel, split: DatasetSplit, index: usize) -> CurriculumSample {
    let mut rng = Rng64::new(seed.wrapping_add(index as u64 * 104729));
    let t_list: &[TemplateDef] = match level {
        CurriculumLevel::L01DirectArithmetic => L01_TEMPLATES,
        CurriculumLevel::L02InventoryUnits => L02_TEMPLATES,
        CurriculumLevel::L03EpistemicRejection => L03_TEMPLATES,
    };
    let candidates: Vec<&TemplateDef> = t_list.iter().filter(|t| t.split == split).collect();
    let tmpl = candidates[(rng.next_u64() as usize) % candidates.len()];

    let a_num = rng.gen_range(2, 30);
    let b_num = rng.gen_range(1, 15);
    let noun = NOUN_UNITS[(rng.next_u64() as usize) % NOUN_UNITS.len()];
    let use_word_a = (rng.next_u64() % 3) == 0;
    let use_word_b = (rng.next_u64() % 3) == 0;

    let a_str = if use_word_a {
        num_to_text(a_num).unwrap_or_else(|| a_num.to_string())
    } else {
        a_num.to_string()
    };
    let b_str = if use_word_b {
        num_to_text(b_num).unwrap_or_else(|| b_num.to_string())
    } else {
        b_num.to_string()
    };

    let prompt = tmpl.pattern.replace("{a}", &a_str).replace("{b}", &b_str).replace("{units}", noun.1);

    let (gold_rir, status, ans) = match tmpl.op_kind {
        "add" => {
            let ops = vec![ArithmeticStep { operator: RirArithmeticOp::Add, operand: b_num as f64 }];
            let rir = RirDocumentV1::new_inventory(if level == CurriculumLevel::L02InventoryUnits { Some(noun.1.to_string()) } else { None }, a_num as f64, ops);
            let mut plan = ArithmeticPlan::new(a_num as f64, if level == CurriculumLevel::L02InventoryUnits { Some(noun.1.to_string()) } else { None });
            plan.add_step(ArithmeticOp::Add, b_num as f64);
            let out = plan.execute().unwrap_or(0.0);
            (rir, "VALID", plan.format_result(out))
        }
        "sub" => {
            let ops = vec![ArithmeticStep { operator: RirArithmeticOp::Subtract, operand: b_num as f64 }];
            let rir = RirDocumentV1::new_inventory(if level == CurriculumLevel::L02InventoryUnits { Some(noun.1.to_string()) } else { None }, a_num as f64, ops);
            let mut plan = ArithmeticPlan::new(a_num as f64, if level == CurriculumLevel::L02InventoryUnits { Some(noun.1.to_string()) } else { None });
            plan.add_step(ArithmeticOp::Subtract, b_num as f64);
            let out = plan.execute().unwrap_or(0.0);
            (rir, "VALID", plan.format_result(out))
        }
        "sub_inv" => {
            // "a b quítale a" => b - a
            let ops = vec![ArithmeticStep { operator: RirArithmeticOp::Subtract, operand: a_num as f64 }];
            let rir = RirDocumentV1::new_inventory(None, b_num as f64, ops);
            let mut plan = ArithmeticPlan::new(b_num as f64, None);
            plan.add_step(ArithmeticOp::Subtract, a_num as f64);
            let out = plan.execute().unwrap_or(0.0);
            (rir, "VALID", plan.format_result(out))
        }
        "mul" => {
            let ops = vec![ArithmeticStep { operator: RirArithmeticOp::Multiply, operand: b_num as f64 }];
            let rir = RirDocumentV1::new_inventory(None, a_num as f64, ops);
            let mut plan = ArithmeticPlan::new(a_num as f64, None);
            plan.add_step(ArithmeticOp::Multiply, b_num as f64);
            let out = plan.execute().unwrap_or(0.0);
            (rir, "VALID", plan.format_result(out))
        }
        "div" => {
            let b_div = if b_num == 0 { 2.0 } else { b_num as f64 };
            let ops = vec![ArithmeticStep { operator: RirArithmeticOp::Divide, operand: b_div }];
            let rir = RirDocumentV1::new_inventory(None, a_num as f64, ops);
            let mut plan = ArithmeticPlan::new(a_num as f64, None);
            plan.add_step(ArithmeticOp::Divide, b_div);
            let out = plan.execute().unwrap_or(0.0);
            (rir, "VALID", plan.format_result(out))
        }
        "incomplete" => {
            let rir = RirDocumentV1::new_rejected(RirValidityStatus::Incomplete, "Falta cantidad inicial", vec!["initial_quantity".to_string()]);
            (rir, "INCOMPLETE", "Rechazo epistémico: Falta cantidad inicial".to_string())
        }
        _ => {
            let rir = RirDocumentV1::new_rejected(RirValidityStatus::Unsupported, "Fuera de dominio formal", vec![]);
            (rir, "UNSUPPORTED", "Rechazo epistémico: Fuera de dominio formal".to_string())
        }
    };

    // Verificación semántica independiente formal
    if status == "VALID" {
        assert!(gold_rir.validate_semantic().is_ok(), "El RIR generado debe ser semánticamente válido");
    }

    CurriculumSample {
        id: format!("{}_{:?}_{:04}", tmpl.id, split, index),
        seed,
        level,
        split,
        template_id: tmpl.id.to_string(),
        prompt,
        gold_rir,
        expected_status: status.to_string(),
        oracle_answer: ans,
    }
}
