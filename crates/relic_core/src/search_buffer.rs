//! Scratch buffers reutilizables para BFS bidireccional determinista sin reasignaciones en el heap.
//! Menos de 180 líneas.

use std::cell::RefCell;

#[derive(Default)]
pub struct SearchBuffers {
    pub avoid_mask: Vec<bool>,
    pub parent_fwd: Vec<u32>,
    pub dist_fwd: Vec<u32>,
    pub parent_bwd: Vec<u32>,
    pub dist_bwd: Vec<u32>,
    pub touched_fwd: Vec<u32>,
    pub touched_bwd: Vec<u32>,
    pub touched_avoid: Vec<u32>,
}

impl SearchBuffers {
    pub fn prepare(&mut self, n: usize) {
        if self.avoid_mask.len() < n { self.avoid_mask.resize(n, false); }
        if self.parent_fwd.len() < n { self.parent_fwd.resize(n, u32::MAX); }
        if self.dist_fwd.len() < n { self.dist_fwd.resize(n, u32::MAX); }
        if self.parent_bwd.len() < n { self.parent_bwd.resize(n, u32::MAX); }
        if self.dist_bwd.len() < n { self.dist_bwd.resize(n, u32::MAX); }
        self.touched_fwd.clear();
        self.touched_bwd.clear();
        self.touched_avoid.clear();
    }

    pub fn reset(&mut self) {
        for &id in &self.touched_avoid {
            if let Some(slot) = self.avoid_mask.get_mut(id as usize) { *slot = false; }
        }
        for &id in &self.touched_fwd {
            if let Some(p) = self.parent_fwd.get_mut(id as usize) { *p = u32::MAX; }
            if let Some(d) = self.dist_fwd.get_mut(id as usize) { *d = u32::MAX; }
        }
        for &id in &self.touched_bwd {
            if let Some(p) = self.parent_bwd.get_mut(id as usize) { *p = u32::MAX; }
            if let Some(d) = self.dist_bwd.get_mut(id as usize) { *d = u32::MAX; }
        }
        self.touched_fwd.clear();
        self.touched_bwd.clear();
        self.touched_avoid.clear();
    }
}

thread_local! {
    pub static BUFFERS: RefCell<SearchBuffers> = RefCell::new(SearchBuffers::default());
}

