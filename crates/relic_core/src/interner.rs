//! Internador de identificadores compactos `String <-> u32`.
//! Almacena cada cadena una única vez como `Box<str>` sin duplicación en índices.
//! Menos de 180 líneas.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone, Default)]
pub struct NodeInterner {
    lookup: HashMap<u64, Vec<u32>>,
    to_str: Vec<Box<str>>,
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

impl NodeInterner {
    pub fn new() -> Self { Self::default() }

    pub fn intern(&mut self, s: &str) -> u32 {
        let h = hash_str(s);
        if let Some(bucket) = self.lookup.get(&h) {
            for &candidate_id in bucket {
                if let Some(candidate_str) = self.to_str.get(candidate_id as usize) {
                    if &**candidate_str == s {
                        return candidate_id;
                    }
                }
            }
        }
        let id = self.to_str.len() as u32;
        self.to_str.push(s.to_string().into_boxed_str());
        self.lookup.entry(h).or_default().push(id);
        id
    }

    pub fn get_id(&self, s: &str) -> Option<u32> {
        let h = hash_str(s);
        let bucket = self.lookup.get(&h)?;
        for &candidate_id in bucket {
            if let Some(candidate_str) = self.to_str.get(candidate_id as usize) {
                if &**candidate_str == s {
                    return Some(candidate_id);
                }
            }
        }
        None
    }

    pub fn resolve(&self, id: u32) -> Option<&str> {
        self.to_str.get(id as usize).map(|s| &**s)
    }

    pub fn len(&self) -> usize { self.to_str.len() }
    pub fn is_empty(&self) -> bool { self.to_str.is_empty() }
    pub fn clear(&mut self) {
        self.lookup.clear();
        self.to_str.clear();
    }
}
