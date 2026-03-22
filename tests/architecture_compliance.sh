#!/usr/bin/env bash
set -u

PASS_COUNT=0
FAIL_COUNT=0

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  echo -e "${GREEN}[PASS]${NC} $*"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  echo -e "${RED}[FAIL]${NC} $*"
}

must_grep() {
  local desc="$1"
  local pattern="$2"
  local file="$3"

  if grep -Eq "$pattern" "$file"; then
    pass "$desc"
  else
    fail "$desc"
  fi
}

must_not_grep() {
  local desc="$1"
  local pattern="$2"
  local file="$3"

  if grep -Eq "$pattern" "$file"; then
    fail "$desc"
  else
    pass "$desc"
  fi
}

echo -e "${BLUE}[INFO]${NC} Running architecture compliance checks"

# Section 2.2 relational metadata prefixes
must_grep "inode metadata key prefix is ino_meta" 'ino_meta:' src/file_manager.rs
must_grep "inode recipe key prefix is ino_recipe" 'ino_recipe:' src/file_manager.rs
must_grep "dirent key prefix is dirent" 'dirent:' src/file_manager.rs

# bincode serialization/deserialization in metadata engine
must_grep "bincode serialize used in metadata engine" 'bincode::serialize' src/file_manager.rs
must_grep "bincode deserialize used in metadata engine" 'bincode::deserialize' src/file_manager.rs

# avoid unwrap on sled db operations in metadata engine
must_not_grep "no unwrap directly on sled db operations" 'db\.[a-zA-Z_]+\([^\)]*\)\.unwrap\(' src/file_manager.rs

# CAS requirements
must_grep "CAS path sharding by 2-char prefix" 'join\(&hash\[0\.\.2\]\)' src/storage.rs
must_grep "zstd compression on write" 'zstd::encode_all' src/storage.rs
must_grep "zstd decompression on read" 'zstd::decode_all' src/storage.rs

# Snapshot behavior wiring
must_grep "snapshot create control file exists" '\.create' src/fuse_handler.rs
must_grep "snapshot read-only enforced with EROFS" 'EROFS' src/fuse_handler.rs
must_grep "startup uses dirent edges for hydrate" 'list_dirents' src/fuse_handler.rs

# Architectural MUST checks likely still pending
must_grep "FastCDC gear hash implementation present" 'GEAR|gear' src/chunker.rs
must_grep "write-back page cache present" 'page_cache|HashMap<u64, \(Vec<u8>, bool\)>' src/fuse_handler.rs

echo ""
echo "========================================"
echo "Architecture Compliance Report"
echo "========================================"
echo "Pass: ${PASS_COUNT}"
echo "Fail: ${FAIL_COUNT}"

if [[ "$FAIL_COUNT" -eq 0 ]]; then
  echo -e "${GREEN}RESULT: PASS${NC}"
  exit 0
fi

echo -e "${RED}RESULT: FAIL${NC}"
exit 1
