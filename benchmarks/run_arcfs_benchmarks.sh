#!/bin/bash
set -e

ARENA_DIR="/home/kishore/benchmark_arena"
RESULTS_DIR="$(pwd)/benchmarks/results"
FIO_JOBS_DIR="$(pwd)/benchmarks/fio_jobs"
ARCFS_MOUNT="$ARENA_DIR/arcfs_mount"

echo "========================================"
echo " 🚀 ArcFS Benchmark Suite Execution"
echo "========================================"

if ! mountpoint -q "$ARCFS_MOUNT"; then
    echo "[!] ERROR: ArcFS is not mounted at $ARCFS_MOUNT!"
    echo "    Please run the daemon in another terminal first:"
    echo "    cargo run --release -- --storage-dir $ARENA_DIR/arcfs_backend mount $ARCFS_MOUNT"
    exit 1
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
        echo "[*] Running Class/Profile: $class / $job on ArcFS"
        echo "----------------------------------------"

        echo " -> Wiping ArcFS mount to reset state..."
        sudo rm -rf "$ARCFS_MOUNT"/*

        mkdir -p "$RESULTS_DIR/$class/arcfs_mount"
        result_file="$RESULTS_DIR/$class/arcfs_mount/${job}_${class}_arcfs_mount.json"

        fio --directory="$ARCFS_MOUNT" \
            --output-format=json \
            --output="$result_file" \
            "$FIO_JOBS_DIR/$class/${job}.fio"

        echo "    Done. Saved to $result_file"
    done
done

echo "========================================"
echo "[+] ArcFS benchmarks completed successfully!"
echo "========================================"
