//! Normalización léxica y sintáctica previa al parseo aritmético.
//! Soporta operadores cotidianos proporcionales ("la mitad de", "el doble de", "X% de Y",
//! "Y más/menos el X%", "repartir X entre Y") y macros/alias de `LexiconView`.

use relic_core::ArithmeticOp;
use relic_lexicon::LexiconView;

pub fn normalize_query(text: &str) -> String {
    normalize_query_with_lexicon(text, None)
}

/// Extrae una expresión balanceada o término numérico al inicio de `s`, devolviendo `(term, rest)`.
fn take_leading_term(s: &str) -> Option<(String, String)> {
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('(') {
        let mut depth = 0i32;
        for (idx, ch) in trimmed.char_indices() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    let term = trimmed[..=idx].to_string();
                    let rest = trimmed[idx + 1..].to_string();
                    return Some((term, rest));
                }
            }
        }
        return None;
    }
    let end = trimmed
        .find(|c: char| c.is_whitespace() || matches!(c, '+' | '-' | '*' | '/' | ')'))
        .unwrap_or(trimmed.len());
    if end == 0 {
        return None;
    }
    let token = &trimmed[..end];
    if token.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Some((token.to_string(), trimmed[end..].to_string()))
    } else {
        None
    }
}

/// Reescribe operadores proporcionales prefijos (`__HALF__`, `__DOUBLE__`, etc.) en postfijos RPN exactos.
fn expand_proportional_markers(mut s: String) -> String {
    let markers = [
        ("__HALF__", "/ 2"),
        ("__THIRD__", "/ 3"),
        ("__QUARTER__", "/ 4"),
        ("__DOUBLE__", "* 2"),
        ("__TRIPLE__", "* 3"),
    ];
    let mut changed = true;
    while changed {
        changed = false;
        for &(marker, op_suffix) in &markers {
            if let Some(pos) = s.rfind(marker) {
                let before = &s[..pos];
                let after = &s[pos + marker.len()..];
                if let Some((term, rest)) = take_leading_term(after) {
                    s = format!("{before}({term} {op_suffix}){rest}");
                    changed = true;
                    break;
                }
            }
        }
    }
    s
}

/// Reescribe variaciones porcentuales (`Y + __PCT__ X` -> `Y + ((X / 100) * Y)`) y porcentajes directos (`X% de Y` -> `((X / 100) * Y)`).
fn expand_percentage_constructs(mut s: String) -> String {
    // 1. Variación porcentual: "<base> + __PCT__ <pct>" o "<base> - __PCT__ <pct>"
    for (pat, op_ch) in [(" + __pct__ ", '+'), (" - __pct__ ", '-')] {
        while let Some(pos) = s.find(pat) {
            let left_part = s[..pos].trim_end();
            let right_part = s[pos + pat.len()..].trim_start();
            let base_tok = left_part.split_whitespace().last().unwrap_or("");
            if let Some((pct_tok, rest)) = take_leading_term(right_part) {
                if !base_tok.is_empty() && base_tok.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    let prefix_len = left_part.len() - base_tok.len();
                    let prefix = &left_part[..prefix_len];
                    s = format!("{prefix}({base_tok} {op_ch} (({pct_tok} / 100) * {base_tok})){rest}");
                    continue;
                }
            }
            break;
        }
    }

    // 2. Porcentaje directo: "__PCT_OF__ <X> __DE__ <Y>" -> "((<X> / 100) * <Y>)"
    while let Some(pos) = s.find("__pct_of__") {
        let before = &s[..pos];
        let after = s[pos + "__pct_of__".len()..].trim_start();
        if let Some((x_tok, after_x)) = take_leading_term(after) {
            let after_x_trim = after_x.trim_start();
            if let Some(after_de) = after_x_trim.strip_prefix("de ") {
                if let Some((y_tok, rest)) = take_leading_term(after_de) {
                    s = format!("{before}(({x_tok} / 100) * {y_tok}){rest}");
                    continue;
                }
            }
        }
        break;
    }

    s
}

pub fn normalize_query_with_lexicon(text: &str, lexicon: Option<&LexiconView>) -> String {
    let clean = text.trim_matches(|c: char| matches!(c, '¿' | '?' | '¡' | '!' | '.' | ';'));
    let mut s = clean.to_lowercase();

    // Aplicar PhraseMacros aprendidas en el léxico antes de normalizar
    if let Some(view) = lexicon {
        s = view.apply_phrase_macros(&s);
    }

    for prefix in [
        "cuánto es ", "cuanto es ", "cuánto da ", "cuanto da ",
        "calcula ", "resultado de ",
    ] {
        if s.starts_with(prefix) {
            s = s[prefix.len()..].trim().to_string();
            break;
        }
    }

    // Reparto explícito: "repartir X entre Y" / "reparte X entre Y" / "dividir X entre Y"
    for rep_prefix in ["repartir ", "reparte ", "dividir ", "divide "] {
        if let Some(rest) = s.strip_prefix(rep_prefix) {
            if let Some((a, b)) = rest.split_once(" entre ").or_else(|| rest.split_once(" en ")) {
                s = format!("{} / {}", a.trim(), b.trim());
                break;
            }
        }
    }

    // Normalizar operadores verbales multi-palabra
    s = s.replace(" dividido por ", " / ")
        .replace(" dividido entre ", " / ")
        .replace(" dividido en ", " / ")
        .replace(" por ciento de ", "% de ")
        .replace(" porciento de ", "% de ")
        .replace(" por ciento", "%")
        .replace(" porciento", "%");

    // Normalizar variaciones porcentuales: "más el X%", "menos el X%"
    s = s.replace(" más el ", " + __pct__ ")
        .replace(" mas el ", " + __pct__ ")
        .replace(" menos el ", " - __pct__ ");

    // Marcar operadores proporcionales cotidianos para transformar a postfijo RPN exacto
    s = s.replace("la mitad de ", "__HALF__ ")
        .replace("mitad de ", "__HALF__ ")
        .replace("un tercio de ", "__THIRD__ ")
        .replace("la tercera parte de ", "__THIRD__ ")
        .replace("un cuarto de ", "__QUARTER__ ")
        .replace("la cuarta parte de ", "__QUARTER__ ")
        .replace("el doble de ", "__DOUBLE__ ")
        .replace("doble de ", "__DOUBLE__ ")
        .replace("el triple de ", "__TRIPLE__ ")
        .replace("triple de ", "__TRIPLE__ ");

    for suffix in [" son", " es"] {
        while s.ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
        }
    }

    if let Some(rest) = s.strip_prefix("sumar ").or_else(|| s.strip_prefix("suma ")) {
        if let Some((a, b)) = rest.split_once(" y ").or_else(|| rest.split_once(" e ")) {
            s = format!("{} + {}", a.trim(), b.trim());
        }
    } else if let Some(rest) = s.strip_prefix("restar ").or_else(|| s.strip_prefix("resta ")) {
        if let Some((a, b)) = rest.split_once(" y ") {
            s = format!("{} - {}", a.trim(), b.trim());
        } else if let Some((a, b)) = rest.split_once(" a ") {
            s = format!("{} - {}", b.trim(), a.trim());
        }
    } else if s.starts_with("quita ") || s.starts_with("quitar ") || s.starts_with("quítale ") || s.starts_with("quitale ") {
        let rest = &s[s.find(' ').map(|i| i + 1).unwrap_or(s.len())..];
        if let Some((a, b)) = rest.split_once(" a ") {
            s = format!("{} - {}", b.trim(), a.trim());
        }
    } else if let Some(rest) = s.strip_prefix("multiplicar ").or_else(|| s.strip_prefix("multiplica ")) {
        if let Some((a, b)) = rest.split_once(" por ").or_else(|| rest.split_once(" y ")) {
            s = format!("{} * {}", a.trim(), b.trim());
        }
    }

    // Convertir tokens "X%" en "__pct_of__ X" cuando precede a "de", o simplemente "X" tras "__pct__"
    let raw_words: Vec<&str> = s.split_whitespace().collect();
    let mut pre_words: Vec<String> = Vec::with_capacity(raw_words.len() + 4);
    for (idx, &w) in raw_words.iter().enumerate() {
        if let Some(num_part) = w.strip_suffix('%') {
            if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit() || c == '.') {
                let prev_is_pct_var = idx > 0 && raw_words[idx - 1] == "__pct__";
                if prev_is_pct_var {
                    pre_words.push(num_part.to_string());
                } else {
                    pre_words.push("__pct_of__".to_string());
                    pre_words.push(num_part.to_string());
                }
                continue;
            }
        }
        pre_words.push(w.to_string());
    }

    let words: Vec<&str> = pre_words.iter().map(|s| s.as_str()).collect();
    let mut mapped: Vec<String> = Vec::with_capacity(words.len());

    let is_numeric_or_numeral = |w: &str| -> bool {
        let clean_w = w.trim_matches(|c| c == '(' || c == ')');
        !clean_w.is_empty()
            && (clean_w.chars().all(|c| c.is_ascii_digit() || c == '.')
                || matches!(
                    clean_w,
                    "__HALF__" | "__THIRD__" | "__QUARTER__" | "__DOUBLE__" | "__TRIPLE__" | "__pct_of__"
                        | "cero" | "un" | "uno" | "una" | "dos" | "tres" | "cuatro" | "cinco" | "seis"
                        | "siete" | "ocho" | "nueve" | "diez" | "once" | "doce" | "trece" | "catorce"
                        | "quince" | "dieciséis" | "dieciseis" | "diecisiete" | "dieciocho" | "diecinueve"
                        | "veinte" | "veintiuno" | "veintiún" | "veintiun" | "veintidós" | "veintidos"
                        | "veintitrés" | "veintitres" | "veinticuatro" | "veinticinco" | "veintiséis"
                        | "veintiseis" | "veintisiete" | "veintiocho" | "veintinueve" | "treinta"
                        | "cuarenta" | "cincuenta" | "sesenta" | "setenta" | "ochenta" | "noventa" | "cien"
                ))
    };

    for (i, &w) in words.iter().enumerate() {
        let norm = match w {
            "cero" => "0",
            "un" | "uno" | "una" => "1",
            "dos" => "2",
            "tres" => "3",
            "cuatro" => "4",
            "cinco" => "5",
            "seis" => "6",
            "siete" => "7",
            "ocho" => "8",
            "nueve" => "9",
            "diez" => "10",
            "once" => "11",
            "doce" => "12",
            "trece" => "13",
            "catorce" => "14",
            "quince" => "15",
            "dieciséis" | "dieciseis" => "16",
            "diecisiete" => "17",
            "dieciocho" => "18",
            "diecinueve" => "19",
            "veinte" => "20",
            "veintiuno" | "veintiún" | "veintiun" => "21",
            "veintidós" | "veintidos" => "22",
            "veintitrés" | "veintitres" => "23",
            "veinticuatro" => "24",
            "veinticinco" => "25",
            "veintiséis" | "veintiseis" => "26",
            "veintisiete" => "27",
            "veintiocho" => "28",
            "veintinueve" => "29",
            "treinta" => "30",
            "cuarenta" => "40",
            "cincuenta" => "50",
            "sesenta" => "60",
            "setenta" => "70",
            "ochenta" => "80",
            "noventa" => "90",
            "cien" => "100",
            "más" | "mas" => {
                let prev_num = i > 0 && is_numeric_or_numeral(words[i - 1]);
                let next_num = words.get(i + 1).is_some_and(|next| is_numeric_or_numeral(next));
                if prev_num && next_num { "+" } else { w }
            }
            "menos" => {
                let prev_num = i > 0 && is_numeric_or_numeral(words[i - 1]);
                let next_num = words.get(i + 1).is_some_and(|next| is_numeric_or_numeral(next));
                if prev_num && next_num { "-" } else { w }
            }
            "por" => {
                let prev_num = i > 0 && is_numeric_or_numeral(words[i - 1]);
                let next_num = words.get(i + 1).is_some_and(|next| is_numeric_or_numeral(next));
                if prev_num && next_num { "*" } else { w }
            }
            "entre" => {
                let prev_num = i > 0 && is_numeric_or_numeral(words[i - 1]);
                let next_num = words.get(i + 1).is_some_and(|next| is_numeric_or_numeral(next));
                if prev_num && next_num { "/" } else { w }
            }
            other => {
                let prev_num = i > 0 && is_numeric_or_numeral(words[i - 1]);
                let next_num = words.get(i + 1).is_some_and(|next| is_numeric_or_numeral(next));
                if prev_num && next_num {
                    if let Some(op) = lexicon.and_then(|view| view.resolve_op_alias(other)) {
                        match op {
                            ArithmeticOp::Add => "+",
                            ArithmeticOp::Subtract => "-",
                            ArithmeticOp::Multiply => "*",
                            ArithmeticOp::Divide => "/",
                        }
                    } else {
                        other
                    }
                } else {
                    other
                }
            }
        };
        mapped.push(norm.to_string());
    }

    let joined = mapped.join(" ");
    let with_pct = expand_percentage_constructs(joined);
    expand_proportional_markers(with_pct)
}
