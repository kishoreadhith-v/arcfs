#!/bin/bash
# ArcFS Time Travel Demo - Automated Version
# Demonstrates Copy-on-Write snapshots with historical preservation

set -e

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║          ArcFS Temporal Snapshot System Demo                  ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# Cleanup and mount
echo "→ Step 1: Mounting filesystem"
pkill -9 arcfs 2>/dev/null || true
fusermount -u mnt 2>/dev/null || true
rm -rf my_storage
mkdir -p mnt
cargo build --release > /dev/null 2>&1
target/release/arcfs mount mnt > /tmp/arcfs.log 2>&1 &
FS_PID=$!

# Wait for mount
for i in {1..10}; do
    if mountpoint -q mnt 2>/dev/null; then
        break
    fi
    sleep 0.5
done

echo "✓ Mounted (PID: $FS_PID)"
echo ""

# Step 2: Create V1
echo "→ Step 2: Creating Version 1"
echo "Document V1 - Initial draft" > mnt/document.txt
cat mnt/document.txt
echo ""

# Step 3: Snapshot V1
echo "→ Step 3: Taking snapshot 'v1'"
echo "snap_v1" > mnt/.snapshots/.create
ls mnt/.snapshots/
echo ""

# Step 4: Modify to V2
echo "→ Step 4: Modifying document (V2)"
echo "Document V2 - Added intro section" > mnt/document.txt
cat mnt/document.txt
echo ""

# Step 5: Snapshot V2
echo "→ Step 5: Taking snapshot 'v2'"
echo "snap_v2" > mnt/.snapshots/.create
ls mnt/.snapshots/
echo ""

# Step 6: Modify to V3
echo "→ Step 6: Modifying document (V3)"
echo "Document V3 - Final version with conclusion" > mnt/document.txt
cat mnt/document.txt
echo ""

# Step 7: Verify all versions
echo "→ Step 7: Reading all versions simultaneously"
echo "  Current (V3):"
cat mnt/document.txt | sed 's/^/    /'
echo ""
echo "  Snapshot v2:"
cat mnt/.snapshots/snap_v2/document.txt | sed 's/^/    /'
echo ""
echo "  Snapshot v1:"
cat mnt/.snapshots/snap_v1/document.txt | sed 's/^/    /'
echo ""

echo "✓ Demo complete! All 3 versions preserved independently."
echo ""
echo "→ Cleaning up..."
pkill -9 arcfs
fusermount -u mnt 2>/dev/null || true
echo "✓ Done"
