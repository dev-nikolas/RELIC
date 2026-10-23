//! # EpisodicMemory
//!
//! Almacenamiento explícito y determinista de hechos relacionales en grafos.
//! Estructuras de datos indexadas en memoria pura (`HashMap`, `HashSet`).
//! Cero acoplamiento neuronal, tiempo de consulta O(1).

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeResolution<'a> {
    Exact(&'a str),
    SingleAlias(&'a str),
    Ambiguous(Vec<String>),
    NotFound,
}

/// Memoria episódica explícita para registrar y consultar hechos relacionales.
#[derive(Debug, Clone, Default)]
pub struct EpisodicMemory {
    adj: HashMap<String, Vec<String>>,
    edges: HashSet<(String, String)>,
    nodes: HashSet<String>,
    canonical_map: HashMap<String, Vec<String>>,
    query_counter: usize,
}

fn canonical_node_key(s: &str) -> String {
    s.trim().to_lowercase()
        .replace('á', "a")
        .replace('é', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ú', "u")
}

impl EpisodicMemory {
    /// Inicializa una memoria episódica vacía.
    pub fn new() -> Self {
        Self {
            adj: HashMap::new(),
            edges: HashSet::new(),
            nodes: HashSet::new(),
            canonical_map: HashMap::new(),
            query_counter: 0,
        }
    }

    /// Limpia todos los hechos registrados y reinicia el contador de consultas.
    pub fn clear(&mut self) {
        self.adj.clear();
        self.edges.clear();
        self.nodes.clear();
        self.canonical_map.clear();
        self.query_counter = 0;
    }

    /// Inserta un hecho relacional dirigido `u -> v`.
    /// Descarta aristas duplicadas automáticamente.
    pub fn insert_edge(&mut self, u: impl Into<String>, v: impl Into<String>) -> bool {
        let u_str = u.into();
        let v_str = v.into();
        self.nodes.insert(u_str.clone());
        self.nodes.insert(v_str.clone());

        let cu = canonical_node_key(&u_str);
        let lu = self.canonical_map.entry(cu).or_default();
        if !lu.contains(&u_str) { lu.push(u_str.clone()); }

        let cv = canonical_node_key(&v_str);
        let lv = self.canonical_map.entry(cv).or_default();
        if !lv.contains(&v_str) { lv.push(v_str.clone()); }

        let pair = (u_str.clone(), v_str.clone());
        if self.edges.insert(pair) {
            self.adj.entry(u_str).or_default().push(v_str);
            true
        } else {
            false
        }
    }

    /// Resuelve un nodo detectando coincidencia exacta, alias único o ambigüedad por colisión.
    pub fn resolve_node(&self, node: &str) -> NodeResolution<'_> {
        if let Some(orig) = self.nodes.get(node) {
            return NodeResolution::Exact(orig.as_str());
        }
        let canon = canonical_node_key(node);
        match self.canonical_map.get(&canon) {
            None => NodeResolution::NotFound,
            Some(candidates) if candidates.len() == 1 => NodeResolution::SingleAlias(&candidates[0]),
            Some(candidates) => NodeResolution::Ambiguous(candidates.clone()),
        }
    }

    /// Comprueba si existe un hecho relacional directo `u -> v`.
    pub fn has_edge(&mut self, u: &str, v: &str) -> bool {
        self.query_counter += 1;
        self.edges.contains(&(u.to_string(), v.to_string()))
    }

    /// Comprueba si un nodo existe unívocamente en la memoria.
    pub fn contains_node(&self, node: &str) -> bool {
        matches!(self.resolve_node(node), NodeResolution::Exact(_) | NodeResolution::SingleAlias(_))
    }

    /// Consulta direccionable: devuelve la lista de vecinos alcanzables directamente desde `node`.
    pub fn get_neighbors(&mut self, node: &str) -> &[String] {
        self.query_counter += 1;
        self.adj.get(node).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Parsea e indexa hechos relacionales desde texto en formato 'Grafo: N0->N1, N1->N5. ¿Camino ...'
    pub fn load_from_text(&mut self, text: &str) -> usize {
        let edges_part = if let Some(idx) = text.find(". ¿Camino ") {
            &text[..idx]
        } else {
            text
        };

        let cleaned = edges_part.replace("Grafo:", "").replace(',', " ");
        let mut count = 0;

        for token in cleaned.split_whitespace() {
            let clean_tok = token.trim_matches(|c| c == '.' || c == ';' || c == ',');
            if let Some((u, v)) = clean_tok.split_once("->") {
                if !u.is_empty() && !v.is_empty() && !v.ends_with('?') {
                    if self.insert_edge(u, v) {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Número total de nodos únicos de origen registrados.
    pub fn node_count(&self) -> usize {
        self.adj.len()
    }

    /// Número total de nodos únicos (origen y destino) registrados.
    pub fn total_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Número total de aristas dirigidas únicas registradas.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Total de consultas direccionables realizadas a la memoria.
    pub fn total_queries(&self) -> usize {
        self.query_counter
    }
}
