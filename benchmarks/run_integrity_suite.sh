#!/bin/bash
set -e

ARENA_DIR="/home/kishore/benchmark_arena"
RESULTS_DIR="$(pwd)/benchmarks/results/integrity"
FIO_JOBS_DIR="$(pwd)/benchmarks/fio_jobs/integrity"
BINDFS_MOUNT="$ARENA_DIR/bindfs_mount"

PROFILES=("seq_verify" "rand4k_verify" "rand64k_verify" "fsync4k_verify")
TARGETS=("ext4_mount" "btrfs_mount" "bindfs_mount" "arcfs_mount")

if [ -n "${INTEGRITY_PROFILE:-}" ]; then
    PROFILES=("$INTEGRITY_PROFILE")
fi

if [ -n "${INTEGRITY_TARGET:-}" ]; then
    TARGETS=("$INTEGRITY_TARGET")
fi

echo "========================================"
echo " 🔒 Integrity Verification Suite"
echo "========================================"

if [ ! -d "$ARENA_DIR" ]; then
    echo "[!] ERROR: benchmark arena not found: $ARENA_DIR"
    exit 1
fi

if ! command -v fio >/dev/null 2>&1; then
    echo "[!] ERROR: fio not found. Install with: sudo apt install fio"
    exit 1
fi

mkdir -p "$RESULTS_DIR"

# Prepare bindfs mount if available
if ! mountpoint -q "$BINDFS_MOUNT"; then
    if command -v bindfs >/dev/null 2>&1; then
        mkdir -p "$BINDFS_MOUNT"
        echo "[+] Mounting bindfs on $BINDFS_MOUNT"
        sudo bindfs "$ARENA_DIR/ext4_mount" "$BINDFS_MOUNT"
    else
        echo "[!] bindfs not installed; bindfs target will be skipped"
    fi
fi

run_profile_on_target() {
    local profile=$1
    local target=$2
    local target_dir="$ARENA_DIR/$target"

    if [ ! -d "$target_dir" ]; then
        echo "[-] $target missing at $target_dir; skipping"
        return
    fi

    if [ "$target" = "arcfs_mount" ] && ! mountpoint -q "$target_dir"; then
        echo "[-] ArcFS not mounted at $target_dir; skipping"
        return
    fi

    if [ "$target" = "bindfs_mount" ] && ! mountpoint -q "$target_dir"; then
        echo "[-] BindFS not mounted at $target_dir; skipping"
        return
    fi

    if [ "$target" = "bindfs_mount" ]; then
        sudo rm -rf "$ARENA_DIR/ext4_mount"/*
    else
        sudo rm -rf "$target_dir"/*
    fi

    mkdir -p "$RESULTS_DIR/$target"
    local result_file="$RESULTS_DIR/$target/${profile}_integrity_${target}.json"

    echo " -> $profile on $target"
    fio --directory="$target_dir" \
        --output-format=json \
        --output="$result_file" \
        "$FIO_JOBS_DIR/${profile}.fio"

    echo "    saved: $result_file"
}

for profile in "${PROFILES[@]}"; do
    echo "----------------------------------------"
    echo "[*] Running integrity profile: $profile"
    echo "----------------------------------------"
    for target in "${TARGETS[@]}"; do
        run_profile_on_target "$profile" "$target"
    done
done

echo "========================================"
echo "[+] Integrity suite completed"
echo "========================================"
