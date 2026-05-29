//! # Power Limit Prober (RAPL)
//!
//! Intel chips use RAPL (Running Average Power Limit) to manage thermals.
//! This module probes the sysfs interface to read the PL1 (Long Term) and
//! PL2 (Short Term) power limits in watts.

use std::fs;
use std::path::Path;

pub struct PowerLimits {
    pub pl1_watts: f64,
    pub pl2_watts: f64,
}

/// Reads the hardware power limits from the Intel RAPL interface.
pub fn read_power_limits() -> Option<PowerLimits> {
    let base_path = "/sys/class/powercap/intel-rapl:0";
    
    if !Path::new(base_path).exists() {
        return None;
    }

    // RAPL values are in microwatts. We divide by 1_000_000 to get Watts.
    let pl1_uw = fs::read_to_string(format!("{}/constraint_0_power_limit_uw", base_path))
        .ok()?
        .trim()
        .parse::<f64>()
        .ok()?;
        
    let pl2_uw = fs::read_to_string(format!("{}/constraint_1_power_limit_uw", base_path))
        .ok()?
        .trim()
        .parse::<f64>()
        .ok()?;

    Some(PowerLimits {
        pl1_watts: pl1_uw / 1_000_000.0,
        pl2_watts: pl2_uw / 1_000_000.0,
    })
}

pub fn print_power_limits() {
    println!("┌─────────────────────────────────────────────────┐");
    println!("│          HARDWARE POWER LIMITS (RAPL)           │");
    println!("├─────────────────────────────────────────────────┤");
    
    match read_power_limits() {
        Some(limits) => {
            println!("│  PL1 (Sustained Power): {:>6.1} W              │", limits.pl1_watts);
            println!("│  PL2 (Boost Power):     {:>6.1} W              │", limits.pl2_watts);
            println!("│                                                 │");
            println!("│  * If PL1 is much lower than PL2, the chip      │");
            println!("│    will throttle heavily during long workloads. │");
        }
        None => {
            println!("│  ✗ RAPL interface not accessible or missing.    │");
        }
    }
    println!("└─────────────────────────────────────────────────┘");
}
