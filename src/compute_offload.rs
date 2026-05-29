//! # Layer 4: Compute & AI Offloading
//!
//! The Core Ultra 5 125H contains three compute engines:
//! 1. **Intel Arc GPU (7 Xe-Cores)** — Parallel math, graphics, large matrix ops (XMX)
//! 2. **Intel AI Boost (NPU)** — Low-power continuous AI inference
//! 3. **CPU (P-Cores)** — Low-latency sequential logic
//!
//! This module provides the OpenVINO integration for offloading AI inference
//! to the GPU and NPU, bypassing the CPU entirely.
//!
//! NOTE: The `openvino` Rust crate v0.6 uses the legacy Inference Engine C API.
//! OpenVINO 2024+ ships only the 2.0 API. We use `std::panic::catch_unwind` to
//! gracefully handle runtime library mismatches and provide actionable guidance.

use openvino::Core;
use std::panic;

/// Runs AI model inference on the Intel Arc GPU + NPU using OpenVINO.
///
/// The model is loaded via `read_network_from_file` and compiled with
/// `"MULTI:GPU,NPU"` target for dynamic load balancing.
///
/// # Arguments
/// * `model_xml` - Path to the OpenVINO IR model XML file
/// * `model_bin` - Path to the OpenVINO IR model weights BIN file
pub fn run_ai_inference(model_xml: &str, model_bin: &str) {
    let result = panic::catch_unwind(|| {
        let mut core = Core::new(None).expect("Failed to initialize OpenVINO runtime");

        let network = core
            .read_network_from_file(model_xml, model_bin)
            .expect("Failed to read OpenVINO model");

        println!("[Compute] Model loaded: {} -> {}", model_xml, model_bin);

        let mut executable = core
            .load_network(&network, "MULTI:GPU,NPU")
            .expect("Failed to compile model for Arc & NPU");

        println!("[Compute] Model compiled for MULTI:GPU,NPU target");

        let mut request = executable
            .create_infer_request()
            .expect("Failed to create inference request");

        request.infer().expect("Inference execution failed");

        println!("[Compute] Inference complete (GPU + NPU offload)");
    });

    if let Err(_) = result {
        eprintln!("[Compute] OpenVINO inference failed. See list_available_devices() for details.");
    }
}

/// Runs inference on CPU only (fallback).
pub fn run_cpu_inference(model_xml: &str, model_bin: &str) {
    let result = panic::catch_unwind(|| {
        let mut core = Core::new(None).expect("Failed to initialize OpenVINO runtime");

        let network = core
            .read_network_from_file(model_xml, model_bin)
            .expect("Failed to read OpenVINO model");

        let mut executable = core
            .load_network(&network, "CPU")
            .expect("Failed to compile model for CPU");

        let mut request = executable
            .create_infer_request()
            .expect("Failed to create inference request");

        request.infer().expect("Inference execution failed");

        println!("[Compute] CPU-only inference complete");
    });

    if let Err(_) = result {
        eprintln!("[Compute] CPU inference failed.");
    }
}

/// Detects and prints available compute devices and OpenVINO status.
pub fn list_available_devices() {
    // Check if the OpenVINO C library is present on the system
    let ov_lib_found = std::env::var("LD_LIBRARY_PATH")
        .unwrap_or_default()
        .contains("openvino");

    println!("[Compute] Available compute hardware:");

    // Check GPU
    let gpu_info = std::fs::read_to_string("/sys/bus/pci/devices/0000:00:02.0/device")
        .unwrap_or_default();
    if !gpu_info.is_empty() {
        println!("  ✓ Intel Arc Graphics (Meteor Lake-P) detected");
    } else {
        println!("  ✗ Intel Arc GPU not detected");
    }

    // Check NPU
    let npu_present = std::path::Path::new("/sys/bus/pci/devices/0000:00:0b.0").exists();
    if npu_present {
        println!("  ✓ Intel AI Boost NPU detected");
    } else {
        println!("  ✗ Intel NPU not detected");
    }

    // Check OpenVINO runtime
    let ov_result = panic::catch_unwind(|| {
        Core::new(None).ok()
    });

    match ov_result {
        Ok(Some(_)) => {
            println!("  ✓ OpenVINO runtime loaded successfully");
        }
        Ok(None) => {
            println!("  ✗ OpenVINO runtime failed to initialize");
            print_openvino_install_help();
        }
        Err(_) => {
            // API version mismatch (openvino crate 0.6 vs OpenVINO 2024+ runtime)
            if ov_lib_found {
                println!("  ⚠ OpenVINO library found but API version mismatch");
                println!("    The openvino Rust crate v0.6 uses the legacy IE C API,");
                println!("    but your OpenVINO 2024 runtime exports the 2.0 API.");
                println!("    → Fix: Update to openvino crate v0.7+ when available,");
                println!("      or use OpenVINO via Python/C++ bindings directly.");
            } else {
                println!("  ✗ OpenVINO runtime not available");
                print_openvino_install_help();
            }
        }
    }

    // Show what can be done without OpenVINO
    println!();
    println!("[Compute] Direct GPU compute via wgpu (Layer 3) is available without OpenVINO.");
    println!("[Compute] For AI inference, use OpenVINO Python or C++ bindings.");
}

fn print_openvino_install_help() {
    println!("    Install: download from https://storage.openvinotoolkit.org/");
    println!("    Then:  export LD_LIBRARY_PATH=/path/to/openvino/runtime/lib/intel64");
}
