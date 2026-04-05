#!/usr/bin/env bash
set -euo pipefail

BIN="target/debug/arcfs"
MOUNT_DIR="mnt"
STORAGE_DIR="my_storage"
LOG_FILE="/tmp/arcfs-verify-single-backing.log"
FS_PID=""

cleanup() {
  if mountpoint -q "${MOUNT_DIR}"; then
    fusermount -u "${MOUNT_DIR}" >/dev/null 2>&1 || true
  fi

  if [[ -n "${FS_PID}" ]] && kill -0 "${FS_PID}" >/dev/null 2>&1; then
    kill "${FS_PID}" >/dev/null 2>&1 || true
    wait "${FS_PID}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "[1/7] Building binary"
cargo build -q

echo "[2/7] Resetting storage + mount dir"
rm -rf "${STORAGE_DIR}"
mkdir -p "${MOUNT_DIR}"

if mountpoint -q "${MOUNT_DIR}"; then
  fusermount -u "${MOUNT_DIR}" >/dev/null 2>&1 || true
fi

echo "[3/7] Mounting ArcFS"
"${BIN}" mount "${MOUNT_DIR}" >"${LOG_FILE}" 2>&1 &
FS_PID=$!

for _ in $(seq 1 40); do
  if mountpoint -q "${MOUNT_DIR}"; then
    break
  fi
  sleep 0.2
done

if ! mountpoint -q "${MOUNT_DIR}"; then
  echo "[FAIL] Mount timeout. See ${LOG_FILE}"
  exit 1
fi

echo "[4/7] Creating file in live path"
mkdir -p "${MOUNT_DIR}/docs/sub"
echo "report-v1" > "${MOUNT_DIR}/docs/sub/report.txt"

echo "[5/7] Verifying tag permutations + write-through"
TAG_A="$(cat "${MOUNT_DIR}/@tags/docs/sub/report.txt")"
TAG_B="$(cat "${MOUNT_DIR}/@tags/sub/docs/report.txt")"

echo "    @tags/docs/sub/report.txt => ${TAG_A}"
echo "    @tags/sub/docs/report.txt => ${TAG_B}"

if [[ "${TAG_A}" != "report-v1" || "${TAG_B}" != "report-v1" ]]; then
  echo "[FAIL] Tag permutations did not resolve to expected content"
  exit 1
fi

echo "via-tag" > "${MOUNT_DIR}/@tags/sub/docs/report.txt"
LIVE_AFTER_WRITE="$(cat "${MOUNT_DIR}/docs/sub/report.txt")"
echo "    live docs/sub/report.txt after tag write => ${LIVE_AFTER_WRITE}"

if [[ "${LIVE_AFTER_WRITE}" != "via-tag" ]]; then
  echo "[FAIL] Write through tag path did not hit live backing file"
  exit 1
fi

echo "[6/7] Optional create through tag path"
echo "new-via-tag" > "${MOUNT_DIR}/@tags/docs/sub/new_via_tag.txt"
NEW_LIVE="$(cat "${MOUNT_DIR}/docs/sub/new_via_tag.txt")"
echo "    live docs/sub/new_via_tag.txt => ${NEW_LIVE}"

if [[ "${NEW_LIVE}" != "new-via-tag" ]]; then
  echo "[FAIL] Create through tag path did not materialize in live tree"
  exit 1
fi

echo "[7/7] Unmounting and proving single-backing metadata"
cleanup
FS_PID=""

INSPECT_OUT="$("${BIN}" inspect)"
REPORT_DIRENT_COUNT="$(echo "${INSPECT_OUT}" | grep -E 'dirent:.*:report\.txt' | wc -l | tr -d ' ')"
RECIPE_COUNT="$(echo "${INSPECT_OUT}" | grep -E 'ino_recipe:' | wc -l | tr -d ' ')"
CAS_FILES="$(find "${STORAGE_DIR}/cas" -type f 2>/dev/null | wc -l | tr -d ' ')"

echo "    dirent entries ending in report.txt: ${REPORT_DIRENT_COUNT}"
echo "    total ino_recipe entries: ${RECIPE_COUNT}"
echo "    total CAS chunk files: ${CAS_FILES}"

if [[ "${REPORT_DIRENT_COUNT}" != "1" ]]; then
  echo "[FAIL] Expected exactly 1 live dirent for report.txt, got ${REPORT_DIRENT_COUNT}"
  exit 1
fi

if [[ "${CAS_FILES}" -lt 1 ]]; then
  echo "[FAIL] No CAS chunk files found"
  exit 1
fi

echo "[PASS] Tag paths are virtual views; backing file is single live entry with shared content."