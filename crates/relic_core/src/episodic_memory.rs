//! # EpisodicMemory
//!
//! Almacenamiento explícito y determinista de hechos relacionales en grafos.
//! Estructuras de datos indexadas en memoria pura (`HashMap`, `HashSet`).
//! Cero acoplamiento neuronal, tiempo de consulta O(1).

use std::collections::{HashMap, HashSet};

/// Memoria episódica explícita para registrar y consultar hechos relacionales.
#[derive(Debug, Clone, Default)]
pub struct EpisodicMemory {
    adj: HashMap<String, Vec<String>>,
    edges: HashSet<(String, String)>,
    query_counter: usize,
}

impl EpisodicMemory {
    /// Inicializa una memoria episódica vacía.
    pub fn new() -> Self {
        Self {
            adj: HashMap::new(),
            edges: HashSet::new(),
            query_counter: 0,
        }
    }

    /// Limpia todos los hechos registrados y reinicia el contador de consultas.
    pub fn clear(&mut self) {
        self.adj.clear();
        self.edges.clear();
        self.query_counter = 0;
    }

    /// Inserta un hecho relacional dirigido `u -> v`.
    /// Descarta aristas duplicadas automáticamente.
    pub fn insert_edge(&mut self, u: impl Into<String>, v: impl Into<String>) -> bool {
        let u_str = u.into();
        let v_str = v.into();
        let pair = (u_str.clone(), v_str.clone());

        if self.edges.insert(pair) {
            self.adj.entry(u_str).or_default().push(v_str);
            true
        } else {
            false
        }
    }

    /// Comprueba si existe un hecho relacional directo `u -> v`.
    pub fn has_edge(&mut self, u: &str, v: &str) -> bool {
        self.query_counter += 1;
        self.edges.contains(&(u.to_string(), v.to_string()))
    }

    /// Comprueba si un nodo existe en la memoria (como origen o como destino de alguna arista).
    pub fn contains_node(&self, node: &str) -> bool {
        self.adj.contains_key(node) || self.edges.iter().any(|(u, v)| u == node || v == node)
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

    /// Número total de aristas dirigidas únicas registradas.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Total de consultas direccionables realizadas a la memoria.
    pub fn total_queries(&self) -> usize {
        self.query_counter
    }
}
