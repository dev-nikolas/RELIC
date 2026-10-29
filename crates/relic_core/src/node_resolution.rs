//! # node_resolution
//!
//! Resolucion canonica de entidades, alias y estadisticas de aristas.
//! Menos de 180 lineas.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::interner::NodeInterner;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeResolution<'a> {
    Exact(&'a str),
    SingleAlias(&'a str),
    Ambiguous(Vec<String>),
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeStats {
    pub weight: f64,
    pub usage_count: u64,
}

impl Default for EdgeStats {
    fn default() -> Self {
        Self { weight: 1.0, usage_count: 0 }
    }
}

pub fn canonical_node_key(s: &str) -> String {
    s.trim().to_lowercase()
        .replace('á', "a").replace('é', "e").replace('í', "i").replace('ó', "o").replace('ú', "u")
}

#[derive(Debug, Clone, Default)]
pub struct NodeResolver {
    pub active_nodes: HashSet<u32>,
    pub canonical_map: HashMap<u32, Vec<u32>>,
    pub explicit_aliases: HashMap<u32, Vec<u32>>,
}

impl NodeResolver {
    pub fn clear(&mut self) {
        self.active_nodes.clear();
        self.canonical_map.clear();
        self.explicit_aliases.clear();
    }

    pub fn activate(&mut self, id: u32, name: &str, interner: &mut NodeInterner) {
        self.active_nodes.insert(id);
        let key = canonical_node_key(name);
        let can_id = interner.intern(&key);
        let list = self.canonical_map.entry(can_id).or_default();
        if !list.contains(&id) {
            list.push(id);
        }
    }

    pub fn deactivate(&mut self, id: u32, name: &str, interner: &NodeInterner) {
        self.active_nodes.remove(&id);
        let key = canonical_node_key(name);
        if let Some(can_id) = interner.get_id(&key) {
            if let Some(list) = self.canonical_map.get_mut(&can_id) {
                list.retain(|&x| x != id);
            }
        }
    }

    pub fn register_alias(&mut self, a_id: u32, alias_str: &str, c_id: u32, interner: &mut NodeInterner) {
        let exact = self.explicit_aliases.entry(a_id).or_default();
        if !exact.contains(&c_id) { exact.push(c_id); }
        let key = canonical_node_key(alias_str);
        let can_id = interner.intern(&key);
        let list = self.canonical_map.entry(can_id).or_default();
        if !list.contains(&c_id) { list.push(c_id); }
    }

    pub fn unregister_alias(&mut self, a_id: u32, alias_str: &str, c_id: u32, interner: &NodeInterner) {
        if let Some(exact) = self.explicit_aliases.get_mut(&a_id) {
            exact.retain(|&x| x != c_id);
            if exact.is_empty() { self.explicit_aliases.remove(&a_id); }
        }
        let key = canonical_node_key(alias_str);
        if let Some(can_id) = interner.get_id(&key) {
            if let Some(list) = self.canonical_map.get_mut(&can_id) {
                list.retain(|&x| x != c_id);
                if list.is_empty() { self.canonical_map.remove(&can_id); }
            }
        }
    }

    pub fn resolve<'a>(&self, node: &str, interner: &'a NodeInterner) -> NodeResolution<'a> {
        if let Some(id) = interner.get_id(node) {
            if self.active_nodes.contains(&id) {
                if let Some(aliased) = self.explicit_aliases.get(&id) {
                    let mut candidates = vec![interner.resolve(id).unwrap().to_string()];
                    for &a_id in aliased {
                        if let Some(a_str) = interner.resolve(a_id) {
                            if !candidates.iter().any(|c| c == a_str) {
                                candidates.push(a_str.to_string());
                            }
                        }
                    }
                    if candidates.len() > 1 {
                        return NodeResolution::Ambiguous(candidates);
                    }
                }
                return NodeResolution::Exact(interner.resolve(id).unwrap());
            }
        }
        let key = canonical_node_key(node);
        let Some(can_id) = interner.get_id(&key) else {
            return NodeResolution::NotFound;
        };
        match self.canonical_map.get(&can_id) {
            None => NodeResolution::NotFound,
            Some(c) if c.len() == 1 => {
                let s = interner.resolve(c[0]).unwrap();
                NodeResolution::SingleAlias(s)
            }
            Some(c) => {
                let cand: Vec<String> = c.iter().filter_map(|&id| interner.resolve(id).map(|s| s.to_string())).collect();
                NodeResolution::Ambiguous(cand)
            }
        }
    }
}
