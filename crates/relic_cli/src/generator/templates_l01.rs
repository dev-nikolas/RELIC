//! Plantillas para Nivel 1: Aritmética directa y variaciones sintácticas.
use super::types::DatasetSplit;

#[derive(Debug, Clone, Copy)]
pub struct TemplateDef {
    pub id: &'static str,
    pub split: DatasetSplit,
    pub pattern: &'static str,
    pub op_kind: &'static str,
}

pub const L01_TEMPLATES: &[TemplateDef] = &[
    // --- ENTRENAMIENTO (L01 Train: 36 plantillas) ---
    TemplateDef { id: "T_L01_01", split: DatasetSplit::Train, pattern: "cuánto es {a} + {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_02", split: DatasetSplit::Train, pattern: "calcula {a} - {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_03", split: DatasetSplit::Train, pattern: "suma {a} y {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_04", split: DatasetSplit::Train, pattern: "multiplica {a} por {b}", op_kind: "mul" },
    TemplateDef { id: "T_L01_05", split: DatasetSplit::Train, pattern: "cuánto da {a} * {b}", op_kind: "mul" },
    TemplateDef { id: "T_L01_06", split: DatasetSplit::Train, pattern: "resta {a} y {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_07", split: DatasetSplit::Train, pattern: "cuánto es {a} dividido entre {b}", op_kind: "div" },
    TemplateDef { id: "T_L01_08", split: DatasetSplit::Train, pattern: "{a} más {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_09", split: DatasetSplit::Train, pattern: "{a} mas {b}", op_kind: "add" }, // ruido
    TemplateDef { id: "T_L01_10", split: DatasetSplit::Train, pattern: "cuanto es {a} mas {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_11", split: DatasetSplit::Train, pattern: "{a} menos {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_12", split: DatasetSplit::Train, pattern: "calcule {a} + {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_13", split: DatasetSplit::Train, pattern: "calcule {a} - {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_14", split: DatasetSplit::Train, pattern: "calcule {a} * {b}", op_kind: "mul" },
    TemplateDef { id: "T_L01_15", split: DatasetSplit::Train, pattern: "obtén el valor de {a} + {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_16", split: DatasetSplit::Train, pattern: "obtén el valor de {a} - {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_17", split: DatasetSplit::Train, pattern: "dime el resultado de {a} + {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_18", split: DatasetSplit::Train, pattern: "dime el resultado de {a} - {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_19", split: DatasetSplit::Train, pattern: "total de {a} con {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_20", split: DatasetSplit::Train, pattern: "diferencia entre {a} y {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_21", split: DatasetSplit::Train, pattern: "producto de {a} y {b}", op_kind: "mul" },
    TemplateDef { id: "T_L01_22", split: DatasetSplit::Train, pattern: "división de {a} entre {b}", op_kind: "div" },
    TemplateDef { id: "T_L01_23", split: DatasetSplit::Train, pattern: "agregue {b} a {a}", op_kind: "add" },
    TemplateDef { id: "T_L01_24", split: DatasetSplit::Train, pattern: "sumar {a} y {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_25", split: DatasetSplit::Train, pattern: "restar {b} de {a}", op_kind: "sub" },
    TemplateDef { id: "T_L01_26", split: DatasetSplit::Train, pattern: "multiplicar {a} por {b}", op_kind: "mul" },
    TemplateDef { id: "T_L01_27", split: DatasetSplit::Train, pattern: "dividir {a} entre {b}", op_kind: "div" },
    TemplateDef { id: "T_L01_28", split: DatasetSplit::Train, pattern: "{a} adicionado con {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_29", split: DatasetSplit::Train, pattern: "{a} disminuido en {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_30", split: DatasetSplit::Train, pattern: "junta {a} con {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_31", split: DatasetSplit::Train, pattern: "sustrae {b} de {a}", op_kind: "sub" },
    TemplateDef { id: "T_L01_32", split: DatasetSplit::Train, pattern: "{a} por {b}", op_kind: "mul" },
    TemplateDef { id: "T_L01_33", split: DatasetSplit::Train, pattern: "{a} sobre {b}", op_kind: "div" },
    TemplateDef { id: "T_L01_34", split: DatasetSplit::Train, pattern: "cuánto queda si de {a} sacamos {b}", op_kind: "sub" },
    TemplateDef { id: "T_L01_35", split: DatasetSplit::Train, pattern: "calcula la suma de {a} e {b}", op_kind: "add" },
    TemplateDef { id: "T_L01_36", split: DatasetSplit::Train, pattern: "cuanto resulta {a} - {b}", op_kind: "sub" },
    // --- HOLDOUT POR PLANTILLA (L01 Holdout: 16 plantillas exclusivas) ---
    TemplateDef { id: "H_L01_01", split: DatasetSplit::HoldoutTemplate, pattern: "a {a} quítale {b}", op_kind: "sub" },
    TemplateDef { id: "H_L01_02", split: DatasetSplit::HoldoutTemplate, pattern: "resultado de sumar {a} e {b}", op_kind: "add" },
    TemplateDef { id: "H_L01_03", split: DatasetSplit::HoldoutTemplate, pattern: "{a} menos {b} son?", op_kind: "sub" },
    TemplateDef { id: "H_L01_04", split: DatasetSplit::HoldoutTemplate, pattern: "póngale {b} a {a}", op_kind: "add" },
    TemplateDef { id: "H_L01_05", split: DatasetSplit::HoldoutTemplate, pattern: "sáquele {b} a {a}", op_kind: "sub" },
    TemplateDef { id: "H_L01_06", split: DatasetSplit::HoldoutTemplate, pattern: "cuánto le falta a {b} para alcanzar {a}", op_kind: "sub" },
    TemplateDef { id: "H_L01_07", split: DatasetSplit::HoldoutTemplate, pattern: "móchele {b} a {a}", op_kind: "sub" }, // modismo
    TemplateDef { id: "H_L01_08", split: DatasetSplit::HoldoutTemplate, pattern: "métele {b} a los {a}", op_kind: "add" },
    TemplateDef { id: "H_L01_09", split: DatasetSplit::HoldoutTemplate, pattern: "si parto {a} en {b} pedazos iguales", op_kind: "div" },
    TemplateDef { id: "H_L01_10", split: DatasetSplit::HoldoutTemplate, pattern: "repartir {a} entre {b} porciones", op_kind: "div" },
    TemplateDef { id: "H_L01_11", split: DatasetSplit::HoldoutTemplate, pattern: "si a {a} le descontamos {b}", op_kind: "sub" },
    TemplateDef { id: "H_L01_12", split: DatasetSplit::HoldoutTemplate, pattern: "al valor {a} aplícale descuento de {b}", op_kind: "sub" },
    TemplateDef { id: "H_L01_13", split: DatasetSplit::HoldoutTemplate, pattern: "acumula {b} encima de {a}", op_kind: "add" },
    TemplateDef { id: "H_L01_14", split: DatasetSplit::HoldoutTemplate, pattern: "si triplicas no sino que haces {a} por {b}", op_kind: "mul" },
    TemplateDef { id: "H_L01_15", split: DatasetSplit::HoldoutTemplate, pattern: "bájale {b} rayitas a {a}", op_kind: "sub" },
    TemplateDef { id: "H_L01_16", split: DatasetSplit::HoldoutTemplate, pattern: "súmele {b} a ese {a}", op_kind: "add" },
];
