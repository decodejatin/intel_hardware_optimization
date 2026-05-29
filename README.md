# Intel Core Ultra 5 125H Optimization Playbook
## Mission: Achieving Apple Silicon Parity

This project serves as the definitive guide and codebase blueprint for optimizing the **Intel Core Ultra 5 125H (Meteor Lake)** processor to achieve performance parity with Apple Silicon Macs.

Apple Silicon achieves its speed through extreme hardware-software co-optimization (Unified Memory, strict thread scheduling, and native compilation). By default, Windows and Linux treat the 125H as a generic x86 chip, wasting its potential.

This playbook provides the architecture to fix that.

### Hardware Overview (Core Ultra 5 125H)
*   **CPU:** 14 Cores / 18 Threads (4 P-Cores, 8 E-Cores, 2 LP E-Cores)
*   **GPU:** Intel Arc Graphics (7 Xe-Cores with XMX Matrix Engines)
*   **NPU:** Intel AI Boost (Dedicated low-power inference)
*   **Memory:** Shared System RAM (LPDDR5X up to 120 GB/s)

### Optimization Layers
Please proceed through the following documentation to implement the optimizations:

1.  [OS & Thread Optimization](./01-os-thread-optimization.md) - Escaping the hybrid core scheduling penalty.
2.  [Compiler Tuning](./02-compiler-tuning.md) - Forcing Rust/C++ to target Meteor Lake silicon natively.
3.  [Memory Architecture](./03-memory-architecture.md) - Implementing Zero-Copy and Page-Locked memory.
4.  [Compute & AI Offloading](./04-gpu-npu-compute.md) - Leveraging the Arc GPU, XMX, and NPU via OpenVINO.

### Core Philosophy
**Stop copying memory. Stop compiling generic binaries. Control your threads.**
