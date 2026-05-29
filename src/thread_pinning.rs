//! # Layer 1: OS & Thread Optimization
//!
//! Explicit thread affinity (core pinning) for the Intel Core Ultra 5 125H
//! hybrid architecture:
//! - 4 Performance Cores (P-Cores): High clock speed, high power
//! - 8 Efficient Cores (E-Cores): Lower clock speed, highly efficient
//! - 2 Low-Power Efficient Cores (LP E-Cores): Ultra-low power, SoC tile
//!
//! Without manual pinning, the OS scheduler may assign critical compute threads
//! to E-Cores, causing up to 60% performance loss.

use core_affinity;
use rayon::ThreadPoolBuilder;

/// Number of P-Core logical threads on the Core Ultra 5 125H.
/// The first 8 logical thread IDs (0–7) map to the 4 P-Cores (2 threads each via HT).
const P_CORE_THREAD_COUNT: usize = 8;

/// Builds a Rayon thread pool where every worker is pinned to a P-Core.
///
/// All work submitted via `pool.install(|| { ... })` will execute exclusively
/// on Performance Cores, guaranteeing Mac-like latency and throughput for
/// math/render/AI workloads.
///
/// # Panics
/// Panics if core IDs cannot be retrieved or the thread pool cannot be built.
pub fn initialize_p_core_threadpool() -> rayon::ThreadPool {
    // 1. Get all available core IDs from the OS
    let core_ids = core_affinity::get_core_ids()
        .expect("Failed to retrieve core IDs from the OS");

    // 2. Select the P-Cores (the first 8 logical threads for the 125H)
    let p_core_ids: Vec<_> = core_ids.into_iter().take(P_CORE_THREAD_COUNT).collect();

    println!(
        "[Thread Pinning] Detected {} logical cores. Pinning {} workers to P-Cores.",
        core_affinity::get_core_ids().unwrap().len(),
        p_core_ids.len()
    );

    // 3. Build a custom Rayon threadpool bound strictly to P-Cores
    let pool = ThreadPoolBuilder::new()
        .num_threads(P_CORE_THREAD_COUNT)
        .start_handler(move |thread_idx| {
            let core_id = p_core_ids[thread_idx];
            let success = core_affinity::set_for_current(core_id);
            if success {
                println!(
                    "[Thread Pinning] Worker {} pinned to P-Core {}",
                    thread_idx, core_id.id
                );
            } else {
                eprintln!(
                    "[Thread Pinning] WARNING: Failed to pin worker {} to P-Core {}",
                    thread_idx, core_id.id
                );
            }
        })
        .build()
        .expect("Failed to build P-Core thread pool");

    pool
}

/// Runs a sample parallel workload on the P-Core pool to demonstrate pinning.
pub fn demo_p_core_workload() {
    let pool = initialize_p_core_threadpool();

    pool.install(|| {
        use rayon::prelude::*;

        let data: Vec<f64> = (0..1_000_000).map(|i| i as f64).collect();

        let sum: f64 = data.par_iter().map(|x| x.sqrt()).sum();

        println!(
            "[Thread Pinning] P-Core workload complete. Sum of sqrt(0..1M) = {:.4}",
            sum
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p_core_pool_creation() {
        let pool = initialize_p_core_threadpool();
        // The pool should have exactly P_CORE_THREAD_COUNT threads
        assert_eq!(pool.current_num_threads(), P_CORE_THREAD_COUNT);
    }
}
