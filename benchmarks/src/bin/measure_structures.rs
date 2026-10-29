//! Medición y atribución del 100% de memoria en EpisodicMemory con GlobalAlloc.
//! Menos de 180 líneas.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use relic_core::EpisodicMemory;

struct CountingAlloc;
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() { ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed); }
        ret
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

fn evaluate_topology(name: &str, num_facts: usize, num_nodes_target: usize) {
    let baseline = ALLOCATED.load(Ordering::Relaxed);
    let mut mem = EpisodicMemory::new();

    for i in 0..num_facts {
        let u = if num_nodes_target == num_facts {
            format!("Node_A_{i}")
        } else {
            format!("Node_Src_{i}")
        };
        let v = if num_nodes_target == num_facts {
            format!("Node_A_{}", (i + 1) % num_facts)
        } else {
            format!("Node_Dst_{i}")
        };
        mem.insert_edge(u, v);
    }

    let total_alive = ALLOCATED.load(Ordering::Relaxed).saturating_sub(baseline);
    let v_count = mem.total_node_count();
    let e_count = mem.edge_count();

    // 1. Cadenas puras
    let mut str_bytes = 0usize;
    for id in 0..v_count as u32 {
        if let Some(s) = mem.interner.resolve(id) { str_bytes += s.len(); }
    }
    // 2. Desglose analítico
    let box_ptrs = v_count * 16; // Box<str> en to_str
    let interner_buckets = v_count.next_power_of_two() * 32; // HashMap<u64, Vec<u32>>
    let interner_subvecs = v_count * 24; // Vec<u32> structs
    let adj_outer = 2 * (v_count * 24); // adj_fwd y adj_bwd outer vectors
    let adj_inner_payload = 2 * (e_count * 4); // u32 de aristas
    let adj_inner_subvecs = 2 * (v_count * 24); // 2*V subvectores de adyacencia
    let edges_hash_buckets = e_count.next_power_of_two() * 16; // HashSet<(u32, u32)>
    let edge_stats_buckets = e_count.next_power_of_two() * 40; // HashMap<(u32, u32), EdgeStats>
    let resolver_active = v_count.next_power_of_two() * 8; // HashSet<u32>
    let resolver_canonical = v_count.next_power_of_two() * 32 + (v_count * 24); // HashMap + subvecs

    let accounted_pure = str_bytes + box_ptrs + interner_buckets + interner_subvecs
        + adj_outer + adj_inner_payload + adj_inner_subvecs + edges_hash_buckets
        + edge_stats_buckets + resolver_active + resolver_canonical;
    let slop_unused_capacity = total_alive.saturating_sub(accounted_pure);

    println!("\n============================================================");
    println!("  TOPOLOGÍA: {name} (V={v_count}, E={e_count})");
    println!("============================================================");
    println!("  Memoria Viva Motor (EpisodicMemory) : {:.2} MB ({} bytes)", total_alive as f64 / (1024.0 * 1024.0), total_alive);
    println!("  Bytes por hecho (motor puro)        : {:.1} bytes/hecho", total_alive as f64 / e_count as f64);
    println!("------------------------------------------------------------");
    println!("  Atribución del 100% de Bytes:");
    println!("    1. Cadenas Box<str> en Arena       : {:>6.2} MB ({:>4.1}%)", (str_bytes + box_ptrs) as f64 / (1024.0 * 1024.0), ((str_bytes + box_ptrs) as f64 / total_alive as f64) * 100.0);
    println!("    2. Tablas Hash (Buckets 2^N)       : {:>6.2} MB ({:>4.1}%)", (interner_buckets + edges_hash_buckets + edge_stats_buckets + resolver_active) as f64 / (1024.0 * 1024.0), ((interner_buckets + edges_hash_buckets + edge_stats_buckets + resolver_active) as f64 / total_alive as f64) * 100.0);
    println!("    3. Subvectores Vec<u32> (4x|V|)    : {:>6.2} MB ({:>4.1}%)", (interner_subvecs + adj_inner_subvecs + resolver_canonical) as f64 / (1024.0 * 1024.0), ((interner_subvecs + adj_inner_subvecs + resolver_canonical) as f64 / total_alive as f64) * 100.0);
    println!("    4. Carga útil de Aristas (u32)     : {:>6.2} MB ({:>4.1}%)", adj_inner_payload as f64 / (1024.0 * 1024.0), (adj_inner_payload as f64 / total_alive as f64) * 100.0);
    println!("    5. Capacidad ociosa Vec/Hash (Slop): {:>6.2} MB ({:>4.1}%)", slop_unused_capacity as f64 / (1024.0 * 1024.0), (slop_unused_capacity as f64 / total_alive as f64) * 100.0);
    println!("    TOTAL ATRIBUIDO                   : {:>6.2} MB (100.0%)", total_alive as f64 / (1024.0 * 1024.0));
    println!("------------------------------------------------------------");
    let csr_est_mb = (2.0 * ((v_count + 1) * 4 + e_count * 4) as f64) / (1024.0 * 1024.0);
    let adj_actual_mb = (adj_outer + adj_inner_subvecs + adj_inner_payload) as f64 / (1024.0 * 1024.0);
    println!("  Estimación de Reducción con CSR:");
    println!("    - Adyacencia actual (Vec<Vec<u32>>): ~{:.2} MB", adj_actual_mb);
    println!("    - Adyacencia con CSR (2 buffers planos): ~{:.2} MB", csr_est_mb);
    println!("    - Ahorro proyectado de CSR: -{:.1}% en adyacencia", (1.0 - csr_est_mb / adj_actual_mb) * 100.0);
}

fn evaluate_search_buffers() {
    let baseline = ALLOCATED.load(Ordering::Relaxed);
    println!("\n============================================================");
    println!("  MEDICIÓN SearchBuffers POR HILO");
    println!("============================================================");
    let mut buf = relic_core::search_buffer::SearchBuffers::default();
    println!("  Capacidad inicial: {} bytes", ALLOCATED.load(Ordering::Relaxed).saturating_sub(baseline));

    buf.prepare(50_000);
    let cap_50k = ALLOCATED.load(Ordering::Relaxed).saturating_sub(baseline);
    println!("  Tras prepare(50.000 nodos)  : {:>8} bytes ({:.2} KB)", cap_50k, cap_50k as f64 / 1024.0);

    buf.prepare(100_000);
    let cap_100k = ALLOCATED.load(Ordering::Relaxed).saturating_sub(baseline);
    println!("  Tras prepare(100.000 nodos) : {:>8} bytes ({:.2} KB)", cap_100k, cap_100k as f64 / 1024.0);

    buf.prepare(50_000);
    let cap_50k_re = ALLOCATED.load(Ordering::Relaxed).saturating_sub(baseline);
    println!("  Tras prepare(50.000) reit.  : {:>8} bytes ({:.2} KB, retiene max)", cap_50k_re, cap_50k_re as f64 / 1024.0);

    buf.reset();
    let cap_after_reset = ALLOCATED.load(Ordering::Relaxed).saturating_sub(baseline);
    println!("  Tras reset()                : {:>8} bytes (retiene buffers para reuso)", cap_after_reset);
}

fn main() {
    evaluate_topology("50k hechos sobre 50k nodos (denso)", 50_000, 50_000);
    evaluate_topology("50k hechos sobre 100k nodos (bipartito/disperso)", 50_000, 100_000);
    evaluate_search_buffers();
}
