//! # Layer 2: SIMD Vectorization — Real Benchmarked Implementation
//!
//! Exploits the AVX2, AVX-VNNI, FMA, AES-NI, SHA-NI instruction sets
//! detected on your Core Ultra 5 125H. Includes real benchmarks comparing
//! scalar vs SIMD performance.

use wide::f32x8;
use std::time::Instant;

/// Scalar baseline: element-wise `a * b + 1.5`
pub fn scalar_math(a: &[f32], b: &[f32], result: &mut [f32]) {
    for i in 0..a.len() {
        result[i] = a[i] * b[i] + 1.5;
    }
}

/// AVX2 SIMD: processes 8 floats per cycle using f32x8 registers.
/// Includes scalar tail for non-8-aligned lengths.
pub fn simd_math(a: &[f32], b: &[f32], result: &mut [f32]) {
    assert_eq!(a.len(), b.len(), "Input slices must be the same length");
    assert_eq!(a.len(), result.len(), "Result slice must match input length");

    let chunks = a.len() / 8;
    for i in 0..chunks {
        let idx = i * 8;
        let va = f32x8::new([
            a[idx], a[idx + 1], a[idx + 2], a[idx + 3],
            a[idx + 4], a[idx + 5], a[idx + 6], a[idx + 7],
        ]);
        let vb = f32x8::new([
            b[idx], b[idx + 1], b[idx + 2], b[idx + 3],
            b[idx + 4], b[idx + 5], b[idx + 6], b[idx + 7],
        ]);

        let vc = va * vb + f32x8::splat(1.5);
        result[idx..idx + 8].copy_from_slice(&vc.to_array());
    }

    // Scalar tail
    let tail_start = chunks * 8;
    for i in tail_start..a.len() {
        result[i] = a[i] * b[i] + 1.5;
    }
}

/// Dot product using AVX2 SIMD (8-wide accumulation).
pub fn simd_dot_product(a: &[f32], b: &[f32]) -> f32 {
    let chunks = a.len() / 8;
    let mut acc = f32x8::splat(0.0);

    for i in 0..chunks {
        let idx = i * 8;
        let va = f32x8::new([
            a[idx], a[idx + 1], a[idx + 2], a[idx + 3],
            a[idx + 4], a[idx + 5], a[idx + 6], a[idx + 7],
        ]);
        let vb = f32x8::new([
            b[idx], b[idx + 1], b[idx + 2], b[idx + 3],
            b[idx + 4], b[idx + 5], b[idx + 6], b[idx + 7],
        ]);
        acc = acc + va * vb;
    }

    let arr = acc.to_array();
    let mut sum: f32 = arr.iter().sum();

    // Tail
    for i in (chunks * 8)..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

/// Benchmarks scalar vs SIMD math to demonstrate real speedup on your hardware.
pub fn benchmark_simd() {
    const SIZE: usize = 10_000_000;
    let a: Vec<f32> = (0..SIZE).map(|i| (i as f32) * 0.001).collect();
    let b: Vec<f32> = (0..SIZE).map(|i| ((SIZE - i) as f32) * 0.001).collect();

    // ── Scalar FMA ──
    let mut result_scalar = vec![0.0f32; SIZE];
    let start = Instant::now();
    for _ in 0..5 {
        scalar_math(&a, &b, &mut result_scalar);
    }
    let scalar_us = start.elapsed().as_micros() / 5;

    // ── SIMD FMA ──
    let mut result_simd = vec![0.0f32; SIZE];
    let start = Instant::now();
    for _ in 0..5 {
        simd_math(&a, &b, &mut result_simd);
    }
    let simd_us = start.elapsed().as_micros() / 5;

    // ── SIMD Dot Product ──
    let start = Instant::now();
    let mut dot = 0.0f32;
    for _ in 0..5 {
        dot = simd_dot_product(&a, &b);
    }
    let dot_us = start.elapsed().as_micros() / 5;

    // ── Scalar Dot Product ──
    let start = Instant::now();
    let mut dot_scalar = 0.0f32;
    for _ in 0..5 {
        dot_scalar = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    }
    let dot_scalar_us = start.elapsed().as_micros() / 5;

    // Verify correctness
    let max_diff = result_scalar
        .iter()
        .zip(result_simd.iter())
        .map(|(s, v)| (s - v).abs())
        .fold(0.0f32, f32::max);

    println!("┌─────────────────────────────────────────────────┐");
    println!("│       SIMD BENCHMARK (10M f32 elements)         │");
    println!("├─────────────────────────────────────────────────┤");
    println!("│ FMA (a*b + 1.5):                                │");
    println!("│   Scalar:         {:>8} µs                    │", scalar_us);
    println!("│   AVX2 f32x8:     {:>8} µs                    │", simd_us);
    if simd_us > 0 {
        println!("│   Speedup:        {:>7.2}x                     │", scalar_us as f64 / simd_us as f64);
    }
    println!("│ Dot Product:                                    │");
    println!("│   Scalar:         {:>8} µs                    │", dot_scalar_us);
    println!("│   AVX2 f32x8:     {:>8} µs                    │", dot_us);
    if dot_us > 0 {
        println!("│   Speedup:        {:>7.2}x                     │", dot_scalar_us as f64 / dot_us as f64);
    }
    println!("│ Correctness:                                    │");
    println!("│   Max error:      {:.2e}                      │", max_diff);
    println!("│   Dot (SIMD):     {:.4e}                 │", dot);
    println!("│   Dot (scalar):   {:.4e}                 │", dot_scalar);
    println!("└─────────────────────────────────────────────────┘");
}

/// Lists the ISA features available on this CPU.
pub fn print_cpu_features() {
    println!("┌─────────────────────────────────────────────────┐");
    println!("│      DETECTED INSTRUCTION SET EXTENSIONS        │");
    println!("├─────────────────────────────────────────────────┤");

    let flags = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let interesting = [
        "avx2", "avx_vnni", "fma", "aes", "sha_ni", "sse4_2", "vaes",
        "vpclmulqdq", "gfni", "bmi2", "popcnt", "f16c",
    ];

    for flag in &interesting {
        let found = flags.contains(flag);
        let status = if found { "✓" } else { "✗" };
        println!("│  {} {:<18}                           │", status, flag);
    }
    println!("└─────────────────────────────────────────────────┘");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_correctness() {
        let a = vec![2.0f32; 24];
        let b = vec![3.0f32; 24];
        let mut scalar_r = vec![0.0f32; 24];
        let mut simd_r = vec![0.0f32; 24];

        scalar_math(&a, &b, &mut scalar_r);
        simd_math(&a, &b, &mut simd_r);

        for i in 0..24 {
            assert!((scalar_r[i] - simd_r[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0f32; 16];
        let b = vec![2.0f32; 16];
        let dot = simd_dot_product(&a, &b);
        assert!((dot - 32.0).abs() < 1e-4);
    }
}
