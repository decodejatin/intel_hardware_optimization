# Layer 4: Compute & AI Offloading

Your Core Ultra 5 125H contains three separate compute engines. To match Apple Silicon's Neural Engine and GPU performance, you must offload tasks properly.

1.  **Intel Arc GPU (7 Xe-Cores):** For highly parallel math, graphics, and large matrix operations. Contains XMX (Xe Matrix Extensions) hardware.
2.  **Intel AI Boost (NPU):** For low-power, continuous AI inference (background blur, voice recognition, small LLMs).
3.  **CPU (P-Cores):** For low-latency sequential logic.

## Leveraging OpenVINO for AI
Intel's OpenVINO toolkit is the equivalent of Apple's CoreML. It automatically compiles AI models (ONNX, PyTorch) to run optimally across the CPU, GPU, and NPU using Unified Shared Memory.

### Rust Implementation with OpenVINO
Using the `openvino` crate, you can target the GPU and NPU.

```rust
// Cargo.toml: openvino = "0.6"
use openvino::{Core, DeviceType};

pub fn run_ai_inference() {
    let mut core = Core::new().expect("Failed to init OpenVINO");
    
    // Load your model
    let model = core.read_model_from_file("my_llm_model.xml", "my_llm_model.bin")
        .expect("Failed to read model");

    // MAGIC HAPPENS HERE:
    // "MULTI:GPU,NPU" tells OpenVINO to dynamically load balance the inference
    // across your Intel Arc GPU and the low-power NPU, bypassing the CPU entirely.
    let compiled_model = core.compile_model(&model, "MULTI:GPU,NPU")
        .expect("Failed to compile model for Arc & NPU");

    // Create execution request
    let mut request = compiled_model.create_infer_request().unwrap();
    
    // Execute inference (Zero-copy memory under the hood)
    request.infer().unwrap();
}
```

## Leveraging SYCL / oneAPI (C++ Interop)
For absolute maximum performance writing custom compute kernels (to directly access the Arc GPU's XMX matrix multipliers), you should use Intel's oneAPI/SYCL.

1. Write your kernel in `.cpp` using `sycl::queue` and `malloc_shared`.
2. Compile with Intel's `icpx` compiler.
3. Expose a C ABI and call it from your Rust application using the `libc` or `bindgen` crates.
