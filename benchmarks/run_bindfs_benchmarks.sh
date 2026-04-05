#!/bin/bash
set -e

ARENA_DIR="/home/kishore/benchmark_arena"
RESULTS_DIR="$(pwd)/benchmarks/results"
FIO_JOBS_DIR="$(pwd)/benchmarks/fio_jobs"
BINDFS_MOUNT="$ARENA_DIR/bindfs_mount"

echo "========================================"
echo " 🚀 BindFS (FUSE Baseline) Benchmark Suite"
echo "========================================"

mkdir -p "$BINDFS_MOUNT"

if ! mountpoint -q "$BINDFS_MOUNT"; then
    echo "[+] Mounting Ext4 through BindFS (FUSE pass-through)..."
    sudo bindfs "$ARENA_DIR/ext4_mount" "$BINDFS_MOUNT"
fi

mkdir -p "$RESULTS_DIR"

# Three-class suite (override with CLASS, e.g. CLASS=worst_case or CLASS=responsive,durable)
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

echo "[+] Classes selected: ${CLASSES[*]}"

for class in "${CLASSES[@]}"; do
    for job in "${JOBS[@]}"; do
        echo "----------------------------------------"
        echo "[*] Running Class/Profile: $class / $job on BindFS"
        echo "----------------------------------------"

        sudo rm -rf "$ARENA_DIR/ext4_mount/"*

        mkdir -p "$RESULTS_DIR/$class/bindfs_mount"
        result_file="$RESULTS_DIR/$class/bindfs_mount/${job}_${class}_bindfs_mount.json"

        fio --directory="$BINDFS_MOUNT" \
            --output-format=json \
            --output="$result_file" \
            "$FIO_JOBS_DIR/$class/${job}.fio"

        echo "    Done. Saved to $result_file"
    done
done

echo "========================================"
echo "[+] BindFS benchmarks completed successfully!"
echo "========================================"
sudo umount "$BINDFS_MOUNT"
