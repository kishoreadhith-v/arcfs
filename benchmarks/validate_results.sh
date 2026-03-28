#!/bin/bash

RESULTS_DIR="$(pwd)/benchmarks/results"

echo "========================================"
echo "📝 Benchmark Results Validation Report"
echo "========================================"

if [ ! -d "$RESULTS_DIR" ]; then
    echo "[!] No results directory found."
    exit 1
fi

function extract_metric {
    local job=$1
    local mount=$2
    local query=$3
    local label=$4
    local unit=$5
    
    local file="${RESULTS_DIR}/${job}_${mount}.json"
    if [ -f "$file" ]; then
        local value=$(jq "$query" "$file")
        echo -e "  ✅ $mount:\t$label: $value $unit"
    else
        echo -e "  ❌ $mount:\tMissing data file!"
    fi
}

echo ""
echo "1. Sequential Write Throughput (Goal: Measure max bandwidth)"
extract_metric "seq_write" "ext4_mount" '.jobs[0].write.bw' "Bandwidth" "KiB/s"
extract_metric "seq_write" "btrfs_mount" '.jobs[0].write.bw' "Bandwidth" "KiB/s"

echo ""
echo "2. Random Write IOPS & Latency (Goal: Extract IOPS and clat_ns)"
extract_metric "rand_write" "ext4_mount" '.jobs[0].write.iops' "IOPS" "ops/sec"
extract_metric "rand_write" "ext4_mount" '.jobs[0].write.clat_ns.mean' "Mean Latency" "ns"
extract_metric "rand_write" "btrfs_mount" '.jobs[0].write.iops' "IOPS" "ops/sec"
extract_metric "rand_write" "btrfs_mount" '.jobs[0].write.clat_ns.mean' "Mean Latency" "ns"

echo ""
echo "3. Realistic Mix (Goal: Test Bimodal workloads)"
extract_metric "realistic_mix" "ext4_mount" '.jobs[0].read.bw' "Read BW" "KiB/s"
extract_metric "realistic_mix" "ext4_mount" '.jobs[0].write.bw' "Write BW" "KiB/s"
extract_metric "realistic_mix" "btrfs_mount" '.jobs[0].read.bw' "Read BW" "KiB/s"
extract_metric "realistic_mix" "btrfs_mount" '.jobs[0].write.bw' "Write BW" "KiB/s"

echo ""
echo "4. Massive Stream (Goal: Memory Pressure / Cache test)"
extract_metric "massive_stream" "ext4_mount" '.jobs[0].write.bw' "Bandwidth" "KiB/s"
extract_metric "massive_stream" "btrfs_mount" '.jobs[0].write.bw' "Bandwidth" "KiB/s"

echo ""
echo "5. Paranoid DB Strict ACID (Goal: fsync overhead)"
extract_metric "paranoid_db" "ext4_mount" '.jobs[0].write.iops' "IOPS" "ops/sec"
extract_metric "paranoid_db" "btrfs_mount" '.jobs[0].write.iops' "IOPS" "ops/sec"

echo ""
echo "========================================"
echo "Data Validation Complete!"