# Layer 3: Memory Architecture (Zero-Copy)

Apple Silicon's greatest advantage is **Unified Memory**. The CPU and GPU share the same physical memory space, eliminating the slow PCIe bus transfer (which typically bottlenecks traditional PCs).

Your Intel Core Ultra 5 125H **also has Unified Memory**. The CPU and the Intel Arc iGPU share your system's DDR5 RAM . However, you must explicitly program your software to use it properly.

## The Goal: Zero-Copy Compute
If you use standard `memcpy` to move data from CPU arrays to GPU buffers, you are wasting the architecture. Both processors must read/write to the exact same pointer.

## Implementation 1: wgpu (Rust Graphics/Compute)
When using the `wgpu` crate for GPU compute, you must allocate buffers as Host-Visible.

```rust
// wgpu buffer allocation for Unified Memory on Intel Arc
let unified_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("Unified Zero-Copy Buffer"),
    size: 1024 * 1024 * 64, // 64 MB
    // MAP_WRITE allows CPU to write directly; STORAGE allows GPU to compute
    usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::STORAGE,
    mapped_at_creation: true,
});

// CPU writes directly to the memory address. NO copy is performed.
{
    let mut buffer_view = unified_buffer.slice(..).get_mapped_range_mut();
    buffer_view.copy_from_slice(&my_cpu_data);
}
unified_buffer.unmap();

// Now dispatch the GPU shader. It reads the EXACT same physical RAM instantly.
```

## Implementation 2: Page-Locked Memory (Advanced)
Standard OS memory can be swapped to disk (Page File), causing massive latency spikes. To achieve Mac-level consistency, lock critical compute buffers into physical RAM using OS system calls.

```rust
use libc::{mlock, munlock, c_void};

pub struct PinnedBuffer {
    data: Vec<f32>,
}

impl PinnedBuffer {
    pub fn new(size: usize) -> Self {
        let mut data = vec![0.0f32; size];
        unsafe {
            // Ask OS to lock this memory into physical RAM
            let ptr = data.as_mut_ptr() as *mut c_void;
            let byte_size = size * std::mem::size_of::<f32>();
            if mlock(ptr, byte_size) != 0 {
                eprintln!("Warning: Failed to pin memory. Run with elevated privileges.");
            }
        }
        Self { data }
    }
}

impl Drop for PinnedBuffer {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.data.as_mut_ptr() as *mut c_void;
            let byte_size = self.data.len() * std::mem::size_of::<f32>();
            munlock(ptr, byte_size);
        }
    }
}
```
