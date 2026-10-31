//! # Constantes y Parámetros del Predictor ESN
//!
//! Dimensiones de incrustación de Takens y parámetros del filtro RLS recursivo.

pub const EMBED_DIM: usize = 16;
pub const RESERVOIR_SIZE: usize = 64;
pub const NORM_WINDOW: usize = 6400;
pub const RLS_FORGET: f64 = 0.999;
pub const RLS_DELTA: f64 = 1.0;

