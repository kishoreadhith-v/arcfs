#!/bin/bash
set -e

ARENA_DIR="/home/kishore/benchmark_arena"
RESULTS_DIR="$(pwd)/benchmarks/results"
FIO_JOBS_DIR="$(pwd)/benchmarks/fio_jobs"

echo "========================================"
echo " BetterFS Benchmark Suite Execution"
echo "========================================"

# 1. Safety & Environment Checks
if [ ! -d "$ARENA_DIR" ]; then
    echo "[!] ERROR: Benchmark arena not found at $ARENA_DIR."
    echo "    Please create the loopback arena outside the repo first."
    exit 1
fi

if ! command -v fio &> /dev/null; then
    echo "[!] ERROR: 'fio' is not installed. Run: sudo apt install fio"
    exit 1
fi

mkdir -p "$RESULTS_DIR"
echo "[+] Results will be saved to: $RESULTS_DIR"

# Fio job list mapped exactly to the spec
JOBS=("seq_write" "rand_write" "realistic_mix" "massive_stream" "paranoid_db")
MOUNTS=("ext4_mount" "btrfs_mount")

# 2. Execute Matrix
for job in "${JOBS[@]}"; do
    echo "----------------------------------------"
    echo "[*] Running Profile: $job"
    echo "----------------------------------------"
    
    for target in "${MOUNTS[@]}"; do
        target_dir="$ARENA_DIR/$target"
        
        # Check if the target directory exists and is writable
        if [ ! -d "$target_dir" ]; then
            echo "[-] Target missing: $target_dir, skipping..."
            continue
        fi

        echo " -> Testing on $target..."
        result_file="$RESULTS_DIR/${job}_${target}.json"
        
        # Clean up files in arena to prevent filling up the 5GB loopback
        sudo rm -rf "$target_dir"/*

        # Run FIO and override the target directory dynamically
        fio --directory="$target_dir" \
            --output-format=json \
            --output="$result_file" \
            "$FIO_JOBS_DIR/${job}.fio"
            
        echo "    Done. Saved to $result_file"
    done
done

echo "========================================"
echo "[+] All benchmarks completed successfully!"
echo "========================================"