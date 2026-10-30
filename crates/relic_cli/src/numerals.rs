//! Reescritura de numerales en español y expansión de marcadores proporcionales/porcentuales a RPN.
/// Extrae una expresión balanceada o término numérico al inicio de `s`, devolviendo `(term, rest)`.
pub fn take_leading_term(s: &str) -> Option<(String, String)> {
    let trimmed = s.trim_start();
    if trimmed.is_empty() { return None; }
    if trimmed.starts_with('(') {
        let mut depth = 0i32;
        for (idx, ch) in trimmed.char_indices() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some((trimmed[..=idx].to_string(), trimmed[idx + 1..].to_string()));
                }
            }
        }
        return None;
    }
    let end = trimmed
        .find(|c: char| c.is_whitespace() || matches!(c, '+' | '-' | '*' | '/' | ')'))
        .unwrap_or(trimmed.len());
    if end == 0 { return None; }
    let token = &trimmed[..end];
    if token.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Some((token.to_string(), trimmed[end..].to_string()))
    } else {
        None
    }
}

/// Reescribe operadores proporcionales prefijos (`__HALF__`, `__DOUBLE__`, etc.) en postfijos RPN exactos.
pub fn expand_proportional_markers(mut s: String) -> String {
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
                let (before, after) = (&s[..pos], &s[pos + marker.len()..]);
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

/// Reescribe variaciones porcentuales y porcentajes directos en subexpresiones RPN.
pub fn expand_percentage_constructs(mut s: String) -> String {
    for (pat, op_ch) in [(" + __pct__ ", '+'), (" - __pct__ ", '-')] {
        while let Some(pos) = s.find(pat) {
            let left_part = s[..pos].trim_end();
            let right_part = s[pos + pat.len()..].trim_start();
            let base_tok = left_part.split_whitespace().last().unwrap_or("");
            if let Some((pct_tok, rest)) = take_leading_term(right_part) {
                if !base_tok.is_empty() && base_tok.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    let prefix = &left_part[..left_part.len() - base_tok.len()];
                    s = format!("{prefix}({base_tok} {op_ch} (({pct_tok} / 100) * {base_tok})){rest}");
                    continue;
                }
            }
            break;
        }
    }
    while let Some(pos) = s.find("__pct_of__") {
        let before = &s[..pos];
        let after = s[pos + "__pct_of__".len()..].trim_start();
        if let Some((x_tok, after_x)) = take_leading_term(after) {
            if let Some(after_de) = after_x.trim_start().strip_prefix("de ") {
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

/// Convierte un numeral básico en español a representación decimal ASCII.
pub fn map_spanish_numeral(w: &str) -> Option<&'static str> {
    Some(match w {
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
        _ => return None,
    })
}

pub fn is_numeric_or_numeral(w: &str) -> bool {
    let clean_w = w.trim_matches(|c| c == '(' || c == ')');
    !clean_w.is_empty()
        && (clean_w.chars().all(|c| c.is_ascii_digit() || c == '.')
            || map_spanish_numeral(clean_w).is_some()
            || matches!(
                clean_w,
                "__HALF__" | "__THIRD__" | "__QUARTER__" | "__DOUBLE__" | "__TRIPLE__" | "__pct_of__"
            ))
}

