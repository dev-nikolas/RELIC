//! # EpisodicMemory
//!
//! Almacenamiento explícito y determinista de hechos relacionales en grafos con pesos de refuerzo.
//! Menos de 180 líneas.

use std::collections::{HashMap, HashSet};
use crate::interner::NodeInterner;
pub use crate::node_resolution::{canonical_node_key, EdgeStats, NodeResolution, NodeResolver};

#[derive(Debug, Clone, Default)]
pub struct EpisodicMemory {
    pub interner: NodeInterner,
    adj_fwd: Vec<Vec<u32>>,
    adj_bwd: Vec<Vec<u32>>,
    edges: HashSet<(u32, u32)>,
    edge_stats: HashMap<(u32, u32), EdgeStats>,
    pub edge_world_overrides: HashMap<(u32, u32), u64>,
    resolver: NodeResolver,
    query_counter: usize,
    query_cache: Vec<String>,
}

impl EpisodicMemory {
    pub fn new() -> Self { Self::default() }

    pub fn clear(&mut self) {
        self.interner.clear(); self.adj_fwd.clear(); self.adj_bwd.clear();
        self.edges.clear(); self.edge_stats.clear(); self.resolver.clear();
        self.edge_world_overrides.clear(); self.query_counter = 0; self.query_cache.clear();
    }

    pub fn insert_edge(&mut self, u: impl Into<String>, v: impl Into<String>) -> bool {
        self.insert_edge_with_weight(u, v, 1.0)
    }

    fn ensure_adj_capacity(&mut self, id: u32) {
        let needed = id as usize + 1;
        if self.adj_fwd.len() < needed { self.adj_fwd.resize_with(needed, Vec::new); }
        if self.adj_bwd.len() < needed { self.adj_bwd.resize_with(needed, Vec::new); }
    }

    pub fn insert_edge_with_weight(&mut self, u: impl Into<String>, v: impl Into<String>, weight: f64) -> bool {
        let (u_str, v_str) = (u.into(), v.into());
        let (u_id, v_id) = (self.interner.intern(&u_str), self.interner.intern(&v_str));
        self.resolver.activate(u_id, &u_str, &mut self.interner);
        self.resolver.activate(v_id, &v_str, &mut self.interner);
        self.ensure_adj_capacity(u_id.max(v_id));

        let pair = (u_id, v_id);
        let clamped_w = weight.max(0.0);
        if self.edges.insert(pair) {
            self.adj_fwd[u_id as usize].push(v_id);
            self.adj_fwd[u_id as usize].sort_by(|&a, &b| self.interner.resolve(a).cmp(&self.interner.resolve(b)));
            self.adj_bwd[v_id as usize].push(u_id);
            self.adj_bwd[v_id as usize].sort_by(|&a, &b| self.interner.resolve(a).cmp(&self.interner.resolve(b)));
            self.edge_stats.insert(pair, EdgeStats { weight: clamped_w, usage_count: 0 });
            true
        } else {
            if let Some(stats) = self.edge_stats.get_mut(&pair) { stats.weight = clamped_w; }
            false
        }
    }

    pub fn remove_edge(&mut self, u: &str, v: &str) -> bool {
        let (Some(u_id), Some(v_id)) = (self.interner.get_id(u), self.interner.get_id(v)) else { return false; };
        let pair = (u_id, v_id);
        if self.edges.remove(&pair) {
            if let Some(nbrs) = self.adj_fwd.get_mut(u_id as usize) { nbrs.retain(|&x| x != v_id); }
            if let Some(preds) = self.adj_bwd.get_mut(v_id as usize) { preds.retain(|&x| x != u_id); }
            self.edge_stats.remove(&pair);
            self.edge_world_overrides.remove(&pair);
            if !self.edges.iter().any(|&(x, y)| x == u_id || y == u_id) { self.resolver.deactivate(u_id, u, &self.interner); }
            if !self.edges.iter().any(|&(x, y)| x == v_id || y == v_id) { self.resolver.deactivate(v_id, v, &self.interner); }
            true
        } else { false }
    }

    pub fn set_edge_world_mask(&mut self, u: &str, v: &str, mask: u64) -> bool {
        let Some(pair) = self.interner.get_id(u).zip(self.interner.get_id(v)) else { return false; };
        self.edge_world_overrides.insert(pair, mask);
        true
    }
    pub fn set_edge_world_mask_u32(&mut self, u: u32, v: u32, mask: u64) {
        self.edge_world_overrides.insert((u, v), mask);
    }
    pub fn get_edge_world_mask_u32(&self, u: u32, v: u32) -> u64 {
        self.edge_world_overrides.get(&(u, v)).copied().unwrap_or(!0u64)
    }

    pub fn reinforce_edge(&mut self, u: &str, v: &str, delta: f64) -> Option<f64> {
        let pair = (self.interner.get_id(u)?, self.interner.get_id(v)?);
        let stats = self.edge_stats.get_mut(&pair)?;
        stats.weight = (stats.weight + delta).max(0.0);
        stats.usage_count += 1;
        Some(stats.weight)
    }

    pub fn set_edge_stats(&mut self, u: &str, v: &str, weight: f64, usage_count: u64) -> bool {
        let Some(pair) = self.interner.get_id(u).zip(self.interner.get_id(v)) else { return false; };
        if let Some(stats) = self.edge_stats.get_mut(&pair) { stats.weight = weight.max(0.0); stats.usage_count = usage_count; true } else { false }
    }

    pub fn edge_weight(&self, u: &str, v: &str) -> Option<f64> {
        self.interner.get_id(u).zip(self.interner.get_id(v)).and_then(|p| self.edge_stats.get(&p).map(|s| s.weight))
    }
    pub fn edge_stats(&self, u: &str, v: &str) -> Option<&EdgeStats> {
        self.interner.get_id(u).zip(self.interner.get_id(v)).and_then(|p| self.edge_stats.get(&p))
    }

    pub fn neighbors_with_weights(&self, node: &str) -> Vec<(String, f64)> {
        let (Some(u_id), Some(nbrs)) = (self.interner.get_id(node), self.interner.get_id(node).and_then(|id| self.adj_fwd.get(id as usize))) else { return Vec::new(); };
        nbrs.iter().filter_map(|&n| Some((self.interner.resolve(n)?.to_string(), self.edge_stats.get(&(u_id, n)).map(|s| s.weight).unwrap_or(1.0)))).collect()
    }

    pub fn register_entity_alias(&mut self, alias: impl Into<String>, canonical: impl Into<String>) {
        let (a_str, c_str) = (alias.into(), canonical.into());
        let (a, c) = (self.interner.intern(&a_str), self.interner.intern(&c_str));
        self.resolver.register_alias(a, &a_str, c, &mut self.interner);
    }
    pub fn unregister_entity_alias(&mut self, alias: &str, canonical: &str) {
        if let (Some(a), Some(c)) = (self.interner.get_id(alias), self.interner.get_id(canonical)) {
            self.resolver.unregister_alias(a, alias, c, &self.interner);
        }
    }

    pub fn resolve_node(&self, node: &str) -> NodeResolution<'_> {
        self.resolver.resolve(node, &self.interner)
    }

    pub fn has_edge(&mut self, u: &str, v: &str) -> bool {
        self.query_counter += 1;
        let Some(pair) = self.interner.get_id(u).zip(self.interner.get_id(v)) else { return false; };
        self.edges.contains(&pair)
    }

    pub fn contains_node(&self, node: &str) -> bool {
        matches!(self.resolve_node(node), NodeResolution::Exact(_) | NodeResolution::SingleAlias(_))
    }

    pub fn get_neighbors(&mut self, node: &str) -> &[String] {
        self.query_counter += 1; self.query_cache.clear();
        if let Some(u_id) = self.interner.get_id(node) {
            if let Some(nbrs) = self.adj_fwd.get(u_id as usize) {
                for &v_id in nbrs { if let Some(s) = self.interner.resolve(v_id) { self.query_cache.push(s.to_string()); } }
            }
        }
        self.query_cache.as_slice()
    }

    pub fn get_neighbors_u32(&self, id: u32) -> &[u32] { self.adj_fwd.get(id as usize).map(|v| v.as_slice()).unwrap_or(&[]) }
    pub fn get_predecessors_u32(&self, id: u32) -> &[u32] { self.adj_bwd.get(id as usize).map(|v| v.as_slice()).unwrap_or(&[]) }

    pub fn load_from_text(&mut self, text: &str) -> usize {
        let edges_part = text.find(". ¿Camino ").map_or(text, |idx| &text[..idx]);
        let cleaned = edges_part.replace("Grafo:", "").replace(',', " ");
        let mut count = 0;
        for token in cleaned.split_whitespace() {
            let clean_tok = token.trim_matches(|c| matches!(c, '.' | ';' | ','));
            if let Some((u, v)) = clean_tok.split_once("->") {
                if !u.is_empty() && !v.is_empty() && !v.ends_with('?') && self.insert_edge(u, v) { count += 1; }
            }
        }
        count
    }

    pub fn node_count(&self) -> usize { self.adj_fwd.iter().filter(|v| !v.is_empty()).count() }
    pub fn total_node_count(&self) -> usize { self.resolver.active_nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }
    pub fn total_queries(&self) -> usize { self.query_counter }
    pub fn increment_queries(&mut self, delta: usize) { self.query_counter += delta; }
}
