//! # relic_cli (Library Interface)
//!
//! Exporta los módulos de análisis sintáctico, despacho, inferencia y benchmarking.
//! Menos de 180 líneas.

pub mod bench;
pub mod demo;
pub mod dispatch;
pub mod graph;
pub mod knowledge;
pub mod math;
pub mod monitor;
pub mod parity;
pub mod query;

pub use dispatch::{parse_dispatch_intent, parse_inventory_problem, DispatchIntent, QuerySpec};
pub use graph::parse_query_to_spec;
pub use math::parse_math_expression;
