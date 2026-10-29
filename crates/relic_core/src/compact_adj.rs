//! # compact_adj
//!
//! Representacion compacta de adyacencia para evitar asignaciones de heap en nodos de grado <= 4.
//! Menos de 180 lineas.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactAdj {
    Inline(u8, [u32; 4]),
    Heap(Vec<u32>),
}

impl Default for CompactAdj {
    fn default() -> Self { CompactAdj::Inline(0, [0; 4]) }
}

impl CompactAdj {
    pub fn as_slice(&self) -> &[u32] {
        match self {
            CompactAdj::Inline(len, arr) => &arr[..*len as usize],
            CompactAdj::Heap(v) => v.as_slice(),
        }
    }

    pub fn push(&mut self, val: u32) {
        match self {
            CompactAdj::Inline(len, arr) => {
                if (*len as usize) < 4 {
                    arr[*len as usize] = val;
                    *len += 1;
                } else {
                    let mut vec = Vec::with_capacity(8);
                    vec.extend_from_slice(&arr[..4]);
                    vec.push(val);
                    *self = CompactAdj::Heap(vec);
                }
            }
            CompactAdj::Heap(v) => v.push(val),
        }
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&u32) -> bool,
    {
        match self {
            CompactAdj::Inline(len, arr) => {
                let mut new_len = 0u8;
                for i in 0..*len as usize {
                    let val = arr[i];
                    if f(&val) {
                        arr[new_len as usize] = val;
                        new_len += 1;
                    }
                }
                *len = new_len;
            }
            CompactAdj::Heap(v) => v.retain(f),
        }
    }

    pub fn sort_by<F>(&mut self, f: F)
    where
        F: FnMut(&u32, &u32) -> std::cmp::Ordering,
    {
        match self {
            CompactAdj::Inline(len, arr) => { arr[..*len as usize].sort_by(f); }
            CompactAdj::Heap(v) => v.sort_by(f),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            CompactAdj::Inline(len, _) => *len == 0,
            CompactAdj::Heap(v) => v.is_empty(),
        }
    }
}

