use relic_cli::monitor::{get_current_private_heap_kb, get_current_rss_kb};
use relic_blackwall::{AnomalyGateMode, BlackwallGate};

fn main() {
    // 1. RSS absolutamente en frío (antes de inicializar cualquier estructura de RELIC)
    let cold_rss_kb = get_current_rss_kb();
    let cold_heap_kb = get_current_private_heap_kb();

    println!("============================================================");
    println!("  MEDICIÓN FORMAL DE ARRANQUE EN FRÍO (RSS Y HEAP)");
    println!("============================================================");
    println!("1. Proceso en frío (0 estructuras):");
    println!("   RSS:  {:>6.2} MB ({:>7} KB)", cold_rss_kb as f64 / 1024.0, cold_rss_kb);
    println!("   Heap: {:>6.2} MB ({:>7} KB)", cold_heap_kb as f64 / 1024.0, cold_heap_kb);

    // 2. BlackwallGate con Génesis y Telemetría Desactivada (Disabled)
    let mut gate_disabled = BlackwallGate::with_genesis();
    gate_disabled.telemetry_arena.mode = AnomalyGateMode::Disabled;
    let disabled_rss_kb = get_current_rss_kb();
    let disabled_heap_kb = get_current_private_heap_kb();

    println!("\n2. BlackwallGate::with_genesis() [Telemetría DISABLED]:");
    println!("   RSS:  {:>6.2} MB ({:>7} KB) | Delta: {:>+6} KB", disabled_rss_kb as f64 / 1024.0, disabled_rss_kb, disabled_rss_kb.saturating_sub(cold_rss_kb));
    println!("   Heap: {:>6.2} MB ({:>7} KB) | Delta: {:>+6} KB", disabled_heap_kb as f64 / 1024.0, disabled_heap_kb, disabled_heap_kb.saturating_sub(cold_heap_kb));

    // 3. BlackwallGate con Génesis y Telemetría en PassiveMonitor (0 canales alojados, lazy)
    let _gate_passive = BlackwallGate::with_genesis();
    let passive_rss_kb = get_current_rss_kb();
    let passive_heap_kb = get_current_private_heap_kb();

    println!("\n3. BlackwallGate::with_genesis() [Telemetría PASSIVE_MONITOR (0 canales)]: ");
    println!("   RSS:  {:>6.2} MB ({:>7} KB) | Delta vs Disabled: {:>+6} KB", passive_rss_kb as f64 / 1024.0, passive_rss_kb, passive_rss_kb.saturating_sub(disabled_rss_kb));
    println!("   Heap: {:>6.2} MB ({:>7} KB) | Delta vs Disabled: {:>+6} KB", passive_heap_kb as f64 / 1024.0, passive_heap_kb, passive_heap_kb.saturating_sub(disabled_heap_kb));

    // 4. Alojamiento de 1 Canal con ventanas llenas (16,000 ECDF, 6,400 Norm, 64x64 RLS)
    let mut gate_1ch = BlackwallGate::with_genesis();
    for step in 0..16_000 {
        let _ = gate_1ch.commit_transaction_with_telemetry(&[], &[(0, 25.0 + (step % 20) as f64)], step as u64);
    }
    let one_ch_rss_kb = get_current_rss_kb();
    let one_ch_heap_kb = get_current_private_heap_kb();

    println!("\n4. BlackwallGate con 1 Canal en Estado Estacionario (16.000 puntos):");
    println!("   RSS:  {:>6.2} MB ({:>7} KB) | Delta vs Passive: {:>+6} KB", one_ch_rss_kb as f64 / 1024.0, one_ch_rss_kb, one_ch_rss_kb.saturating_sub(passive_rss_kb));
    println!("   Heap: {:>6.2} MB ({:>7} KB) | Delta vs Passive: {:>+6} KB", one_ch_heap_kb as f64 / 1024.0, one_ch_heap_kb, one_ch_heap_kb.saturating_sub(passive_heap_kb));

    println!("============================================================");
}

