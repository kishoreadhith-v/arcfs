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
MOUNTS=("ext4_mount" "btrfs_mount")

echo "[+] Classes selected: ${CLASSES[*]}"

# 2. Execute Matrix
for class in "${CLASSES[@]}"; do
    for job in "${JOBS[@]}"; do
        echo "----------------------------------------"
        echo "[*] Running Class/Profile: $class / $job"
        echo "----------------------------------------"

        for target in "${MOUNTS[@]}"; do
            target_dir="$ARENA_DIR/$target"

            if [ ! -d "$target_dir" ]; then
                echo "[-] Target missing: $target_dir, skipping..."
                continue
            fi

            echo " -> Testing on $target..."
            mkdir -p "$RESULTS_DIR/$class/$target"
            result_file="$RESULTS_DIR/$class/$target/${job}_${class}_${target}.json"

            sudo rm -rf "$target_dir"/*

            fio --directory="$target_dir" \
                --output-format=json \
                --output="$result_file" \
                "$FIO_JOBS_DIR/$class/${job}.fio"

            echo "    Done. Saved to $result_file"
        done
    done
done

echo "========================================"
echo "[+] All benchmarks completed successfully!"
echo "========================================"