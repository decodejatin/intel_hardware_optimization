//! # Layer 1: OS & Thread Optimization — Real Implementation
//!
//! Detects the actual hybrid core topology of the Intel Core Ultra 5 125H
//! at runtime and pins threads to correct P-Core / E-Core / LP E-Core sets.
//!
//! ## Your Machine's Topology (auto-detected):
//! - P-Cores  (4.5 GHz): CPUs 0-7   (4 cores × 2 HT = 8 threads)
//! - E-Cores  (3.6 GHz): CPUs 8-15  (8 cores × 1 thread)
//! - LP E-Cores (2.5 GHz): CPUs 16-17 (2 cores × 1 thread, SoC tile)

use core_affinity::{self, CoreId};
use rayon::ThreadPoolBuilder;
use std::fs;
use std::time::Instant;

/// Core type classification based on max frequency.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreType {
    /// Performance Core (4.5 GHz on 125H)
    PCores,
    /// Efficient Core (3.6 GHz on 125H)
    ECores,
    /// Low-Power Efficient Core (2.5 GHz on 125H, SoC tile)
    LPCores,
}

/// Detected core info from sysfs.
#[derive(Debug, Clone)]
pub struct DetectedCore {
    pub cpu_id: usize,
    pub max_freq_khz: u64,
    pub core_type: CoreType,
}

/// Detects the hybrid core topology from sysfs at runtime.
/// Returns cores sorted by CPU ID.
pub fn detect_core_topology() -> Vec<DetectedCore> {
    let mut cores = Vec::new();

    for entry in fs::read_dir("/sys/devices/system/cpu/").unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();

        // Match cpu0, cpu1, ... cpuN
        if !name.starts_with("cpu") {
            continue;
        }
        let id_str = &name[3..];
        let cpu_id: usize = match id_str.parse() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let freq_path = format!(
            "/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq",
            cpu_id
        );
        let max_freq_khz: u64 = match fs::read_to_string(&freq_path) {
            Ok(s) => s.trim().parse().unwrap_or(0),
            Err(_) => continue,
        };

        let core_type = if max_freq_khz >= 4_000_000 {
            CoreType::PCores
        } else if max_freq_khz >= 3_000_000 {
            CoreType::ECores
        } else {
            CoreType::LPCores
        };

        cores.push(DetectedCore {
            cpu_id,
            max_freq_khz,
            core_type,
        });
    }

    cores.sort_by_key(|c| c.cpu_id);
    cores
}

/// Returns the CPU IDs for a given core type.
pub fn get_cpu_ids_for(cores: &[DetectedCore], core_type: CoreType) -> Vec<usize> {
    cores
        .iter()
        .filter(|c| c.core_type == core_type)
        .map(|c| c.cpu_id)
        .collect()
}

/// Prints the full detected topology.
pub fn print_topology(cores: &[DetectedCore]) {
    let p_cores = get_cpu_ids_for(cores, CoreType::PCores);
    let e_cores = get_cpu_ids_for(cores, CoreType::ECores);
    let lp_cores = get_cpu_ids_for(cores, CoreType::LPCores);

    println!("┌─────────────────────────────────────────────────┐");
    println!("│         DETECTED CORE TOPOLOGY                  │");
    println!("├─────────────────────────────────────────────────┤");
    println!(
        "│ P-Cores  (High-Perf): {:?}",
        p_cores
    );
    println!(
        "│ E-Cores  (Efficient): {:?}",
        e_cores
    );
    println!(
        "│ LP-Cores (Low-Power): {:?}",
        lp_cores
    );
    println!(
        "│ Total threads: {}",
        cores.len()
    );
    println!("└─────────────────────────────────────────────────┘");
}

/// Builds a Rayon thread pool pinned to P-Cores only.
pub fn build_p_core_pool() -> rayon::ThreadPool {
    let cores = detect_core_topology();
    let p_core_cpus = get_cpu_ids_for(&cores, CoreType::PCores);
    let all_core_ids = core_affinity::get_core_ids().expect("Failed to get core IDs");
    let num_p = p_core_cpus.len();

    // Map P-Core CPU IDs to core_affinity CoreIds
    let p_core_ids: Vec<CoreId> = p_core_cpus
        .iter()
        .filter_map(|&cpu| all_core_ids.iter().find(|c| c.id == cpu).copied())
        .collect();

    println!("[Thread Pool] Building P-Core pool with {} threads", num_p);

    ThreadPoolBuilder::new()
        .num_threads(num_p)
        .start_handler(move |thread_idx| {
            if thread_idx < p_core_ids.len() {
                let core_id = p_core_ids[thread_idx];
                if core_affinity::set_for_current(core_id) {
                    println!(
                        "  ✓ Worker {} → P-Core CPU {}",
                        thread_idx, core_id.id
                    );
                } else {
                    eprintln!(
                        "  ✗ Worker {} failed to pin to CPU {}",
                        thread_idx, core_id.id
                    );
                }
            }
        })
        .build()
        .expect("Failed to build P-Core thread pool")
}

/// Builds a Rayon thread pool pinned to E-Cores only (for IO/background work).
pub fn build_e_core_pool() -> rayon::ThreadPool {
    let cores = detect_core_topology();
    let e_core_cpus = get_cpu_ids_for(&cores, CoreType::ECores);
    let all_core_ids = core_affinity::get_core_ids().expect("Failed to get core IDs");
    let num_e = e_core_cpus.len();

    let e_core_ids: Vec<CoreId> = e_core_cpus
        .iter()
        .filter_map(|&cpu| all_core_ids.iter().find(|c| c.id == cpu).copied())
        .collect();

    println!("[Thread Pool] Building E-Core pool with {} threads", num_e);

    ThreadPoolBuilder::new()
        .num_threads(num_e)
        .start_handler(move |thread_idx| {
            if thread_idx < e_core_ids.len() {
                let core_id = e_core_ids[thread_idx];
                if core_affinity::set_for_current(core_id) {
                    println!(
                        "  ✓ Worker {} → E-Core CPU {}",
                        thread_idx, core_id.id
                    );
                }
            }
        })
        .build()
        .expect("Failed to build E-Core thread pool")
}

/// Benchmarks P-Core vs E-Core vs unpinned performance to prove the impact.
pub fn benchmark_pinning() {
    use rayon::prelude::*;

    let data: Vec<f64> = (0..10_000_000).map(|i| i as f64).collect();

    // ── Unpinned (default scheduler) ──
    let start = Instant::now();
    let _sum: f64 = data.par_iter().map(|x| x.sqrt().sin().cos()).sum();
    let unpinned_ms = start.elapsed().as_millis();

    // ── P-Core pinned ──
    let p_pool = build_p_core_pool();
    let start = Instant::now();
    p_pool.install(|| {
        let _sum: f64 = data.par_iter().map(|x| x.sqrt().sin().cos()).sum();
    });
    let p_core_ms = start.elapsed().as_millis();

    // ── E-Core pinned ──
    let e_pool = build_e_core_pool();
    let start = Instant::now();
    e_pool.install(|| {
        let _sum: f64 = data.par_iter().map(|x| x.sqrt().sin().cos()).sum();
    });
    let e_core_ms = start.elapsed().as_millis();

    println!();
    println!("┌─────────────────────────────────────────────────┐");
    println!("│     THREAD PINNING BENCHMARK (10M elements)     │");
    println!("├─────────────────────────────────────────────────┤");
    println!("│ Unpinned (OS scheduler):  {:>6} ms              │", unpinned_ms);
    println!("│ P-Core pinned:            {:>6} ms              │", p_core_ms);
    println!("│ E-Core pinned:            {:>6} ms              │", e_core_ms);
    if p_core_ms > 0 && e_core_ms > 0 {
        let speedup = e_core_ms as f64 / p_core_ms as f64;
        println!("│ P-Core speedup vs E-Core: {:>5.1}x               │", speedup);
    }
    println!("└─────────────────────────────────────────────────┘");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_detection() {
        let cores = detect_core_topology();
        assert!(!cores.is_empty(), "Should detect at least one core");
    }

    #[test]
    fn test_p_core_pool_creation() {
        let pool = build_p_core_pool();
        let p_count = get_cpu_ids_for(&detect_core_topology(), CoreType::PCores).len();
        assert_eq!(pool.current_num_threads(), p_count);
    }
}
