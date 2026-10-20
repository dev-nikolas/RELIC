//! Mapeo biyectivo byte-a-caracter para BPE a nivel de bytes.
//! Garantiza paridad total con Hugging Face ByteLevel sin dependencias externas.
//! Menos de 180 lineas.

use std::collections::HashMap;

/// Construye la tabla biyectiva de mapeo de bytes (0..=255) a caracteres Unicode.
pub fn bytes_to_unicode() -> (HashMap<u8, char>, HashMap<char, u8>) {
    let mut byte_to_char = HashMap::with_capacity(256);
    let mut char_to_byte = HashMap::with_capacity(256);

    let mut direct_ranges = Vec::new();
    // '!'..='~'
    for b in 33u8..=126 {
        direct_ranges.push(b);
    }
    // '¡'..='¬'
    for b in 161u8..=172 {
        direct_ranges.push(b);
    }
    // '®'..='ÿ'
    for b in 174u8..=255 {
        direct_ranges.push(b);
    }

    let mut shift: u32 = 0;
    for b in 0u8..=255 {
        if direct_ranges.contains(&b) {
            let c = b as char;
            byte_to_char.insert(b, c);
            char_to_byte.insert(c, b);
        } else {
            let c = std::char::from_u32(256 + shift).expect("Unicode code point valido");
            shift += 1;
            byte_to_char.insert(b, c);
            char_to_byte.insert(c, b);
        }
    }

    (byte_to_char, char_to_byte)
}

/// Codifica una cadena UTF-8 en su representacion de caracteres ByteLevel.
pub fn text_to_bytelevel(text: &str, byte_to_char: &HashMap<u8, char>) -> String {
    text.as_bytes()
        .iter()
        .map(|b| byte_to_char.get(b).copied().unwrap_or('?'))
        .collect()
}

/// Decodifica una secuencia de caracteres ByteLevel a una cadena UTF-8 original.
pub fn bytelevel_to_text(text: &str, char_to_byte: &HashMap<char, u8>) -> Result<String, std::string::FromUtf8Error> {
    let bytes: Vec<u8> = text
        .chars()
        .filter_map(|c| char_to_byte.get(&c).copied())
        .collect();
    String::from_utf8(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bijection_all_bytes() {
        let (b2c, c2b) = bytes_to_unicode();
        assert_eq!(b2c.len(), 256);
        assert_eq!(c2b.len(), 256);
        for b in 0u8..=255 {
            let c = b2c.get(&b).unwrap();
            let recovered = c2b.get(c).unwrap();
            assert_eq!(&b, recovered);
        }
    }

    #[test]
    fn test_roundtrip_ascii_and_spaces() {
        let (b2c, c2b) = bytes_to_unicode();
        let original = "Hello, world! 123";
        let encoded = text_to_bytelevel(original, &b2c);
        let decoded = bytelevel_to_text(&encoded, &c2b).unwrap();
        assert_eq!(original, decoded);
    }
}
