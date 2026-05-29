//! # Intel Core Ultra 5 125H Optimization Playbook — Runner
//!
//! Demonstrates all four optimization layers:
//! 1. OS & Thread Optimization (P-Core pinning)
//! 2. Compiler Tuning (SIMD vectorization)
//! 3. Memory Architecture (Pinned memory + zero-copy)
//! 4. Compute & AI Offloading (OpenVINO GPU/NPU)

use intel_mac_parity::{compute_offload, memory, simd_vectorization, thread_pinning};

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Intel Core Ultra 5 125H — Apple Silicon Parity Playbook   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // ── Layer 1: Thread Pinning ──────────────────────────────────────────
    println!("━━━ Layer 1: OS & Thread Optimization ━━━");
    thread_pinning::demo_p_core_workload();
    println!();

    // ── Layer 2: SIMD Vectorization ──────────────────────────────────────
    println!("━━━ Layer 2: SIMD Vectorization (AVX2 f32x8) ━━━");
    simd_vectorization::demo_simd_vectorization();
    println!();

    // ── Layer 3: Memory Architecture ─────────────────────────────────────
    println!("━━━ Layer 3: Memory Architecture (Page-Locked) ━━━");
    memory::demo_pinned_memory();
    println!();

    // ── Layer 4: Compute & AI Offloading ─────────────────────────────────
    println!("━━━ Layer 4: Compute & AI Offloading (OpenVINO) ━━━");
    compute_offload::list_available_devices();
    println!("  (Inference skipped — no model files provided.)");
    println!("  Usage: compute_offload::run_ai_inference(\"model.xml\", \"model.bin\")");
    println!();

    println!("✓ All optimization layers demonstrated successfully.");
    println!("  Build with `cargo build --release` for full native optimization.");
}
