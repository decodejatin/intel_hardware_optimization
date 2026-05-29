//! # Layer 3: Memory Architecture — Zero-Copy & Page-Locked Memory
//!
//! Apple Silicon's greatest advantage is Unified Memory. The Intel Core Ultra
//! 5 125H also has Unified Memory (CPU + Arc iGPU share DDR5), but software
//! must explicitly leverage it.
//!
//! This module provides:
//! 1. **Zero-Copy wgpu buffer allocation** — Host-visible buffers for GPU compute
//! 2. **Page-Locked (pinned) memory** — Prevents OS page-out for critical buffers

use libc::{c_void, mlock, munlock};
use std::mem;

// ---------------------------------------------------------------------------
// Page-Locked (Pinned) Memory
// ---------------------------------------------------------------------------

/// A heap-allocated `Vec<f32>` whose backing memory is locked into physical RAM
/// via `mlock(2)`, preventing the OS from swapping it to disk.
///
/// This eliminates latency spikes caused by page faults during compute-intensive
/// workloads and is essential for achieving Apple-Silicon-level memory consistency.
///
/// # Platform Note
/// Requires elevated privileges (or a sufficient `RLIMIT_MEMLOCK`) on Linux.
pub struct PinnedBuffer {
    data: Vec<f32>,
}

impl PinnedBuffer {
    /// Allocates `size` floats and locks them into physical RAM.
    ///
    /// Prints a warning to stderr if `mlock` fails (typically due to insufficient
    /// privileges). The buffer is still usable—just not pinned.
    pub fn new(size: usize) -> Self {
        let mut data = vec![0.0f32; size];
        unsafe {
            // Ask the OS to lock this memory into physical RAM
            let ptr = data.as_mut_ptr() as *mut c_void;
            let byte_size = size * mem::size_of::<f32>();
            if mlock(ptr, byte_size) != 0 {
                eprintln!(
                    "[Memory] Warning: Failed to pin {} bytes. \
                     Run with elevated privileges or raise RLIMIT_MEMLOCK.",
                    byte_size
                );
            } else {
                println!(
                    "[Memory] Successfully pinned {} bytes ({} floats) into physical RAM.",
                    byte_size, size
                );
            }
        }
        Self { data }
    }

    /// Returns an immutable slice of the pinned data.
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Returns a mutable slice of the pinned data.
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Number of f32 elements in the buffer.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Drop for PinnedBuffer {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.data.as_mut_ptr() as *mut c_void;
            let byte_size = self.data.len() * mem::size_of::<f32>();
            munlock(ptr, byte_size);
        }
    }
}

// ---------------------------------------------------------------------------
// Zero-Copy wgpu Buffer Allocation
// ---------------------------------------------------------------------------

/// Creates a unified, zero-copy wgpu buffer on the Intel Arc iGPU.
///
/// The buffer is:
/// - `MAP_WRITE` — CPU can write directly to GPU-visible memory
/// - `MAP_READ`  — CPU can read results back without a copy
/// - `STORAGE`   — GPU can use it in compute shaders
/// - `mapped_at_creation` — Immediately available for CPU writes
///
/// This exploits the Intel Core Ultra 5 125H's Unified Memory architecture:
/// CPU and Arc iGPU share the same physical DDR5, so no PCIe bus transfer occurs.
///
/// # Arguments
/// * `device`    - The wgpu device
/// * `label`     - A debug label for the buffer
/// * `size_bytes`- Size of the buffer in bytes
pub fn create_unified_buffer(
    device: &wgpu::Device,
    label: &str,
    size_bytes: u64,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size_bytes,
        // MAP_WRITE: CPU writes directly; MAP_READ: CPU reads back; STORAGE: GPU computes
        usage: wgpu::BufferUsages::MAP_WRITE
            | wgpu::BufferUsages::MAP_READ
            | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: true,
    })
}

/// Writes CPU data into a unified buffer and unmaps it, readying it for GPU dispatch.
///
/// # Arguments
/// * `buffer` - A buffer created with `create_unified_buffer` (still mapped)
/// * `data`   - The raw byte data to write
pub fn write_and_unmap(buffer: &wgpu::Buffer, data: &[u8]) {
    {
        let mut buffer_view = buffer.slice(..).get_mapped_range_mut();
        buffer_view[..data.len()].copy_from_slice(data);
    }
    buffer.unmap();
}

/// Initializes the wgpu adapter and device targeting the Intel Arc iGPU.
///
/// Returns `(device, queue)` on success.
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

    println!("[Memory] wgpu adapter: {}", adapter.get_info().name);

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

/// Demonstration: allocates a pinned buffer and prints stats.
pub fn demo_pinned_memory() {
    const SIZE: usize = 1024 * 1024; // 1M floats = 4 MB
    let mut buf = PinnedBuffer::new(SIZE);

    // Fill with data
    for (i, v) in buf.as_mut_slice().iter_mut().enumerate() {
        *v = i as f32;
    }

    let sum: f32 = buf.as_slice().iter().sum();
    println!(
        "[Memory] Pinned buffer demo: {} floats, sum = {:.1}",
        buf.len(),
        sum
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinned_buffer_creation() {
        let buf = PinnedBuffer::new(256);
        assert_eq!(buf.len(), 256);
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_pinned_buffer_read_write() {
        let mut buf = PinnedBuffer::new(16);
        for (i, v) in buf.as_mut_slice().iter_mut().enumerate() {
            *v = i as f32;
        }
        assert!((buf.as_slice()[0] - 0.0).abs() < 1e-6);
        assert!((buf.as_slice()[15] - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_pinned_buffer_empty() {
        let buf = PinnedBuffer::new(0);
        assert!(buf.is_empty());
    }
}
