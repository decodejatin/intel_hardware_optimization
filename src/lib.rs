//! # Intel Core Ultra 5 125H Optimization Library
//!
//! This crate implements four optimization layers to achieve Apple Silicon
//! performance parity on the Intel Core Ultra 5 125H (Meteor Lake):
//!
//! 1. **Thread Pinning** — P-Core affinity via `core_affinity` + `rayon`
//! 2. **SIMD Vectorization** — Explicit AVX2 operations via `wide`
//! 3. **Memory Architecture** — Zero-copy wgpu buffers + page-locked memory
//! 4. **Compute Offload** — GPU/NPU inference via OpenVINO

pub mod thread_pinning;
pub mod simd_vectorization;
pub mod memory;
pub mod compute_offload;
