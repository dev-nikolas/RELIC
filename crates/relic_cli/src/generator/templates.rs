//! Catálogo de plantillas léxicas y estilos para Niveles 1 a 3.
//! Separación estricta de plantillas Train vs Hold-out. Menos de 180 líneas.

use super::types::DatasetSplit;

pub const NOUN_UNITS: &[(&str, &str)] = &[
    ("manzana", "manzanas"), ("lápiz", "lápices"), ("pan", "panes"),
    ("moneda", "monedas"), ("libro", "libros"), ("gato", "gatos"),
    ("perro", "perros"), ("caja", "cajas"), ("silla", "sillas"),
    ("camisa", "camisas"), ("cuaderno", "cuadernos"), ("árbol", "árboles"),
];

pub const SPANISH_NUMERALS: &[(i64, &str)] = &[
    (0, "cero"), (1, "uno"), (2, "dos"), (3, "tres"), (4, "cuatro"), (5, "cinco"),
    (6, "seis"), (7, "siete"), (8, "ocho"), (9, "nueve"), (10, "diez"), (11, "once"),
    (12, "doce"), (13, "trece"), (14, "catorce"), (15, "quince"), (16, "dieciséis"),
    (17, "diecisiete"), (18, "dieciocho"), (19, "diecinueve"), (20, "veinte"),
    (25, "veinticinco"), (30, "treinta"), (40, "cuarenta"), (50, "cincuenta"),
];

pub fn num_to_text(n: i64) -> Option<&'static str> {
    SPANISH_NUMERALS.iter().find(|(val, _)| *val == n).map(|(_, s)| *s)
}

#[derive(Debug, Clone, Copy)]
pub struct TemplateDef {
    pub id: &'static str,
    pub split: DatasetSplit,
    pub pattern: &'static str,
    pub op_kind: &'static str, // "add", "sub", "mul", "div", "incomplete", "unsupported"
}

pub const L01_TEMPLATES: &[TemplateDef] = &[
    TemplateDef { id: "T_L01_01", split: DatasetSplit::Train, pattern: "cuánto es {a} + {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_02", split: DatasetSplit::Train, pattern: "calcula {a} - {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_03", split: DatasetSplit::Train, pattern: "suma {a} y {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_04", split: DatasetSplit::Train, pattern: "multiplica {a} por {b}", op_kind: "mul" },
    TemplateDef { id: "T_L01_05", split: DatasetSplit::Train, pattern: "cuánto da {a} * {b}", op_kind: "mul" },
    TemplateDef { id: "T_L01_06", split: DatasetSplit::Train, pattern: "resta {a} y {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_07", split: DatasetSplit::Train, pattern: "cuánto es {a} dividido entre {b}", op_kind: "div" },
    TemplateDef { id: "T_L01_08", split: DatasetSplit::Train, pattern: "{a} más {b}", op_kind: "add" },
    // HOLD-OUT EXCLUSIVO DE PLANTILLA (Nunca presente en entrenamiento)
    TemplateDef { id: "H_L01_01", split: DatasetSplit::HoldoutTemplate, pattern: "a {b} quítale {a}", op_kind: "sub_inv" },
    TemplateDef { id: "H_L01_02", split: DatasetSplit::HoldoutTemplate, pattern: "resultado de sumar {a} e {b}", op_kind: "add" },
    TemplateDef { id: "H_L01_03", split: DatasetSplit::HoldoutTemplate, pattern: "{a} menos {b} son?", op_kind: "sub" },
    TemplateDef { id: "H_L01_04", split: DatasetSplit::HoldoutTemplate, pattern: "póngale {a} a {b}", op_kind: "add" },
    TemplateDef { id: "H_L01_05", split: DatasetSplit::HoldoutTemplate, pattern: "sáquele {a} a {b}", op_kind: "sub_inv" },
];

pub const L02_TEMPLATES: &[TemplateDef] = &[
    TemplateDef { id: "T_L02_01", split: DatasetSplit::Train, pattern: "tengo {a} {units} y compro {b}", op_kind: "add" },
    TemplateDef { id: "T_L02_02", split: DatasetSplit::Train, pattern: "tengo {a} {units} y vendo {b}", op_kind: "sub" },
    TemplateDef { id: "T_L02_03", split: DatasetSplit::Train, pattern: "tengo {a} {units} y recibo {b}", op_kind: "add" },
    TemplateDef { id: "T_L02_04", split: DatasetSplit::Train, pattern: "tengo {a} {units} y regalo {b}", op_kind: "sub" },
    TemplateDef { id: "T_L02_05", split: DatasetSplit::Train, pattern: "había {a} {units} y añadieron {b}", op_kind: "add" },
    TemplateDef { id: "T_L02_06", split: DatasetSplit::Train, pattern: "había {a} {units} y quitaron {b}", op_kind: "sub" },
    // HOLD-OUT EXCLUSIVO DE PLANTILLA
    TemplateDef { id: "H_L02_01", split: DatasetSplit::HoldoutTemplate, pattern: "de los {a} {units} que tenía, presté {b}", op_kind: "sub" },
    TemplateDef { id: "H_L02_02", split: DatasetSplit::HoldoutTemplate, pattern: "me gasté {b} de las {a} {units} que tenía", op_kind: "sub" },
    TemplateDef { id: "H_L02_03", split: DatasetSplit::HoldoutTemplate, pattern: "tenía {a} {units} pero me tumbaron {b}", op_kind: "sub" },
    TemplateDef { id: "H_L02_04", split: DatasetSplit::HoldoutTemplate, pattern: "llegaron {b} {units} a las {a} que ya tenía guardadas", op_kind: "add" },
];

pub const L03_TEMPLATES: &[TemplateDef] = &[
    TemplateDef { id: "T_L03_01", split: DatasetSplit::Train, pattern: "tengo algunas {units} y compro {b}", op_kind: "incomplete" },
    TemplateDef { id: "T_L03_02", split: DatasetSplit::Train, pattern: "tengo varios {units} y vendo {b}", op_kind: "incomplete" },
    TemplateDef { id: "T_L03_03", split: DatasetSplit::Train, pattern: "el cielo es de color azul", op_kind: "unsupported" },
    TemplateDef { id: "T_L03_04", split: DatasetSplit::Train, pattern: "quién descubrió el continente de América", op_kind: "unsupported" },
    TemplateDef { id: "T_L03_05", split: DatasetSplit::Train, pattern: "recibo {b} {units}", op_kind: "incomplete" },
    TemplateDef { id: "T_L03_06", split: DatasetSplit::Train, pattern: "cuántas {units} me quedan si regalo {b}", op_kind: "incomplete" },
    // HOLD-OUT EXCLUSIVO DE PLANTILLA
    TemplateDef { id: "H_L03_01", split: DatasetSplit::HoldoutTemplate, pattern: "un poco de {units} más {b}", op_kind: "incomplete" },
    TemplateDef { id: "H_L03_02", split: DatasetSplit::HoldoutTemplate, pattern: "a cuántos grados centígrados hierve el agua", op_kind: "unsupported" },
    TemplateDef { id: "H_L03_03", split: DatasetSplit::HoldoutTemplate, pattern: "la luna llena está hecha de queso", op_kind: "unsupported" },
    TemplateDef { id: "H_L03_04", split: DatasetSplit::HoldoutTemplate, pattern: "me dieron plata pero no me acuerdo cuánta y gasté {b}", op_kind: "incomplete" },
];
