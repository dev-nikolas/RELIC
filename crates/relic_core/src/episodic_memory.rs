//! # EpisodicMemory
//!
//! Almacenamiento explícito y determinista de hechos relacionales en grafos.
//! Estructuras de datos indexadas en memoria pura (`HashMap`, `HashSet`) con pesos de refuerzo.
//! Cero acoplamiento neuronal, tiempo de consulta O(1).

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeResolution<'a> {
    Exact(&'a str),
    SingleAlias(&'a str),
    Ambiguous(Vec<String>),
    NotFound,
}

/// Metadatos de refuerzo asociativo de una arista dirigida (`u -> v`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeStats {
    pub weight: f64,
    pub usage_count: u64,
}

impl Default for EdgeStats {
    fn default() -> Self {
        Self {
            weight: 1.0,
            usage_count: 0,
        }
    }
}

/// Memoria episódica explícita para registrar y consultar hechos relacionales.
#[derive(Debug, Clone, Default)]
pub struct EpisodicMemory {
    adj: HashMap<String, Vec<String>>,
    edges: HashSet<(String, String)>,
    edge_stats: HashMap<(String, String), EdgeStats>,
    nodes: HashSet<String>,
    canonical_map: HashMap<String, Vec<String>>,
    explicit_aliases: HashMap<String, Vec<String>>,
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
            edge_stats: HashMap::new(),
            nodes: HashSet::new(),
            canonical_map: HashMap::new(),
            explicit_aliases: HashMap::new(),
            query_counter: 0,
        }
    }

    /// Limpia todos los hechos registrados y reinicia el contador de consultas.
    pub fn clear(&mut self) {
        self.adj.clear();
        self.edges.clear();
        self.edge_stats.clear();
        self.nodes.clear();
        self.canonical_map.clear();
        self.explicit_aliases.clear();
        self.query_counter = 0;
    }

    /// Inserta un hecho relacional dirigido `u -> v` con peso inicial `1.0`.
    pub fn insert_edge(&mut self, u: impl Into<String>, v: impl Into<String>) -> bool {
        self.insert_edge_with_weight(u, v, 1.0)
    }

    /// Inserta un hecho relacional dirigido `u -> v` con un peso explícito `weight`.
    pub fn insert_edge_with_weight(
        &mut self,
        u: impl Into<String>,
        v: impl Into<String>,
        weight: f64,
    ) -> bool {
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
        let clamped_w = weight.max(0.0);
        if self.edges.insert(pair.clone()) {
            self.adj.entry(u_str).or_default().push(v_str);
            self.edge_stats.insert(pair, EdgeStats { weight: clamped_w, usage_count: 0 });
            true
        } else {
            if let Some(stats) = self.edge_stats.get_mut(&pair) {
                stats.weight = clamped_w;
            }
            false
        }
    }

    /// Refuerza el peso asociativo de una arista `u -> v` sumando `delta` e incrementando su contador de uso.
    pub fn reinforce_edge(&mut self, u: &str, v: &str, delta: f64) -> Option<f64> {
        let key = (u.to_string(), v.to_string());
        let stats = self.edge_stats.get_mut(&key)?;
        stats.weight = (stats.weight + delta).max(0.0);
        stats.usage_count += 1;
        Some(stats.weight)
    }

    /// Fija el peso y contador de uso de una arista existente `u -> v`.
    pub fn set_edge_stats(&mut self, u: &str, v: &str, weight: f64, usage_count: u64) -> bool {
        let key = (u.to_string(), v.to_string());
        if let Some(stats) = self.edge_stats.get_mut(&key) {
            stats.weight = weight.max(0.0);
            stats.usage_count = usage_count;
            true
        } else {
            false
        }
    }

    /// Obtiene el peso actual de una arista `u -> v` (por defecto `1.0` si existe).
    pub fn edge_weight(&self, u: &str, v: &str) -> Option<f64> {
        self.edge_stats
            .get(&(u.to_string(), v.to_string()))
            .map(|s| s.weight)
    }

    /// Obtiene las estadísticas completas (`weight`, `usage_count`) de una arista `u -> v`.
    pub fn edge_stats(&self, u: &str, v: &str) -> Option<&EdgeStats> {
        self.edge_stats.get(&(u.to_string(), v.to_string()))
    }

    /// Devuelve la lista determinista de vecinos de `node` junto con sus pesos de arista.
    pub fn neighbors_with_weights(&self, node: &str) -> Vec<(String, f64)> {
        let Some(neighbors) = self.adj.get(node) else {
            return Vec::new();
        };
        let mut out: Vec<(String, f64)> = neighbors
            .iter()
            .map(|n| {
                let w = self.edge_weight(node, n).unwrap_or(1.0);
                (n.clone(), w)
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Registra un alias léxico explícito hacia una entidad canónica del grafo.
    pub fn register_entity_alias(&mut self, alias: impl Into<String>, canonical: impl Into<String>) {
        let alias_str = alias.into();
        let canonical_str = canonical.into();
        let exact_list = self.explicit_aliases.entry(alias_str.clone()).or_default();
        if !exact_list.contains(&canonical_str) {
            exact_list.push(canonical_str.clone());
        }
        let key = canonical_node_key(&alias_str);
        let list = self.canonical_map.entry(key).or_default();
        if !list.contains(&canonical_str) {
            list.push(canonical_str);
        }
    }

    /// Resuelve un nodo detectando coincidencia exacta, alias único o ambigüedad por colisión.
    pub fn resolve_node(&self, node: &str) -> NodeResolution<'_> {
        if let Some(orig) = self.nodes.get(node) {
            if let Some(aliased) = self.explicit_aliases.get(node) {
                let mut candidates = vec![orig.clone()];
                for a in aliased {
                    if !candidates.contains(a) {
                        candidates.push(a.clone());
                    }
                }
                if candidates.len() > 1 {
                    return NodeResolution::Ambiguous(candidates);
                }
            }
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
        self.adj.get(u).is_some_and(|neighbors| neighbors.iter().any(|n| n == v))
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
                if !u.is_empty() && !v.is_empty() && !v.ends_with('?') && self.insert_edge(u, v) {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn node_count(&self) -> usize { self.adj.len() }
    pub fn total_node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }
    pub fn total_queries(&self) -> usize { self.query_counter }
}
