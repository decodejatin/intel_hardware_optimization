//! # System Tuner: Real OS-Level Optimization Applicator
//!
//! Reads and applies system-level kernel parameters for maximum performance
//! on the Intel Core Ultra 5 125H. These are the tunings that actually make
//! the difference between "generic PC" and "Apple Silicon parity."

use std::fs;

/// Represents a single tunable parameter.
#[derive(Debug)]
pub struct Tunable {
    pub name: &'static str,
    pub path: &'static str,
    pub current: String,
    pub recommended: &'static str,
    pub description: &'static str,
}

/// Audits the current system and returns all tunables with their current values.
pub fn audit_system() -> Vec<Tunable> {
    let mut tunables = Vec::new();

    // 1. CPU Governor
    let gov = read_sysfs_first("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor");
    tunables.push(Tunable {
        name: "CPU Governor",
        path: "/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor",
        current: gov,
        recommended: "performance",
        description: "powersave throttles P-cores to save battery. performance locks them at max frequency.",
    });

    // 2. Intel P-State min frequency percentage
    let min_perf = read_sysfs_first("/sys/devices/system/cpu/intel_pstate/min_perf_pct");
    tunables.push(Tunable {
        name: "Intel P-State Min Perf %",
        path: "/sys/devices/system/cpu/intel_pstate/min_perf_pct",
        current: min_perf,
        recommended: "100",
        description: "Forces P-cores to always run at max frequency. Eliminates ramp-up latency.",
    });

    // 3. VM Swappiness
    let swappiness = read_sysfs_first("/proc/sys/vm/swappiness");
    tunables.push(Tunable {
        name: "VM Swappiness",
        path: "/proc/sys/vm/swappiness",
        current: swappiness,
        recommended: "10",
        description: "Reduces kernel tendency to swap pages to disk. Critical for compute buffers.",
    });

    // 4. Transparent Hugepages
    let thp = read_sysfs_first("/sys/kernel/mm/transparent_hugepage/enabled");
    tunables.push(Tunable {
        name: "Transparent Hugepages",
        path: "/sys/kernel/mm/transparent_hugepage/enabled",
        current: thp,
        recommended: "[always]",
        description: "Enables 2MB pages to reduce TLB misses for large memory allocations.",
    });

    // 5. Dirty ratio (how much RAM can be dirty before writeback)
    let dirty = read_sysfs_first("/proc/sys/vm/dirty_ratio");
    tunables.push(Tunable {
        name: "VM Dirty Ratio",
        path: "/proc/sys/vm/dirty_ratio",
        current: dirty,
        recommended: "40",
        description: "Higher dirty ratio delays disk writeback, keeping more data in RAM longer.",
    });

    // 6. Dirty background ratio
    let dirty_bg = read_sysfs_first("/proc/sys/vm/dirty_background_ratio");
    tunables.push(Tunable {
        name: "VM Dirty Background Ratio",
        path: "/proc/sys/vm/dirty_background_ratio",
        current: dirty_bg,
        recommended: "10",
        description: "When background writeback starts. Lower = smoother, less bursty IO.",
    });

    // 7. NUMA balancing (should be off for single-socket)
    let numa = read_sysfs_first("/proc/sys/kernel/numa_balancing");
    tunables.push(Tunable {
        name: "NUMA Balancing",
        path: "/proc/sys/kernel/numa_balancing",
        current: numa,
        recommended: "0",
        description: "Single-socket system. NUMA balancing adds overhead with no benefit.",
    });

    // 8. sched_energy_aware (scheduler energy awareness)
    let energy = read_sysfs_first("/proc/sys/kernel/sched_energy_aware");
    tunables.push(Tunable {
        name: "Sched Energy Aware",
        path: "/proc/sys/kernel/sched_energy_aware",
        current: energy,
        recommended: "0",
        description: "When ON, scheduler prefers E-cores to save power. Turn OFF for max perf.",
    });

    tunables
}

/// Prints the audit results as a table.
pub fn print_audit() {
    let tunables = audit_system();

    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│              SYSTEM PERFORMANCE AUDIT                            │");
    println!("├──────────────────────────────────────────────────────────────────┤");

    for t in &tunables {
        let status = if t.current.contains(t.recommended) || t.current.trim() == t.recommended {
            "✓"
        } else {
            "✗"
        };

        println!(
            "│ {} {:<30} current={:<12} rec={:<12} │",
            status,
            t.name,
            t.current.trim(),
            t.recommended
        );
    }

    let ok_count = tunables
        .iter()
        .filter(|t| t.current.contains(t.recommended) || t.current.trim() == t.recommended)
        .count();

    println!("├──────────────────────────────────────────────────────────────────┤");
    println!(
        "│ Score: {}/{} optimized                                            │",
        ok_count,
        tunables.len()
    );
    println!("└──────────────────────────────────────────────────────────────────┘");
}

/// Generates a shell script that applies all recommended optimizations.
/// Must be run as root.
pub fn generate_optimization_script() -> String {
    let mut script = String::from("#!/bin/bash\n");
    script.push_str("# Intel Core Ultra 5 125H Performance Optimization Script\n");
    script.push_str("# Run with: sudo bash optimize.sh\n");
    script.push_str("set -e\n\n");

    script.push_str("echo '=== Intel Core Ultra 5 125H Performance Tuning ==='\n\n");

    // CPU Governor → performance
    script.push_str("# Set all CPU governors to performance mode\n");
    script.push_str("echo 'Setting CPU governor to performance...'\n");
    script.push_str("for gov in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do\n");
    script.push_str("  echo performance > \"$gov\" 2>/dev/null || true\n");
    script.push_str("done\n\n");

    // P-State min perf
    script.push_str("# Lock P-State to maximum performance\n");
    script.push_str("if [ -f /sys/devices/system/cpu/intel_pstate/min_perf_pct ]; then\n");
    script.push_str("  echo 100 > /sys/devices/system/cpu/intel_pstate/min_perf_pct\n");
    script.push_str("  echo '  ✓ P-State min_perf_pct = 100'\n");
    script.push_str("fi\n\n");

    // Swappiness
    script.push_str("# Reduce swap tendency\n");
    script.push_str("sysctl -w vm.swappiness=10\n\n");

    // THP
    script.push_str("# Enable transparent hugepages\n");
    script.push_str("echo always > /sys/kernel/mm/transparent_hugepage/enabled\n\n");

    // Dirty ratios
    script.push_str("# Optimize dirty page writeback\n");
    script.push_str("sysctl -w vm.dirty_ratio=40\n");
    script.push_str("sysctl -w vm.dirty_background_ratio=10\n\n");

    // NUMA balancing off
    script.push_str("# Disable NUMA balancing (single socket)\n");
    script.push_str("sysctl -w kernel.numa_balancing=0\n\n");

    // Disable energy-aware scheduling
    script.push_str("# Disable energy-aware scheduling (forces scheduler to use P-cores)\n");
    script.push_str("if [ -f /proc/sys/kernel/sched_energy_aware ]; then\n");
    script.push_str("  echo 0 > /proc/sys/kernel/sched_energy_aware\n");
    script.push_str("  echo '  ✓ Energy-aware scheduling disabled'\n");
    script.push_str("fi\n\n");

    // Raise mlock limit
    script.push_str("# Raise mlock limit for pinned memory buffers\n");
    script.push_str("ulimit -l unlimited 2>/dev/null || true\n\n");

    script.push_str("echo '=== All optimizations applied ==='\n");

    script
}

fn read_sysfs_first(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| "N/A".to_string())
        .trim()
        .to_string()
}
