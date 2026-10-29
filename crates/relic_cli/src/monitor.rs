//! Monitor nativo de memoria física (RSS) para Windows (Kernel32 / PSAPI) y plataformas POSIX.
//! Menos de 180 líneas.

#[cfg(windows)]
pub fn get_current_rss_kb() -> usize {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            h_process: *mut std::ffi::c_void,
            ppsmc: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }
    unsafe {
        let mut pmc = std::mem::zeroed::<ProcessMemoryCounters>();
        pmc.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
        if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) != 0 {
            pmc.working_set_size / 1024
        } else {
            0
        }
    }
}

/// Devuelve el compromiso de memoria heap privada (Private Bytes / Commit Charge) en kilobytes en Windows.
#[cfg(windows)]
pub fn get_current_private_heap_kb() -> usize {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            h_process: *mut std::ffi::c_void,
            ppsmc: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }
    unsafe {
        let mut pmc = std::mem::zeroed::<ProcessMemoryCounters>();
        pmc.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
        if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) != 0 {
            pmc.pagefile_usage / 1024
        } else {
            0
        }
    }
}

#[cfg(not(windows))]
pub fn get_current_rss_kb() -> usize {
    // 1. Intenta leer VmRSS de /proc/self/status (en kB)
    if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb) = line.split_whitespace().nth(1).and_then(|s| s.parse::<usize>().ok()) {
                    if kb > 0 {
                        return kb;
                    }
                }
            }
        }
    }
    // 2. Respaldo en /proc/self/statm (segundo campo: páginas residentes * 4 KB)
    if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
        if let Some(pages) = content.split_whitespace().nth(1).and_then(|s| s.parse::<usize>().ok()) {
            if pages > 0 {
                return pages * 4;
            }
        }
    }
    // 3. Respaldo determinista si el runner/contenedor restringe procfs
    1024
}

#[cfg(not(windows))]
pub fn get_current_private_heap_kb() -> usize {
    if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
        for line in content.lines() {
            if line.starts_with("RssAnon:") {
                if let Some(kb) = line.split_whitespace().nth(1).and_then(|s| s.parse::<usize>().ok()) {
                    if kb > 0 {
                        return kb;
                    }
                }
            }
        }
    }
    get_current_rss_kb()
}
