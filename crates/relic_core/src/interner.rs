//! Internador de identificadores compactos `String <-> u32`.
//! Almacena cadenas en un arena contigua de bytes con offsets u32.
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone)]
pub struct NodeInterner {
    lookup: HashMap<u64, Vec<u32>>,
    offsets: Vec<u32>,
    arena: Vec<u8>,
}

impl Default for NodeInterner {
    fn default() -> Self {
        Self {
            lookup: HashMap::default(),
            offsets: vec![0],
            arena: Vec::default(),
        }
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

impl NodeInterner {
    pub fn new() -> Self { Self::default() }

    pub fn intern(&mut self, s: &str) -> u32 {
        if self.offsets.is_empty() { self.offsets.push(0); }
        let h = hash_str(s);
        if let Some(bucket) = self.lookup.get(&h) {
            for &candidate_id in bucket {
                if self.resolve(candidate_id) == Some(s) {
                    return candidate_id;
                }
            }
        }
        let id = (self.offsets.len() - 1) as u32;
        self.arena.extend_from_slice(s.as_bytes());
        self.offsets.push(self.arena.len() as u32);
        self.lookup.entry(h).or_default().push(id);
        id
    }

    pub fn get_id(&self, s: &str) -> Option<u32> {
        let h = hash_str(s);
        let bucket = self.lookup.get(&h)?;
        bucket.iter().find(|&&cid| self.resolve(cid) == Some(s)).copied()
    }

    pub fn resolve(&self, id: u32) -> Option<&str> {
        let idx = id as usize;
        if idx + 1 < self.offsets.len() {
            let start = self.offsets[idx] as usize;
            let end = self.offsets[idx + 1] as usize;
            if start <= end && end <= self.arena.len() {
                return std::str::from_utf8(&self.arena[start..end]).ok();
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        if self.offsets.is_empty() { 0 } else { self.offsets.len() - 1 }
    }

    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn clear(&mut self) {
        self.lookup.clear();
        self.offsets.clear();
        self.offsets.push(0);
        self.arena.clear();
    }
}
