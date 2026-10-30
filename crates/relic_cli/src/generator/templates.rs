//! Catálogo central de plantillas y numerales en español para el generador.
pub use super::templates_l01::{TemplateDef, L01_TEMPLATES};
pub use super::templates_l02::L02_TEMPLATES;
pub use super::templates_l03::L03_TEMPLATES;

pub const NOUN_UNITS: &[(&str, &str)] = &[
    ("manzana", "manzanas"), ("lápiz", "lápices"), ("pan", "panes"),
    ("moneda", "monedas"), ("libro", "libros"), ("gato", "gatos"),
    ("perro", "perros"), ("caja", "cajas"), ("silla", "sillas"),
    ("camisa", "camisas"), ("cuaderno", "cuadernos"), ("árbol", "árboles"),
    ("peseta", "pesetas"), ("computador", "computadores"), ("reloj", "relojes"),
    ("balón", "balones"), ("zapato", "zapatos"), ("botella", "botellas"),
];

/// Convierte enteros entre 0 y 99 a su representación textual exacta en español.
/// No retorna None para ningún valor en el rango 0..=99.
pub fn num_to_text(n: i64) -> Option<String> {
    if !(0..=99).contains(&n) { return None; }
    let s = match n {
        0 => "cero", 1 => "uno", 2 => "dos", 3 => "tres", 4 => "cuatro",
        5 => "cinco", 6 => "seis", 7 => "siete", 8 => "ocho", 9 => "nueve",
        10 => "diez", 11 => "once", 12 => "doce", 13 => "trece", 14 => "catorce",
        15 => "quince", 16 => "dieciséis", 17 => "diecisiete", 18 => "dieciocho",
        19 => "diecinueve", 20 => "veinte", 21 => "veintiuno", 22 => "veintidós",
        23 => "veintitrés", 24 => "veinticuatro", 25 => "veinticinco", 26 => "veintiséis",
        27 => "veintisiete", 28 => "veintiocho", 29 => "veintinueve", 30 => "treinta",
        40 => "cuarenta", 50 => "cincuenta", 60 => "sesenta", 70 => "setenta",
        80 => "ochenta", 90 => "noventa",
        _ => {
            let tens = match (n / 10) * 10 {
                30 => "treinta", 40 => "cuarenta", 50 => "cincuenta",
                60 => "sesenta", 70 => "setenta", 80 => "ochenta", 90 => "noventa",
                _ => return None,
            };
            let units = match n % 10 {
                1 => "uno", 2 => "dos", 3 => "tres", 4 => "cuatro", 5 => "cinco",
                6 => "seis", 7 => "siete", 8 => "ocho", 9 => "nueve",
                _ => return None,
            };
            return Some(format!("{tens} y {units}"));
        }
    };
    Some(s.to_string())
}
