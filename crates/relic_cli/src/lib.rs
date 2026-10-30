//! # relic_cli (Library Interface)
//!
//! Exporta los módulos de análisis sintáctico, despacho, inferencia y (opcionalmente) herramientas offline.
pub mod answer;
pub mod demo;
pub mod dispatch;
pub mod graph;
pub mod inventory;
pub mod inverted;
pub mod knowledge;
pub mod math;
pub mod monitor;
pub mod normalize;
pub mod numerals;
pub mod parity;
pub mod query;
pub mod relational;

#[cfg(feature = "offline-tools")]
pub mod generator;
#[cfg(feature = "offline-tools")]
pub mod eval_holdout;

pub use answer::{answer_intent, is_generation_request, Answer};
pub use dispatch::{
    parse_dispatch_intent, parse_dispatch_intent_with_lexicon,
    parse_inventory_problem, parse_inventory_problem_with_lexicon,
    DispatchIntent, QuerySpec,
};
pub use graph::parse_query_to_spec;
pub use math::parse_math_expression;
pub use normalize::{normalize_query, normalize_query_with_lexicon};
pub use relational::{parse_relational_query, RelationalQuerySpec, RelationalQuestionType};

#[cfg(feature = "offline-tools")]
pub use generator::{generate_sample, generate_and_export_curriculum, CurriculumLevel, DatasetSplit};
pub use relic_tokenizer;

