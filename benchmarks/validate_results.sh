#!/bin/bash
set -e

RESULTS_DIR="$(pwd)/benchmarks/results"
ALL_CLASSES=("responsive" "durable" "worst_case")
if [ -n "${CLASS:-}" ]; then
    IFS=',' read -r -a CLASSES <<< "$CLASS"
    for class in "${CLASSES[@]}"; do
        if [[ ! " ${ALL_CLASSES[*]} " =~ " ${class} " ]]; then
            echo "[!] ERROR: Unsupported CLASS='$class'. Allowed: ${ALL_CLASSES[*]}"
            exit 1
        fi
    done
else
    CLASSES=("${ALL_CLASSES[@]}")
fi
JOBS=("seq_write" "rand_write" "realistic_mix" "massive_stream" "paranoid_db")
MOUNTS=("ext4_mount" "bindfs_mount" "btrfs_mount" "arcfs_mount")

echo "========================================"
echo "📝 Benchmark Results Validation Report"
echo "========================================"
echo "[+] Classes selected: ${CLASSES[*]}"

if [ ! -d "$RESULTS_DIR" ]; then
    echo "[!] No results directory found."
    exit 1
fi

if ! command -v jq > /dev/null 2>&1; then
    echo "[!] ERROR: jq is required. Install with: sudo apt install jq"
    exit 1
fi

get_metric() {
    local file=$1
    local query=$2
    jq -r "$query // 0" "$file"
}

validate_depth_fairness() {
    local job=$1
    local file=$2

    if [[ "$job" == "rand_write" || "$job" == "realistic_mix" || "$job" == "paranoid_db" ]]; then
        local configured_depth
        configured_depth=$(get_metric "$file" '.jobs[0]["job options"].iodepth')
        if [[ "$configured_depth" == "1" ]]; then
            return 0
        fi

        local depth1
        depth1=$(get_metric "$file" '.jobs[0].iodepth_level["1"]')
        awk -v d="$depth1" 'BEGIN {
            if (d > 80.0) {
                printf("  ⚠️  effective iodepth looks shallow: depth=1 is %.2f%%\n", d)
            }
        }'
    fi
}

for class in "${CLASSES[@]}"; do
    echo ""
    echo "========================================"
    echo "Class: $class"
    echo "========================================"

    for job in "${JOBS[@]}"; do
        echo ""
        echo "--- $job ---"
        for mount in "${MOUNTS[@]}"; do
            file="${RESULTS_DIR}/${class}/${mount}/${job}_${class}_${mount}.json"

            if [ ! -f "$file" ]; then
                echo "  ❌ $mount: missing ${job}_${class}_${mount}.json"
                continue
            fi

            write_bw_mb=$(get_metric "$file" '.jobs[0].write.bw_bytes / 1024 / 1024')
            read_bw_mb=$(get_metric "$file" '.jobs[0].read.bw_bytes / 1024 / 1024')
            write_iops=$(get_metric "$file" '.jobs[0].write.iops')
            read_iops=$(get_metric "$file" '.jobs[0].read.iops')
            p99_ms=$(get_metric "$file" '.jobs[0].write.clat_ns.percentile["99.000000"] / 1000000')
            p999_ms=$(get_metric "$file" '.jobs[0].write.clat_ns.percentile["99.900000"] / 1000000')
            mean_ms=$(get_metric "$file" '.jobs[0].write.clat_ns.mean / 1000000')
            runtime_ms=$(get_metric "$file" '.jobs[0].job_runtime')
            usr_cpu=$(get_metric "$file" '.jobs[0].usr_cpu')
            sys_cpu=$(get_metric "$file" '.jobs[0].sys_cpu')

            echo "  ✅ $mount: bw_w=${write_bw_mb}MB/s bw_r=${read_bw_mb}MB/s iops_w=${write_iops} iops_r=${read_iops} lat_mean=${mean_ms}ms p99=${p99_ms}ms p99.9=${p999_ms}ms runtime=${runtime_ms}ms cpu=${usr_cpu}+${sys_cpu}%"

            validate_depth_fairness "$job" "$file"
        done
    done
done

echo ""
echo "========================================"
echo "Data Validation Complete (3-class suite)!"