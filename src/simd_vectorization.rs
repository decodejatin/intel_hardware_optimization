//! # Layer 2: Compiler Tuning — SIMD Vectorization
//!
//! With `target-cpu=native` set in `.cargo/config.toml`, the Rust compiler
//! automatically attempts to autovectorize loops to AVX2 (8 floats per cycle).
//!
//! This module provides explicit SIMD routines using the `wide` crate for
//! guaranteed vectorization of critical math paths.

use wide::f32x8;

/// Performs element-wise `a * b + 1.5` across two float slices using
/// AVX2 SIMD registers (8 elements per operation).
///
/// Only processes complete 8-element chunks. Remaining tail elements
/// are left unmodified in `result`.
///
/// # Arguments
/// * `a`      - Input slice A
/// * `b`      - Input slice B (must be same length as `a`)
/// * `result` - Output slice  (must be same length as `a`)
///
/// # Panics
/// Panics if slices have different lengths.
pub fn fast_vector_math(a: &[f32], b: &[f32], result: &mut [f32]) {
    assert_eq!(a.len(), b.len(), "Input slices must be the same length");
    assert_eq!(
        a.len(),
        result.len(),
        "Result slice must be the same length as input"
    );

    // Process 8 elements at a time using AVX2 registers natively
    let chunks = a.len() / 8;
    for i in 0..chunks {
        let idx = i * 8;
        let va = f32x8::new([
            a[idx],
            a[idx + 1],
            a[idx + 2],
            a[idx + 3],
            a[idx + 4],
            a[idx + 5],
            a[idx + 6],
            a[idx + 7],
        ]);
        let vb = f32x8::new([
            b[idx],
            b[idx + 1],
            b[idx + 2],
            b[idx + 3],
            b[idx + 4],
            b[idx + 5],
            b[idx + 6],
            b[idx + 7],
        ]);

        let vc = va * vb + f32x8::splat(1.5);

        let out = vc.to_array();
        result[idx..idx + 8].copy_from_slice(&out);
    }

    // Handle remaining tail elements (scalar fallback)
    let tail_start = chunks * 8;
    for i in tail_start..a.len() {
        result[i] = a[i] * b[i] + 1.5;
    }
}

/// Demonstration: runs SIMD math on a sample dataset and prints the result.
pub fn demo_simd_vectorization() {
    const SIZE: usize = 1024;
    let a: Vec<f32> = (0..SIZE).map(|i| i as f32 * 0.1).collect();
    let b: Vec<f32> = (0..SIZE).map(|i| (SIZE - i) as f32 * 0.05).collect();
    let mut result = vec![0.0f32; SIZE];

    fast_vector_math(&a, &b, &mut result);

    println!("[SIMD] Processed {} elements via AVX2 f32x8", SIZE);
    println!(
        "[SIMD] First 8 results: {:?}",
        &result[..8]
    );
    println!(
        "[SIMD] Last  8 results: {:?}",
        &result[SIZE - 8..]
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_vector_math_basic() {
        let a = vec![2.0f32; 16];
        let b = vec![3.0f32; 16];
        let mut result = vec![0.0f32; 16];

        fast_vector_math(&a, &b, &mut result);

        // 2.0 * 3.0 + 1.5 = 7.5
        for &v in &result {
            assert!((v - 7.5).abs() < 1e-6, "Expected 7.5, got {}", v);
        }
    }

    #[test]
    fn test_fast_vector_math_with_tail() {
        // 10 elements: 8 via SIMD + 2 via scalar tail
        let a = vec![1.0f32; 10];
        let b = vec![2.0f32; 10];
        let mut result = vec![0.0f32; 10];

        fast_vector_math(&a, &b, &mut result);

        // 1.0 * 2.0 + 1.5 = 3.5
        for &v in &result {
            assert!((v - 3.5).abs() < 1e-6, "Expected 3.5, got {}", v);
        }
    }

    #[test]
    #[should_panic(expected = "same length")]
    fn test_fast_vector_math_mismatched_lengths() {
        let a = vec![1.0f32; 8];
        let b = vec![1.0f32; 4];
        let mut result = vec![0.0f32; 8];
        fast_vector_math(&a, &b, &mut result);
    }
}
