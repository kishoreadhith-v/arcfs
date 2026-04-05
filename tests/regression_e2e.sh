#!/usr/bin/env bash
set -u

TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-8}"
MOUNT_DIR="${MOUNT_DIR:-mnt}"
STORAGE_DIR="${STORAGE_DIR:-my_storage}"
BIN_PATH="${BIN_PATH:-target/debug/arcfs}"
LOG_FILE="${LOG_FILE:-/tmp/arcfs-regression.log}"

PASS_COUNT=0
FAIL_COUNT=0
FS_PID=""

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

pass() {
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "${GREEN}[PASS]${NC} $*"
}

fail() {
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo -e "${RED}[FAIL]${NC} $*"
}

run_sh() {
    local cmd="$1"
    timeout "${TIMEOUT_SECONDS}" bash -lc "$cmd"
}

must() {
    local description="$1"
    local cmd="$2"

    if run_sh "$cmd" >/dev/null 2>&1; then
        pass "$description"
        return 0
    fi

    fail "$description"
    echo "      cmd: $cmd"
    run_sh "$cmd" >/tmp/arcfs_cmd_stdout.log 2>/tmp/arcfs_cmd_stderr.log || true
    if [[ -s /tmp/arcfs_cmd_stderr.log ]]; then
        sed 's/^/      stderr: /' /tmp/arcfs_cmd_stderr.log
    fi
    if [[ -s /tmp/arcfs_cmd_stdout.log ]]; then
        sed 's/^/      stdout: /' /tmp/arcfs_cmd_stdout.log
    fi
    return 1
}

assert_equals() {
    local description="$1"
    local expected="$2"
    local actual="$3"

    if [[ "$expected" == "$actual" ]]; then
        pass "$description"
    else
        fail "$description (expected='$expected' actual='$actual')"
        return 1
    fi
}

assert_file_content() {
    local description="$1"
    local file_path="$2"
    local expected="$3"

    local output
    if ! output="$(run_sh "cat '$file_path'" 2>/dev/null)"; then
        fail "$description (cannot read $file_path)"
        return 1
    fi

    assert_equals "$description" "$expected" "$output"
}

assert_snapshot_readonly() {
    local description="$1"
    local file_path="$2"

    if run_sh "(echo 'mutate' > '$file_path')" >/tmp/arcfs_ro_out.log 2>/tmp/arcfs_ro_err.log; then
        fail "$description (write unexpectedly succeeded)"
        return 1
    fi

    if grep -qi "read-only file system\|permission denied" /tmp/arcfs_ro_err.log; then
        pass "$description"
        return 0
    fi

    fail "$description (write failed but not due to read-only semantics)"
    sed 's/^/      stderr: /' /tmp/arcfs_ro_err.log
    return 1
}

start_fs() {
    info "Starting ArcFS mount"

    pkill -x arcfs >/dev/null 2>&1 || true
    pkill -f "target/debug/arcfs mount ${MOUNT_DIR}" >/dev/null 2>&1 || true
    fusermount -u "${MOUNT_DIR}" >/dev/null 2>&1 || true
    mkdir -p "${MOUNT_DIR}"

    "${BIN_PATH}" mount "${MOUNT_DIR}" >"${LOG_FILE}" 2>&1 &
    FS_PID=$!

    for _ in $(seq 1 40); do
        if mountpoint -q "${MOUNT_DIR}"; then
            pass "Filesystem mounted"
            return 0
        fi
        sleep 0.2
    done

    fail "Filesystem mount timeout"
    return 1
}

stop_fs() {
    if mountpoint -q "${MOUNT_DIR}"; then
        fusermount -u "${MOUNT_DIR}" >/dev/null 2>&1 || true
    fi

    if [[ -n "$FS_PID" ]] && kill -0 "$FS_PID" >/dev/null 2>&1; then
        kill "$FS_PID" >/dev/null 2>&1 || true
        wait "$FS_PID" >/dev/null 2>&1 || true
    fi

    pkill -x arcfs >/dev/null 2>&1 || true
    pkill -f "target/debug/arcfs mount ${MOUNT_DIR}" >/dev/null 2>&1 || true
    sleep 1
    FS_PID=""
}

cleanup() {
    stop_fs
}

trap cleanup EXIT

main() {
    info "Building project"
    if ! must "cargo check passes" "cargo check -q"; then
        report
        exit 1
    fi

    if ! must "cargo build passes" "cargo build -q"; then
        report
        exit 1
    fi

    info "Resetting storage for deterministic run"
    rm -rf "${STORAGE_DIR}"

    if ! start_fs; then
        report
        exit 1
    fi

    info "Running runtime feature matrix"

    must "root is accessible" "ls -la '${MOUNT_DIR}'"
    must "write regular file" "echo 'v1' > '${MOUNT_DIR}/alpha.txt'"
    assert_file_content "read regular file" "${MOUNT_DIR}/alpha.txt" "v1"

    must "CAS shard directories exist" "find '${STORAGE_DIR}/cas' -mindepth 1 -maxdepth 1 -type d | grep -Eq '/[0-9a-f]{2}$'"
    must "CAS chunk files exist" "find '${STORAGE_DIR}/cas' -mindepth 2 -type f | head -n 1 | grep -q ."

    must "overwrite regular file" "echo 'v2' > '${MOUNT_DIR}/alpha.txt'"
    assert_file_content "read overwritten file" "${MOUNT_DIR}/alpha.txt" "v2"

    must "truncate file with setattr path" "truncate -s 1 '${MOUNT_DIR}/alpha.txt'"
    assert_file_content "read truncated file" "${MOUNT_DIR}/alpha.txt" "v"

    must "create nested directory" "mkdir -p '${MOUNT_DIR}/docs/sub'"
    must "create nested file" "echo 'report-v1' > '${MOUNT_DIR}/docs/sub/report.txt'"
    assert_file_content "read nested file" "${MOUNT_DIR}/docs/sub/report.txt" "report-v1"

    must "@tags virtual root is visible" "ls -la '${MOUNT_DIR}/@tags'"
    assert_file_content "tag path resolves file" "${MOUNT_DIR}/@tags/docs/sub/report.txt" "report-v1"
    assert_file_content "tag permutation resolves file" "${MOUNT_DIR}/@tags/sub/docs/report.txt" "report-v1"
    must "write through tag path" "echo 'report-v1-tagwrite' > '${MOUNT_DIR}/@tags/docs/sub/report.txt'"
    assert_file_content "live file reflects tag write" "${MOUNT_DIR}/docs/sub/report.txt" "report-v1-tagwrite"
    must "create file through tag path" "echo 'new-via-tag' > '${MOUNT_DIR}/@tags/sub/docs/new_via_tag.txt'"
    assert_file_content "created tag file visible in live tree" "${MOUNT_DIR}/docs/sub/new_via_tag.txt" "new-via-tag"

    must "snapshot create via .create" "echo 'snap1' > '${MOUNT_DIR}/.snapshots/.create'"
    must "snapshot listed after .create" "ls -1 '${MOUNT_DIR}/.snapshots' | grep -x 'snap1'"

    must "modify live file after snapshot" "echo 'report-v2' > '${MOUNT_DIR}/docs/sub/report.txt'"
    assert_file_content "live file moved forward" "${MOUNT_DIR}/docs/sub/report.txt" "report-v2"
    assert_file_content "tag path reflects live updates" "${MOUNT_DIR}/@tags/docs/sub/report.txt" "report-v2"
    assert_file_content "snapshot file preserved" "${MOUNT_DIR}/.snapshots/snap1/docs/sub/report.txt" "report-v1-tagwrite"

    assert_snapshot_readonly "snapshot path is read-only" "${MOUNT_DIR}/.snapshots/snap1/docs/sub/report.txt"

    must "snapshot create via legacy mkdir trigger" "mkdir '${MOUNT_DIR}/.snap_snap2'"
    must "legacy snapshot appears in listing" "ls -1 '${MOUNT_DIR}/.snapshots' | grep -x 'snap2'"

    must "delete snapshot by rmdir" "rmdir '${MOUNT_DIR}/.snapshots/snap2'"
    must "snapshot removed from listing" "! ls -1 '${MOUNT_DIR}/.snapshots' | grep -x 'snap2'"

    must "restore workflow trigger" "mkdir '${MOUNT_DIR}/.restore_snap1'"
    sleep 2
    assert_file_content "restore brings live file back to snapshot state" "${MOUNT_DIR}/docs/sub/report.txt" "report-v1-tagwrite"
    must "auto-backup snapshot created on restore" "ls -1 '${MOUNT_DIR}/.snapshots' | grep '^before_restore_'"

    info "Testing persistence across remount"
    stop_fs
    if ! start_fs; then
        report
        exit 1
    fi

    assert_file_content "live file persisted after remount" "${MOUNT_DIR}/docs/sub/report.txt" "report-v1-tagwrite"
    must "snapshot persisted after remount" "ls -1 '${MOUNT_DIR}/.snapshots' | grep -x 'snap1'"
    assert_file_content "snapshot content persisted after remount" "${MOUNT_DIR}/.snapshots/snap1/docs/sub/report.txt" "report-v1-tagwrite"

    info "Running metadata/maintenance checks"
    stop_fs
    pkill -x arcfs >/dev/null 2>&1 || true
    sleep 1

    must "inspect command succeeds" "'${BIN_PATH}' inspect > /tmp/arcfs_inspect.out"
    must "metadata includes ino_meta namespace" "grep -q 'ino_meta:' /tmp/arcfs_inspect.out"
    must "metadata includes ino_recipe namespace" "grep -q 'ino_recipe:' /tmp/arcfs_inspect.out"
    must "metadata includes dirent namespace" "grep -q 'dirent:' /tmp/arcfs_inspect.out"
    must "gc command executes" "'${BIN_PATH}' gc >/tmp/arcfs_gc.out"

    report

    if [[ "$FAIL_COUNT" -gt 0 ]]; then
        exit 1
    fi
}

report() {
    echo
    echo "========================================"
    echo "ArcFS Regression E2E Report"
    echo "========================================"
    echo "Pass: ${PASS_COUNT}"
    echo "Fail: ${FAIL_COUNT}"
    echo "Log:  ${LOG_FILE}"

    if [[ "$FAIL_COUNT" -eq 0 ]]; then
        echo -e "${GREEN}RESULT: PASS${NC}"
    else
        echo -e "${RED}RESULT: FAIL${NC}"
    fi
}

main "$@"
