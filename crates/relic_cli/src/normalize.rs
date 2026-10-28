//! Normalización léxica y sintáctica previa al parseo aritmético. Menos de 180 líneas.

use relic_core::ArithmeticOp;
use relic_lexicon::LexiconView;
use crate::numerals::{
    expand_percentage_constructs, expand_proportional_markers, is_numeric_or_numeral,
    map_spanish_numeral,
};

pub fn normalize_query(text: &str) -> String {
    normalize_query_with_lexicon(text, None)
}

pub fn normalize_query_with_lexicon(text: &str, lexicon: Option<&LexiconView>) -> String {
    let clean = text.trim_matches(|c: char| matches!(c, '¿' | '?' | '¡' | '!' | '.' | ';'));
    let mut s = clean.to_lowercase();

    if let Some(view) = lexicon {
        s = view.apply_phrase_macros(&s);
    }

    for prefix in [
        "cuánto es ", "cuanto es ", "cuánto da ", "cuanto da ", "calcula ", "resultado de ",
    ] {
        if s.starts_with(prefix) {
            s = s[prefix.len()..].trim().to_string();
            break;
        }
    }

    for rep_prefix in ["repartir ", "reparte ", "dividir ", "divide "] {
        if let Some(rest) = s.strip_prefix(rep_prefix) {
            if let Some((a, b)) = rest.split_once(" entre ").or_else(|| rest.split_once(" en ")) {
                s = format!("{} / {}", a.trim(), b.trim());
                break;
            }
        }
    }

    s = s.replace(" dividido por ", " / ")
        .replace(" dividido entre ", " / ")
        .replace(" dividido en ", " / ")
        .replace(" por ciento de ", "% de ")
        .replace(" porciento de ", "% de ")
        .replace(" por ciento", "%")
        .replace(" porciento", "%")
        .replace(" más el ", " + __pct__ ")
        .replace(" mas el ", " + __pct__ ")
        .replace(" menos el ", " - __pct__ ")
        .replace("la mitad de ", "__HALF__ ")
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
        while s.ends_with(suffix) { s.truncate(s.len() - suffix.len()); }
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

    let raw_words: Vec<&str> = s.split_whitespace().collect();
    let mut pre_words: Vec<String> = Vec::with_capacity(raw_words.len() + 4);
    for (idx, &w) in raw_words.iter().enumerate() {
        if let Some(num_part) = w.strip_suffix('%') {
            if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit() || c == '.') {
                if idx > 0 && raw_words[idx - 1] == "__pct__" {
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

    for (i, &w) in words.iter().enumerate() {
        if let Some(digit) = map_spanish_numeral(w) {
            mapped.push(digit.to_string());
            continue;
        }
        let between_nums = i > 0
            && is_numeric_or_numeral(words[i - 1])
            && words.get(i + 1).is_some_and(|next| is_numeric_or_numeral(next));

        let norm = match w {
            "más" | "mas" if between_nums => "+",
            "menos" if between_nums => "-",
            "por" if between_nums => "*",
            "entre" if between_nums => "/",
            other if between_nums => {
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
            }
            other => other,
        };
        mapped.push(norm.to_string());
    }

    expand_proportional_markers(expand_percentage_constructs(mapped.join(" ")))
}
