//! # Layer 3: Memory Architecture — Real Page-Locked & Hugepage Implementation
//!
//! Applies actual memory optimizations to your system:
//! - Page-locked (mlock) buffers to prevent swap-out
//! - Hugepage-backed allocations to reduce TLB misses
//! - Benchmarks pinned vs unpinned memory access patterns

use libc::{c_void, madvise, mlock, mmap, munlock, munmap, MADV_HUGEPAGE};
use std::mem;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Page-Locked (Pinned) Memory
// ---------------------------------------------------------------------------

/// A heap-allocated `Vec<f32>` locked into physical RAM via `mlock(2)`.
/// Prevents the OS from swapping this memory to disk, eliminating
/// latency spikes during compute-intensive workloads.
pub struct PinnedBuffer {
    data: Vec<f32>,
    is_locked: bool,
}

impl PinnedBuffer {
    /// Allocates `size` floats and locks them into physical RAM.
    pub fn new(size: usize) -> Self {
        let mut data = vec![0.0f32; size];
        let byte_size = size * mem::size_of::<f32>();
        let is_locked;

        unsafe {
            let ptr = data.as_mut_ptr() as *mut c_void;
            if mlock(ptr, byte_size) == 0 {
                is_locked = true;
                println!(
                    "  ✓ Pinned {} MB into physical RAM",
                    byte_size / (1024 * 1024)
                );
            } else {
                is_locked = false;
                let errno = *libc::__errno_location();
                eprintln!(
                    "  ✗ mlock failed (errno {}). Current ulimit -l: check with `ulimit -l`",
                    errno
                );
            }
        }
        Self { data, is_locked }
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_locked(&self) -> bool {
        self.is_locked
    }
}

impl Drop for PinnedBuffer {
    fn drop(&mut self) {
        if self.is_locked {
            unsafe {
                let ptr = self.data.as_mut_ptr() as *mut c_void;
                let byte_size = self.data.len() * mem::size_of::<f32>();
                munlock(ptr, byte_size);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Hugepage-backed Memory
// ---------------------------------------------------------------------------

/// A buffer backed by transparent hugepages (2MB pages).
/// Reduces TLB misses dramatically for large memory regions,
/// which is critical for matrix/AI workloads.
pub struct HugePageBuffer {
    ptr: *mut u8,
    byte_size: usize,
    len: usize, // number of f32 elements
}

unsafe impl Send for HugePageBuffer {}
unsafe impl Sync for HugePageBuffer {}

impl HugePageBuffer {
    /// Allocates `num_floats` elements backed by transparent hugepages.
    /// Falls back to regular mmap if hugepage hint fails.
    pub fn new(num_floats: usize) -> Self {
        let byte_size = num_floats * mem::size_of::<f32>();
        // Round up to 2MB page boundary
        let aligned_size = (byte_size + (2 * 1024 * 1024 - 1)) & !(2 * 1024 * 1024 - 1);

        unsafe {
            let ptr = mmap(
                std::ptr::null_mut(),
                aligned_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );

            if ptr == libc::MAP_FAILED {
                panic!("mmap failed for hugepage buffer");
            }

            // Hint the kernel to use transparent hugepages
            let ret = madvise(ptr, aligned_size, MADV_HUGEPAGE);
            if ret == 0 {
                println!("  ✓ Hugepage hint accepted for {} MB", aligned_size / (1024 * 1024));
            } else {
                println!("  ⚠ Hugepage hint failed, using standard pages");
            }

            // Zero-initialize
            std::ptr::write_bytes(ptr as *mut u8, 0, aligned_size);

            // Lock into RAM
            if mlock(ptr, aligned_size) == 0 {
                println!("  ✓ Hugepage buffer locked into RAM");
            }

            Self {
                ptr: ptr as *mut u8,
                byte_size: aligned_size,
                len: num_floats,
            }
        }
    }

    pub fn as_slice(&self) -> &[f32] {
        unsafe { std::slice::from_raw_parts(self.ptr as *const f32, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut f32, self.len) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Drop for HugePageBuffer {
    fn drop(&mut self) {
        unsafe {
            munlock(self.ptr as *mut c_void, self.byte_size);
            munmap(self.ptr as *mut c_void, self.byte_size);
        }
    }
}

// ---------------------------------------------------------------------------
// Zero-Copy wgpu Buffer
// ---------------------------------------------------------------------------

/// Creates a unified zero-copy wgpu buffer for Intel Arc iGPU compute.
pub fn create_unified_buffer(
    device: &wgpu::Device,
    label: &str,
    size_bytes: u64,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size_bytes,
        usage: wgpu::BufferUsages::MAP_WRITE
            | wgpu::BufferUsages::MAP_READ
            | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: true,
    })
}

/// Writes CPU data into a unified buffer and unmaps it for GPU dispatch.
pub fn write_and_unmap(buffer: &wgpu::Buffer, data: &[u8]) {
    {
        let mut view = buffer.slice(..).get_mapped_range_mut();
        view[..data.len()].copy_from_slice(data);
    }
    buffer.unmap();
}

/// Initializes wgpu targeting the Intel Arc iGPU.
pub async fn init_wgpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await?;

    println!("  ✓ wgpu adapter: {}", adapter.get_info().name);

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Intel Arc Device"),
                required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await
        .ok()?;

    Some((device, queue))
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// Benchmarks pinned vs unpinned memory access to demonstrate the impact.
pub fn benchmark_memory() {
    const SIZE: usize = 4 * 1024 * 1024; // 4M floats = 16 MB
    const ITERS: usize = 20;

    // ── Unpinned (standard Vec) ──
    let mut unpinned = vec![0.0f32; SIZE];
    let start = Instant::now();
    for iter in 0..ITERS {
        for i in 0..SIZE {
            unpinned[i] = (i as f32 + iter as f32).sqrt();
        }
    }
    let unpinned_ms = start.elapsed().as_millis();

    // ── Pinned (mlock) ──
    let mut pinned = PinnedBuffer::new(SIZE);
    let start = Instant::now();
    for iter in 0..ITERS {
        for i in 0..SIZE {
            pinned.as_mut_slice()[i] = (i as f32 + iter as f32).sqrt();
        }
    }
    let pinned_ms = start.elapsed().as_millis();

    // ── Hugepage-backed ──
    let mut huge = HugePageBuffer::new(SIZE);
    let start = Instant::now();
    for iter in 0..ITERS {
        for i in 0..SIZE {
            huge.as_mut_slice()[i] = (i as f32 + iter as f32).sqrt();
        }
    }
    let huge_ms = start.elapsed().as_millis();

    println!();
    println!("┌─────────────────────────────────────────────────┐");
    println!("│    MEMORY BENCHMARK (16 MB, {} iterations)      │", ITERS);
    println!("├─────────────────────────────────────────────────┤");
    println!("│ Unpinned (Vec):     {:>6} ms                   │", unpinned_ms);
    println!("│ Pinned (mlock):     {:>6} ms  locked={}       │", pinned_ms, pinned.is_locked());
    println!("│ Hugepage (2MB THP): {:>6} ms                   │", huge_ms);
    println!("└─────────────────────────────────────────────────┘");
}

/// Prints the current memory configuration of the system.
pub fn print_memory_config() {
    println!("┌─────────────────────────────────────────────────┐");
    println!("│          SYSTEM MEMORY CONFIGURATION            │");
    println!("├─────────────────────────────────────────────────┤");

    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal")
                || line.starts_with("MemAvailable")
                || line.starts_with("SwapTotal")
                || line.starts_with("HugePages_Total")
                || line.starts_with("Hugepagesize")
                || line.starts_with("AnonHugePages")
            {
                println!("│  {:<46} │", line.trim());
            }
        }
    }

    if let Ok(swap) = std::fs::read_to_string("/proc/sys/vm/swappiness") {
        println!("│  Swappiness: {:<34} │", swap.trim());
    }

    println!("└─────────────────────────────────────────────────┘");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinned_buffer() {
        let mut buf = PinnedBuffer::new(256);
        assert_eq!(buf.len(), 256);
        buf.as_mut_slice()[0] = 42.0;
        assert!((buf.as_slice()[0] - 42.0).abs() < 1e-6);
    }

    #[test]
    fn test_hugepage_buffer() {
        let mut buf = HugePageBuffer::new(1024);
        assert_eq!(buf.len(), 1024);
        buf.as_mut_slice()[0] = 99.0;
        assert!((buf.as_slice()[0] - 99.0).abs() < 1e-6);
    }
}
