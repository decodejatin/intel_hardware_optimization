#!/bin/bash
# Intel Core Ultra 5 125H Performance Optimization Script
# Run with: sudo bash optimize.sh
set -e

echo '=== Intel Core Ultra 5 125H Performance Tuning ==='

# Set all CPU governors to performance mode
echo 'Setting CPU governor to performance...'
for gov in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
  echo performance > "$gov" 2>/dev/null || true
done

# Lock P-State to maximum performance
if [ -f /sys/devices/system/cpu/intel_pstate/min_perf_pct ]; then
  echo 100 > /sys/devices/system/cpu/intel_pstate/min_perf_pct
  echo '  ✓ P-State min_perf_pct = 100'
fi

# Reduce swap tendency
sysctl -w vm.swappiness=10

# Enable transparent hugepages
echo always > /sys/kernel/mm/transparent_hugepage/enabled

# Optimize dirty page writeback
sysctl -w vm.dirty_ratio=40
sysctl -w vm.dirty_background_ratio=10

# Disable NUMA balancing (single socket)
sysctl -w kernel.numa_balancing=0

# Disable energy-aware scheduling (forces scheduler to use P-cores)
if [ -f /proc/sys/kernel/sched_energy_aware ]; then
  echo 0 > /proc/sys/kernel/sched_energy_aware
  echo '  ✓ Energy-aware scheduling disabled'
fi

# Raise mlock limit for pinned memory buffers
ulimit -l unlimited 2>/dev/null || true

# Raise PL1 (Sustained Power) to 45W to prevent aggressive thermal throttling
RAPL_PATH="/sys/class/powercap/intel-rapl:0/constraint_0_power_limit_uw"
if [ -f "$RAPL_PATH" ]; then
  # 45 Watts = 45,000,000 microWatts
  echo 45000000 > "$RAPL_PATH" 2>/dev/null || true
  echo '  ✓ PL1 sustained power limit raised to 45W'
fi

echo '=== All optimizations applied ==='
