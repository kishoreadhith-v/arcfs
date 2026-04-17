#!/bin/bash
set -e

# ==============================================================================
# ArcFS Disk Usage Benchmark Suite
# ==============================================================================
# Measures on-disk storage consumption across ext4, btrfs, bindfs, and arcfs
# under identical workloads. Produces JSON results for chart generation.
#
# Usage:
#   ./benchmarks/run_disk_usage_benchmarks.sh              # run all tests on all filesystems
#   TESTS=dedup,compress ./benchmarks/run_disk_usage_benchmarks.sh  # run specific tests
#   FS=ext4,arcfs ./benchmarks/run_disk_usage_benchmarks.sh         # run specific filesystems
# ==============================================================================

ARENA_DIR="$HOME/benchmark_arena"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results/disk_usage"
TIMEOUT_SEC=120

# Filesystems
EXT4_MOUNT="$ARENA_DIR/ext4_mount"
BTRFS_MOUNT="$ARENA_DIR/btrfs_mount"
BINDFS_MOUNT="$ARENA_DIR/bindfs_mount"
ARCFS_MOUNT="$ARENA_DIR/arcfs_mount"
ARCFS_BACKEND="$ARENA_DIR/arcfs_backend"

# Test parameters (kept small for speed)
DEDUP_FILE_SIZE_MB=20
DEDUP_MAX_COPIES=10
COMPRESS_FILE_SIZE_MB=20
SNAPSHOT_BASE_SIZE_MB=40
SNAPSHOT_COUNT=5
SNAPSHOT_CHURN_PCT=10
SMALL_FILES_COUNT=500
SMALL_FILES_SIZE_KB=4
MIXED_SOURCE_MB=10
MIXED_LOG_MB=15
MIXED_BINARY_MB=15

# Timestamps for logging
ts() { date '+%H:%M:%S'; }

log_info()  { echo "[$(ts)] [INFO]  $*"; }
log_warn()  { echo "[$(ts)] [WARN]  $*"; }
log_error() { echo "[$(ts)] [ERROR] $*" >&2; }
log_step()  { echo "[$(ts)] [STEP]  >> $*"; }
log_data()  { echo "[$(ts)] [DATA]  $*"; }

# ==============================================================================
# Helpers
# ==============================================================================

measure_physical_bytes() {
    local fs_type="$1"
    local mount_dir="$2"
    sync
    case "$fs_type" in
        ext4)
            # Actual used bytes on the loopback image
            df -B1 "$mount_dir" | awk 'NR==2 {print $3}'
            ;;
        btrfs)
            df -B1 "$mount_dir" | awk 'NR==2 {print $3}'
            ;;
        bindfs)
            # bindfs sits on ext4, measure the ext4 backing
            df -B1 "$EXT4_MOUNT" | awk 'NR==2 {print $3}'
            ;;
        arcfs)
            # CAS pool + metadata db = true physical footprint
            local cas_bytes=0
            local meta_bytes=0
            if [ -d "$ARCFS_BACKEND/cas" ]; then
                cas_bytes=$(du -sb "$ARCFS_BACKEND/cas" 2>/dev/null | awk '{print $1}')
            fi
            if [ -d "$ARCFS_BACKEND/metadata_db" ]; then
                meta_bytes=$(du -sb "$ARCFS_BACKEND/metadata_db" 2>/dev/null | awk '{print $1}')
            fi
            echo $(( cas_bytes + meta_bytes ))
            ;;
    esac
}

measure_logical_bytes() {
    local mount_dir="$1"
    # Use find + stat to sum actual file sizes, avoiding btrfs/FUSE metadata inflation from du
    find "$mount_dir" -type f -exec stat -c%s {} + 2>/dev/null | awk '{s+=$1} END {print s+0}'
}

wipe_mount() {
    local fs_type="$1"
    local mount_dir="$2"
    log_step "Wiping $mount_dir for $fs_type..."
    if [ "$fs_type" = "arcfs" ]; then
        # For arcfs, remove files through the FUSE mount AND wipe backend
        rm -rf "$mount_dir"/* 2>/dev/null || true
        sync
        # Wipe both CAS and metadata_db to fully reset physical storage
        rm -rf "$ARCFS_BACKEND/cas"/* 2>/dev/null || true
        rm -rf "$ARCFS_BACKEND/metadata_db"/* 2>/dev/null || true
    elif [ "$fs_type" = "bindfs" ]; then
        # bindfs writes go to ext4 backing
        sudo rm -rf "$EXT4_MOUNT"/* 2>/dev/null || true
    else
        sudo rm -rf "$mount_dir"/* 2>/dev/null || true
    fi
    sync
}

write_file_with_timeout() {
    local desc="$1"; shift
    log_step "$desc"
    if ! timeout "$TIMEOUT_SEC" "$@"; then
        log_error "TIMEOUT after ${TIMEOUT_SEC}s: $desc"
        return 1
    fi
}

# Generate data with controlled compressibility using dd + python
generate_data_file() {
    local output="$1"
    local size_mb="$2"
    local compress_pct="$3"  # 0=incompressible, 100=fully compressible

    local zero_bytes=$(( size_mb * 1024 * 1024 * compress_pct / 100 ))
    local rand_bytes=$(( size_mb * 1024 * 1024 - zero_bytes ))

    if [ "$rand_bytes" -gt 0 ]; then
        dd if=/dev/urandom bs=1M count=$(( rand_bytes / 1048576 )) 2>/dev/null > "$output"
    else
        > "$output"
    fi
    if [ "$zero_bytes" -gt 0 ]; then
        dd if=/dev/zero bs=1M count=$(( zero_bytes / 1048576 )) 2>/dev/null >> "$output"
    fi
}

save_result() {
    local test_name="$1"
    local fs_type="$2"
    local json_content="$3"
    local outfile="$RESULTS_DIR/${test_name}_${fs_type}.json"
    echo "$json_content" > "$outfile"
    log_info "Result saved: $outfile"
}

# ==============================================================================
# Test A: Duplicate File Test
# ==============================================================================
run_dedup_test() {
    local fs_type="$1"
    local mount_dir="$2"

    log_info "=== DEDUP TEST on $fs_type ($mount_dir) ==="
    wipe_mount "$fs_type" "$mount_dir"

    # Baseline physical usage (empty)
    local base_physical
    base_physical=$(measure_physical_bytes "$fs_type" "$mount_dir")
    log_data "Baseline physical: $base_physical bytes"

    # Create source file
    write_file_with_timeout "Creating ${DEDUP_FILE_SIZE_MB}MB source file" \
        dd if=/dev/urandom of="$mount_dir/source.dat" bs=1M count="$DEDUP_FILE_SIZE_MB" 2>/dev/null
    sync

    local steps="[]"

    for n in 1 2 5 10; do
        if [ "$n" -gt "$DEDUP_MAX_COPIES" ]; then break; fi
        log_step "Creating $n copies..."

        # Remove old copies
        rm -f "$mount_dir"/copy_*.dat 2>/dev/null || true
        sync

        for i in $(seq 1 "$n"); do
            if [ "$fs_type" = "btrfs" ]; then
                # Use reflink for btrfs to test CoW dedup
                timeout "$TIMEOUT_SEC" cp --reflink=always "$mount_dir/source.dat" "$mount_dir/copy_${i}.dat" 2>/dev/null || \
                timeout "$TIMEOUT_SEC" cp "$mount_dir/source.dat" "$mount_dir/copy_${i}.dat"
            elif [ "$fs_type" = "arcfs" ] || [ "$fs_type" = "bindfs" ]; then
                # FUSE mounts can deadlock on same-mount cp; route through host temp
                timeout "$TIMEOUT_SEC" cp "$mount_dir/source.dat" /tmp/_arcfs_dedup_tmp.dat
                timeout "$TIMEOUT_SEC" cp /tmp/_arcfs_dedup_tmp.dat "$mount_dir/copy_${i}.dat"
                rm -f /tmp/_arcfs_dedup_tmp.dat
            else
                timeout "$TIMEOUT_SEC" cp "$mount_dir/source.dat" "$mount_dir/copy_${i}.dat"
            fi
        done
        sync

        local logical physical
        logical=$(measure_logical_bytes "$mount_dir")
        physical=$(measure_physical_bytes "$fs_type" "$mount_dir")
        physical=$(( physical - base_physical ))

        log_data "copies=$n logical=${logical} physical=${physical}"
        steps=$(echo "$steps" | python3 -c "
import json, sys
s = json.load(sys.stdin)
s.append({'copies': $n, 'logical_bytes': $logical, 'physical_bytes': $physical})
print(json.dumps(s))
")
    done

    save_result "dedup" "$fs_type" "$(cat <<EOF
{
  "test": "dedup",
  "filesystem": "$fs_type",
  "file_size_mb": $DEDUP_FILE_SIZE_MB,
  "steps": $steps
}
EOF
)"
}

# ==============================================================================
# Test B: Compression Efficiency
# ==============================================================================
run_compress_test() {
    local fs_type="$1"
    local mount_dir="$2"

    log_info "=== COMPRESSION TEST on $fs_type ($mount_dir) ==="
    wipe_mount "$fs_type" "$mount_dir"

    local base_physical
    base_physical=$(measure_physical_bytes "$fs_type" "$mount_dir")

    local steps="[]"

    for pct in 0 25 50 75 100; do
        log_step "Writing ${COMPRESS_FILE_SIZE_MB}MB file with ${pct}% compressibility..."
        wipe_mount "$fs_type" "$mount_dir"
        local cur_base
        cur_base=$(measure_physical_bytes "$fs_type" "$mount_dir")

        generate_data_file "$mount_dir/data_${pct}.dat" "$COMPRESS_FILE_SIZE_MB" "$pct"
        sync

        local logical physical
        logical=$(stat -c%s "$mount_dir/data_${pct}.dat" 2>/dev/null || echo 0)
        physical=$(measure_physical_bytes "$fs_type" "$mount_dir")
        physical=$(( physical - cur_base ))

        local ratio="1.0"
        if [ "$physical" -gt 0 ] && [ "$logical" -gt 0 ]; then
            ratio=$(python3 -c "print(round($physical / $logical, 4))")
        fi

        log_data "compress_pct=$pct logical=$logical physical=$physical ratio=$ratio"
        steps=$(echo "$steps" | python3 -c "
import json, sys
s = json.load(sys.stdin)
s.append({'compress_pct': $pct, 'logical_bytes': $logical, 'physical_bytes': $physical, 'ratio': $ratio})
print(json.dumps(s))
")
    done

    save_result "compress" "$fs_type" "$(cat <<EOF
{
  "test": "compress",
  "filesystem": "$fs_type",
  "file_size_mb": $COMPRESS_FILE_SIZE_MB,
  "steps": $steps
}
EOF
)"
}

# ==============================================================================
# Test C: Snapshot Storage Cost
# ==============================================================================
run_snapshot_test() {
    local fs_type="$1"
    local mount_dir="$2"

    log_info "=== SNAPSHOT TEST on $fs_type ($mount_dir) ==="
    wipe_mount "$fs_type" "$mount_dir"

    local base_physical
    base_physical=$(measure_physical_bytes "$fs_type" "$mount_dir")

    # Create base dataset: a directory with several files
    log_step "Creating ${SNAPSHOT_BASE_SIZE_MB}MB base dataset..."
    mkdir -p "$mount_dir/data"
    local files_count=10
    local per_file_mb=$(( SNAPSHOT_BASE_SIZE_MB / files_count ))
    for i in $(seq 1 "$files_count"); do
        dd if=/dev/urandom of="$mount_dir/data/file_${i}.dat" bs=1M count="$per_file_mb" 2>/dev/null
    done
    sync

    local steps="[]"
    local logical physical
    logical=$(measure_logical_bytes "$mount_dir")
    physical=$(measure_physical_bytes "$fs_type" "$mount_dir")
    physical=$(( physical - base_physical ))
    log_data "snapshot=0 (base) logical=$logical physical=$physical"
    steps=$(echo "$steps" | python3 -c "
import json, sys
s = json.load(sys.stdin)
s.append({'snapshot': 0, 'logical_bytes': $logical, 'physical_bytes': $physical})
print(json.dumps(s))
")

    for snap in $(seq 1 "$SNAPSHOT_COUNT"); do
        log_step "Taking snapshot $snap and modifying ${SNAPSHOT_CHURN_PCT}% of data..."

        if [ "$fs_type" = "btrfs" ]; then
            # btrfs native snapshot
            sudo btrfs subvolume snapshot "$mount_dir" "$mount_dir/.snap_v${snap}" 2>/dev/null || \
                cp -a "$mount_dir/data" "$mount_dir/snap_v${snap}"
        elif [ "$fs_type" = "arcfs" ]; then
            # ArcFS: copy via host temp to avoid FUSE same-mount deadlock
            mkdir -p "$mount_dir/.snap_v${snap}" 2>/dev/null || true
            timeout "$TIMEOUT_SEC" cp -a "$mount_dir/data" /tmp/_arcfs_snap_tmp/ 2>/dev/null || true
            timeout "$TIMEOUT_SEC" cp -a /tmp/_arcfs_snap_tmp/ "$mount_dir/.snap_v${snap}/data" 2>/dev/null || true
            rm -rf /tmp/_arcfs_snap_tmp
        else
            # ext4/bindfs: full copy (no CoW/snapshot support)
            cp -a "$mount_dir/data" "$mount_dir/snap_v${snap}"
        fi

        # Modify some files in the live data (simulate churn)
        local files_to_modify=$(( files_count * SNAPSHOT_CHURN_PCT / 100 ))
        if [ "$files_to_modify" -lt 1 ]; then files_to_modify=1; fi
        for i in $(seq 1 "$files_to_modify"); do
            dd if=/dev/urandom of="$mount_dir/data/file_${i}.dat" bs=1M count="$per_file_mb" 2>/dev/null
        done
        sync

        logical=$(measure_logical_bytes "$mount_dir")
        physical=$(measure_physical_bytes "$fs_type" "$mount_dir")
        physical=$(( physical - base_physical ))
        log_data "snapshot=$snap logical=$logical physical=$physical"
        steps=$(echo "$steps" | python3 -c "
import json, sys
s = json.load(sys.stdin)
s.append({'snapshot': $snap, 'logical_bytes': $logical, 'physical_bytes': $physical})
print(json.dumps(s))
")
    done

    save_result "snapshot" "$fs_type" "$(cat <<EOF
{
  "test": "snapshot",
  "filesystem": "$fs_type",
  "base_size_mb": $SNAPSHOT_BASE_SIZE_MB,
  "snapshot_count": $SNAPSHOT_COUNT,
  "churn_pct": $SNAPSHOT_CHURN_PCT,
  "steps": $steps
}
EOF
)"
}

# ==============================================================================
# Test D: Many Small Files (Metadata Overhead)
# ==============================================================================
run_smallfiles_test() {
    local fs_type="$1"
    local mount_dir="$2"

    log_info "=== SMALL FILES TEST on $fs_type ($mount_dir) ==="
    wipe_mount "$fs_type" "$mount_dir"

    local base_physical
    base_physical=$(measure_physical_bytes "$fs_type" "$mount_dir")

    log_step "Creating $SMALL_FILES_COUNT files of ${SMALL_FILES_SIZE_KB}KB each..."

    # Create files spread across subdirectories to simulate realistic hierarchy
    local dirs=("src" "lib" "test" "docs" "config" "build" "assets" "data" "logs" "tmp")
    for d in "${dirs[@]}"; do
        mkdir -p "$mount_dir/$d"
    done

    local total_written=0
    for i in $(seq 1 "$SMALL_FILES_COUNT"); do
        local dir_idx=$(( i % ${#dirs[@]} ))
        local target_dir="$mount_dir/${dirs[$dir_idx]}"
        dd if=/dev/urandom of="$target_dir/file_${i}.dat" bs=1K count="$SMALL_FILES_SIZE_KB" 2>/dev/null
        total_written=$(( total_written + SMALL_FILES_SIZE_KB * 1024 ))

        # Progress every 500 files
        if [ $(( i % 500 )) -eq 0 ]; then
            log_step "  Created $i / $SMALL_FILES_COUNT files..."
        fi
    done
    sync

    local logical physical
    logical=$(measure_logical_bytes "$mount_dir")
    physical=$(measure_physical_bytes "$fs_type" "$mount_dir")
    physical=$(( physical - base_physical ))

    local data_bytes=$total_written
    local overhead_bytes=$(( physical - data_bytes ))
    if [ "$overhead_bytes" -lt 0 ]; then overhead_bytes=0; fi

    log_data "files=$SMALL_FILES_COUNT logical=$logical physical=$physical data=$data_bytes overhead=$overhead_bytes"

    save_result "smallfiles" "$fs_type" "$(cat <<EOF
{
  "test": "smallfiles",
  "filesystem": "$fs_type",
  "file_count": $SMALL_FILES_COUNT,
  "file_size_kb": $SMALL_FILES_SIZE_KB,
  "logical_bytes": $logical,
  "physical_bytes": $physical,
  "data_bytes": $data_bytes,
  "overhead_bytes": $overhead_bytes
}
EOF
)"
}

# ==============================================================================
# Test E: Mixed Realistic Workload
# ==============================================================================
run_mixed_test() {
    local fs_type="$1"
    local mount_dir="$2"

    log_info "=== MIXED WORKLOAD TEST on $fs_type ($mount_dir) ==="
    wipe_mount "$fs_type" "$mount_dir"

    local base_physical
    base_physical=$(measure_physical_bytes "$fs_type" "$mount_dir")

    # Source code: highly duplicated small text-like files (compressible)
    log_step "Writing ${MIXED_SOURCE_MB}MB simulated source code (compressible, many small files)..."
    mkdir -p "$mount_dir/src"
    local src_files=$(( MIXED_SOURCE_MB * 1024 / 8 ))  # 8KB per file
    for i in $(seq 1 "$src_files"); do
        # Text-like compressible data (repeated pattern)
        python3 -c "print('// source code line\n' * 512)" > "$mount_dir/src/module_${i}.rs" 2>/dev/null
        if [ $(( i % 1000 )) -eq 0 ]; then
            log_step "  Source files: $i / $src_files"
        fi
    done

    # Logs: highly compressible (repeated patterns)
    log_step "Writing ${MIXED_LOG_MB}MB simulated log files (highly compressible)..."
    mkdir -p "$mount_dir/logs"
    generate_data_file "$mount_dir/logs/app.log" "$MIXED_LOG_MB" 85

    # Binary blobs: incompressible random data
    log_step "Writing ${MIXED_BINARY_MB}MB binary blobs (incompressible)..."
    mkdir -p "$mount_dir/bin"
    dd if=/dev/urandom of="$mount_dir/bin/artifact.bin" bs=1M count="$MIXED_BINARY_MB" 2>/dev/null

    sync

    local logical physical
    logical=$(measure_logical_bytes "$mount_dir")
    physical=$(measure_physical_bytes "$fs_type" "$mount_dir")
    physical=$(( physical - base_physical ))

    local total_input_mb=$(( MIXED_SOURCE_MB + MIXED_LOG_MB + MIXED_BINARY_MB ))

    log_data "logical=$logical physical=$physical input_mb=$total_input_mb"

    save_result "mixed" "$fs_type" "$(cat <<EOF
{
  "test": "mixed",
  "filesystem": "$fs_type",
  "input_mb": $total_input_mb,
  "source_mb": $MIXED_SOURCE_MB,
  "log_mb": $MIXED_LOG_MB,
  "binary_mb": $MIXED_BINARY_MB,
  "logical_bytes": $logical,
  "physical_bytes": $physical
}
EOF
)"
}

# ==============================================================================
# Main Orchestrator
# ==============================================================================

echo "========================================"
echo " ArcFS Disk Usage Benchmark Suite"
echo "========================================"
echo ""

# Parse filesystem selection
ALL_FS=("ext4" "btrfs" "bindfs" "arcfs")
if [ -n "${FS:-}" ]; then
    IFS=',' read -r -a FILESYSTEMS <<< "$FS"
    for f in "${FILESYSTEMS[@]}"; do
        if [[ ! " ${ALL_FS[*]} " =~ " ${f} " ]]; then
            log_error "Unknown filesystem '$f'. Allowed: ${ALL_FS[*]}"
            exit 1
        fi
    done
else
    FILESYSTEMS=("${ALL_FS[@]}")
fi

# Parse test selection
ALL_TESTS=("dedup" "compress" "snapshot" "smallfiles" "mixed")
if [ -n "${TESTS:-}" ]; then
    IFS=',' read -r -a SELECTED_TESTS <<< "$TESTS"
    for t in "${SELECTED_TESTS[@]}"; do
        if [[ ! " ${ALL_TESTS[*]} " =~ " ${t} " ]]; then
            log_error "Unknown test '$t'. Allowed: ${ALL_TESTS[*]}"
            exit 1
        fi
    done
else
    SELECTED_TESTS=("${ALL_TESTS[@]}")
fi

log_info "Filesystems: ${FILESYSTEMS[*]}"
log_info "Tests: ${SELECTED_TESTS[*]}"
log_info "Results dir: $RESULTS_DIR"
echo ""

# Preflight checks
if [ ! -d "$ARENA_DIR" ]; then
    log_error "Benchmark arena not found at $ARENA_DIR"
    log_error "Run benchmarks/setup_arena.sh first"
    exit 1
fi

mkdir -p "$RESULTS_DIR"

# Check mount availability
for fs in "${FILESYSTEMS[@]}"; do
    case "$fs" in
        ext4)
            if ! mountpoint -q "$EXT4_MOUNT" 2>/dev/null; then
                log_error "ext4 not mounted at $EXT4_MOUNT. Run setup_arena.sh."
                exit 1
            fi
            ;;
        btrfs)
            if ! mountpoint -q "$BTRFS_MOUNT" 2>/dev/null; then
                log_error "btrfs not mounted at $BTRFS_MOUNT. Run setup_arena.sh."
                exit 1
            fi
            ;;
        bindfs)
            if ! mountpoint -q "$BINDFS_MOUNT" 2>/dev/null; then
                log_warn "bindfs not mounted. Attempting to mount..."
                mkdir -p "$BINDFS_MOUNT"
                if mountpoint -q "$EXT4_MOUNT" 2>/dev/null; then
                    sudo bindfs "$EXT4_MOUNT" "$BINDFS_MOUNT"
                    log_info "bindfs mounted successfully."
                else
                    log_error "Cannot mount bindfs: ext4 backing not available."
                    exit 1
                fi
            fi
            ;;
        arcfs)
            if ! mountpoint -q "$ARCFS_MOUNT" 2>/dev/null; then
                log_error "ArcFS not mounted at $ARCFS_MOUNT."
                log_error "Start the daemon: cargo run --release -- --storage-dir $ARCFS_BACKEND mount $ARCFS_MOUNT"
                exit 1
            fi
            ;;
    esac
    log_info "$fs mount verified."
done

echo ""
log_info "Starting benchmark suite..."
SUITE_START=$(date +%s)

# Run each test on each filesystem
for test_name in "${SELECTED_TESTS[@]}"; do
    TEST_START=$(date +%s)
    echo ""
    echo "========================================"
    log_info "TEST: $test_name"
    echo "========================================"

    for fs in "${FILESYSTEMS[@]}"; do
        FS_START=$(date +%s)

        local_mount=""
        case "$fs" in
            ext4)   local_mount="$EXT4_MOUNT"   ;;
            btrfs)  local_mount="$BTRFS_MOUNT"  ;;
            bindfs) local_mount="$BINDFS_MOUNT" ;;
            arcfs)  local_mount="$ARCFS_MOUNT"  ;;
        esac

        case "$test_name" in
            dedup)      run_dedup_test "$fs" "$local_mount"      ;;
            compress)   run_compress_test "$fs" "$local_mount"   ;;
            snapshot)   run_snapshot_test "$fs" "$local_mount"   ;;
            smallfiles) run_smallfiles_test "$fs" "$local_mount" ;;
            mixed)      run_mixed_test "$fs" "$local_mount"      ;;
        esac

        FS_END=$(date +%s)
        log_info "$test_name on $fs completed in $(( FS_END - FS_START ))s"
    done

    TEST_END=$(date +%s)
    log_info "Test '$test_name' total: $(( TEST_END - TEST_START ))s"
done

SUITE_END=$(date +%s)
echo ""
echo "========================================"
log_info "All disk usage benchmarks completed in $(( SUITE_END - SUITE_START ))s"
log_info "Results saved to: $RESULTS_DIR"
echo "========================================"

# Print summary table
echo ""
log_info "Result files:"
ls -la "$RESULTS_DIR"/*.json 2>/dev/null | while read -r line; do
    echo "  $line"
done
