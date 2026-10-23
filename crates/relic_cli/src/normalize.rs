//! Normalización léxica y sintáctica liviana previa al parseo.
//! Transforma numerales verbales y giros aritméticos comunes a formas canónicas.
//! Menos de 180 líneas.

pub fn normalize_query(text: &str) -> String {
    let clean = text.trim_matches(|c: char| c == '¿' || c == '?' || c == '¡' || c == '!' || c == '.' || c == ';');
    let mut s = clean.to_lowercase();

    // Reemplazo de prefijos comunes de pregunta
    for prefix in ["cuánto es ", "cuanto es ", "cuánto da ", "cuanto da ", "calcula ", "resultado de "] {
        if s.starts_with(prefix) {
            s = s[prefix.len()..].trim().to_string();
            break;
        }
    }

    // Normalizar operadores verbales multi-palabra
    s = s.replace(" dividido por ", " / ")
         .replace(" dividido entre ", " / ")
         .replace(" dividido en ", " / ");

    // Normalizar copulativo final coloquial: "2 más 2 son", "5 menos 2 es".
    for suffix in [" son", " es"] {
        while s.ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
        }
    }

    // Normalizar formato imperativo: "suma X y Y" / "sumar X y Y"
    if s.starts_with("suma ") || s.starts_with("sumar ") {
        let rest = if s.starts_with("sumar ") { &s[6..] } else { &s[5..] };
        if let Some((a, b)) = rest.split_once(" y ") {
            s = format!("{} + {}", a.trim(), b.trim());
        } else if let Some((a, b)) = rest.split_once(" e ") {
            s = format!("{} + {}", a.trim(), b.trim());
        }
    } else if s.starts_with("resta ") || s.starts_with("restar ") {
        let rest = if s.starts_with("restar ") { &s[7..] } else { &s[6..] };
        if let Some((a, b)) = rest.split_once(" y ") {
            s = format!("{} - {}", a.trim(), b.trim());
        } else if let Some((a, b)) = rest.split_once(" a ") {
            s = format!("{} - {}", b.trim(), a.trim());
        }
    } else if s.starts_with("quita ") || s.starts_with("quitar ") || s.starts_with("quítale ") || s.starts_with("quitale ") {
        // "quítale 4 a 10" ⇒ 10 - 4: el infinitivo marcaría el minuendo tras el operador 'a'.
        let rest = &s[s.find(' ').map(|i| i + 1).unwrap_or(s.len())..];
        if let Some((a, b)) = rest.split_once(" a ") {
            s = format!("{} - {}", b.trim(), a.trim());
        }
    } else if s.starts_with("multiplica ") || s.starts_with("multiplicar ") {
        let rest = if s.starts_with("multiplicar ") { &s[12..] } else { &s[11..] };
        if let Some((a, b)) = rest.split_once(" por ") {
            s = format!("{} * {}", a.trim(), b.trim());
        } else if let Some((a, b)) = rest.split_once(" y ") {
            s = format!("{} * {}", a.trim(), b.trim());
        }
    }

    // Normalizar palabras aritméticas directas
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut mapped: Vec<String> = Vec::with_capacity(words.len());

    for w in words {
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
            "veinte" => "20",
            "más" | "mas" => "+",
            "menos" => "-",
            "por" => "*",
            "entre" => "/",
            other => other,
        };
        mapped.push(norm.to_string());
    }

    mapped.join(" ")
}
