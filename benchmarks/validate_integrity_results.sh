#!/bin/bash
set -e

RESULTS_DIR="$(pwd)/benchmarks/results/integrity"
TARGETS=("ext4_mount" "bindfs_mount" "btrfs_mount" "arcfs_mount")
PROFILES=("seq_verify" "rand4k_verify" "rand64k_verify" "fsync4k_verify")

echo "========================================"
echo " 🔍 Integrity Results Validation"
echo "========================================"

if [ ! -d "$RESULTS_DIR" ]; then
    echo "[!] No integrity results found at $RESULTS_DIR"
    exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "[!] ERROR: jq is required. Install with: sudo apt install jq"
    exit 1
fi

has_fail=0

for profile in "${PROFILES[@]}"; do
    echo ""
    echo "--- $profile ---"
    for target in "${TARGETS[@]}"; do
        file="$RESULTS_DIR/$target/${profile}_integrity_${target}.json"
        if [ ! -f "$file" ]; then
            echo "  ⚠️  $target: missing file"
            continue
        fi

        err=$(jq -r '.jobs[0].error // 0' "$file")
        w_bw=$(jq -r '(.jobs[0].write.bw_bytes // 0) / 1024 / 1024' "$file")
        w_iops=$(jq -r '.jobs[0].write.iops // 0' "$file")
        p99_ms=$(jq -r '(.jobs[0].write.clat_ns.percentile["99.000000"] // 0) / 1000000' "$file")
        runtime_ms=$(jq -r '.jobs[0].job_runtime // 0' "$file")

        if [ "$err" != "0" ]; then
            echo "  ❌ $target: fio error=$err bw=${w_bw}MB/s iops=${w_iops} p99=${p99_ms}ms runtime=${runtime_ms}ms"
            has_fail=1
        else
            echo "  ✅ $target: bw=${w_bw}MB/s iops=${w_iops} p99=${p99_ms}ms runtime=${runtime_ms}ms"
        fi
    done
done

echo ""
echo "========================================"
if [ "$has_fail" -eq 1 ]; then
    echo "Integrity validation found failures"
    exit 2
else
    echo "Integrity validation passed"
fi
