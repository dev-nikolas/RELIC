//! Scratch buffers reutilizables para BFS bidireccional determinista sin reasignaciones en el heap.
//! Menos de 180 líneas.

use std::cell::RefCell;

#[derive(Default)]
pub struct SearchBuffers {
    pub current_gen: u32,
    pub visited_gen_fwd: Vec<u32>,
    pub visited_gen_bwd: Vec<u32>,
    pub avoid_gen: Vec<u32>,
    pub parent_fwd: Vec<u32>,
    pub dist_fwd: Vec<u32>,
    pub parent_bwd: Vec<u32>,
    pub dist_bwd: Vec<u32>,
    pub touched_fwd: Vec<u32>,
    pub touched_bwd: Vec<u32>,
}

impl SearchBuffers {
    pub fn prepare(&mut self, n: usize) {
        if self.visited_gen_fwd.len() < n { self.visited_gen_fwd.resize(n, 0); }
        if self.visited_gen_bwd.len() < n { self.visited_gen_bwd.resize(n, 0); }
        if self.avoid_gen.len() < n { self.avoid_gen.resize(n, 0); }
        if self.parent_fwd.len() < n { self.parent_fwd.resize(n, u32::MAX); }
        if self.dist_fwd.len() < n { self.dist_fwd.resize(n, u32::MAX); }
        if self.parent_bwd.len() < n { self.parent_bwd.resize(n, u32::MAX); }
        if self.dist_bwd.len() < n { self.dist_bwd.resize(n, u32::MAX); }

        self.current_gen = self.current_gen.wrapping_add(1);
        if self.current_gen == 0 {
            self.visited_gen_fwd.fill(0);
            self.visited_gen_bwd.fill(0);
            self.avoid_gen.fill(0);
            self.current_gen = 1;
        }
        self.touched_fwd.clear();
        self.touched_bwd.clear();
    }

    #[inline]
    pub fn reset(&mut self) {
        self.touched_fwd.clear();
        self.touched_bwd.clear();
    }
}

thread_local! {
    pub static BUFFERS: RefCell<SearchBuffers> = RefCell::new(SearchBuffers::default());
}
