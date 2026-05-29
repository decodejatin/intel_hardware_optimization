//! # Intel Core Ultra 5 125H Optimization Suite
//!
//! Real implementation of four optimization layers + system tuner:
//!
//! 1. **Thread Pinning** — Auto-detected P/E/LP core topology + affinity
//! 2. **SIMD Vectorization** — AVX2 f32x8 with real benchmarks
//! 3. **Memory Architecture** — mlock + hugepages + zero-copy wgpu
//! 4. **Compute Offload** — GPU/NPU inference via OpenVINO
//! 5. **System Tuner** — Kernel parameter audit and optimization

pub mod thread_pinning;
pub mod simd_vectorization;
pub mod memory;
pub mod compute_offload;
pub mod system_tuner;
