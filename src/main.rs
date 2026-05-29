//! # Intel Core Ultra 5 125H Optimization Suite — Live Runner
//!
//! Detects hardware, audits system config, benchmarks each optimization
//! layer, and generates an optimization script for your machine.

use intel_mac_parity::{
    compute_offload, memory, power_tuning, simd_vectorization, system_tuner, thread_pinning,
};
use std::fs;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Intel Core Ultra 5 125H — Live Performance Optimization   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // ── System Audit ─────────────────────────────────────────────────────
    println!("━━━ System Audit ━━━");
    system_tuner::print_audit();
    println!();
    
    // ── Power Tuning ─────────────────────────────────────────────────────
    power_tuning::print_power_limits();
    println!();

    // ── Layer 1: Core Topology & Thread Pinning ──────────────────────────
    println!("━━━ Layer 1: Core Topology & Thread Pinning ━━━");
    let cores = thread_pinning::detect_core_topology();
    thread_pinning::print_topology(&cores);
    println!();
    thread_pinning::benchmark_pinning();
    println!();

    // ── Layer 2: ISA Detection & SIMD Benchmarks ─────────────────────────
    println!("━━━ Layer 2: ISA Detection & SIMD Vectorization ━━━");
    simd_vectorization::print_cpu_features();
    simd_vectorization::benchmark_simd();
    println!();

    // ── Layer 3: Memory Architecture ─────────────────────────────────────
    println!("━━━ Layer 3: Memory Architecture ━━━");
    memory::print_memory_config();
    memory::benchmark_memory();
    println!();

    // ── Layer 4: Compute & AI Offloading ─────────────────────────────────
    println!("━━━ Layer 4: Compute & AI Offloading ━━━");
    compute_offload::list_available_devices();
    println!();

    // ── Generate Optimization Script ─────────────────────────────────────
    println!("━━━ Generating Optimization Script ━━━");
    let script = system_tuner::generate_optimization_script();
    let script_path = "optimize.sh";
    fs::write(script_path, &script).expect("Failed to write optimize.sh");
    println!("  ✓ Written to: {}", script_path);
    println!("  → Run with: sudo bash optimize.sh");
    println!("  → Then re-run this binary to see the improvement.");
    println!();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Done. Apply optimizations, then re-run to verify.         ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
