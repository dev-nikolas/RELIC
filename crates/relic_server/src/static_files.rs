//! # relic_server::static_files
//!
//! Servidor de archivos estáticos para el build compilado del frontend (SPA).
//! Menos de 180 líneas.

use std::fs;
use std::path::Path;

pub fn get_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("woff") | Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

pub fn try_serve_static(base_dir: &Path, raw_path: &str) -> Option<(Vec<u8>, &'static str)> {
    let clean_path = raw_path.trim_start_matches('/');
    let target = if clean_path.is_empty() {
        base_dir.join("index.html")
    } else {
        base_dir.join(clean_path)
    };

    if target.is_file() {
        if let Ok(bytes) = fs::read(&target) {
            return Some((bytes, get_content_type(&target)));
        }
    }

    // Fallback SPA para rutas cliente (ej: /research, /settings)
    let index_file = base_dir.join("index.html");
    if index_file.is_file() && !clean_path.starts_with("api") && !clean_path.starts_with("v1") {
        if let Ok(bytes) = fs::read(&index_file) {
            return Some((bytes, "text/html; charset=utf-8"));
        }
    }

    None
}
