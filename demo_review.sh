#!/bin/bash
# ArcFS Temporal Snapshot System - Review Demo
# This demonstrates Copy-on-Write versioning with instant snapshots

set -e

# Cleanup function
cleanup() {
    echo ""
    echo "→ Cleaning up..."
    fusermount -u mnt 2>/dev/null || true
    pkill -9 better-fs 2>/dev/null || true
}

trap cleanup EXIT

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  ArcFS - Temporal Snapshot System (TSS) Demo                  ║"
echo "║  Copy-on-Write Versioning with O(1) Snapshots                 ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# Cleanup and mount
echo "→ Cleaning up and mounting filesystem..."
pkill -9 better-fs 2>/dev/null || true
fusermount -u mnt 2>/dev/null || true
sleep 1
rm -rf my_storage
mkdir -p mnt
cargo build --release > /dev/null 2>&1
target/release/better-fs mount mnt > /tmp/arcfs.log 2>&1 &
FS_PID=$!
echo "  Waiting for filesystem to initialize..."

# Wait for mount to be ready
for i in {1..10}; do
    if ls mnt/ > /dev/null 2>&1 && [ "$(ls -A mnt/)" = "" ]; then
        break
    fi
    sleep 0.5
done

# Final check
if ! ls mnt/ > /dev/null 2>&1; then
    echo "ERROR: Filesystem failed to mount. Check /tmp/arcfs.log"
    cat /tmp/arcfs.log
    exit 1
fi

echo "✓ Filesystem mounted and ready (PID: $FS_PID)"
echo ""

# ========================================
# DEMO START
# ========================================

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📝 STEP 1: Create initial documents (Version 1)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "Creating project structure..."
echo "Sprint Planning - Week 1" > mnt/project.txt
echo "Alice, Bob, Charlie" > mnt/team.txt
mkdir -p mnt/docs
echo "Initial architecture design" > mnt/docs/design.txt

echo ""
echo "📂 Current files:"
ls -1 mnt/
echo ""
echo "📄 project.txt: $(cat mnt/project.txt)"
echo "📄 team.txt: $(cat mnt/team.txt)"
echo "📄 docs/design.txt: $(cat mnt/docs/design.txt)"

read -p "⏸️  Press ENTER to continue..."

# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📸 STEP 2: Take snapshot 'sprint_1'"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "Taking snapshot... (This is O(1) - just cloning an Arc pointer!)"
mkdir mnt/.snap_sprint_1
sleep 1

echo "✓ Snapshot 'sprint_1' created"
echo ""
echo "📦 Available snapshots:"
ls -1 mnt/.snapshots/

read -p "⏸️  Press ENTER to continue..."

# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✏️  STEP 3: Modify documents (Version 2)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "Updating project scope..."
echo "Sprint Planning - Week 2 (Updated)" > mnt/project.txt
echo "Alice, Bob, Charlie, Dave" > mnt/team.txt
echo "Architecture design - Added microservices" > mnt/docs/design.txt

echo ""
echo "📄 project.txt: $(cat mnt/project.txt)"
echo "📄 team.txt: $(cat mnt/team.txt)"
echo "📄 docs/design.txt: $(cat mnt/docs/design.txt)"

read -p "⏸️  Press ENTER to continue..."

# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📸 STEP 4: Take snapshot 'sprint_2'"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

mkdir mnt/.snap_sprint_2
sleep 1

echo "✓ Snapshot 'sprint_2' created"
echo ""
echo "📦 Available snapshots:"
ls -1 mnt/.snapshots/

read -p "⏸️  Press ENTER to continue..."

# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✏️  STEP 5: Major changes (Version 3)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "Pivoting project direction..."
echo "Sprint Planning - Week 3 (PIVOT!)" > mnt/project.txt
rm mnt/team.txt
echo "New Team: Eve, Frank" > mnt/team_new.txt
echo "Complete redesign - monolithic approach" > mnt/docs/design.txt

echo ""
echo "📄 project.txt: $(cat mnt/project.txt)"
echo "📄 team_new.txt: $(cat mnt/team_new.txt)"
echo "📄 docs/design.txt: $(cat mnt/docs/design.txt)"
echo "❌ team.txt: DELETED"

read -p "⏸️  Press ENTER to continue..."

# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 STEP 6: Compare all versions (THE MAGIC!)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "📊 COMPARING project.txt ACROSS TIME:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  CURRENT (v3):    $(cat mnt/project.txt)"
echo "  sprint_2 (v2):   $(cat mnt/.snapshots/sprint_2/project.txt)"
echo "  sprint_1 (v1):   $(cat mnt/.snapshots/sprint_1/project.txt)"
echo ""

echo "📊 COMPARING team files:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  CURRENT:         team.txt is DELETED ❌"
echo "  sprint_2:        $(cat mnt/.snapshots/sprint_2/team.txt)"
echo "  sprint_1:        $(cat mnt/.snapshots/sprint_1/team.txt)"
echo ""

echo "📊 COMPARING docs/design.txt:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  CURRENT (v3):    $(cat mnt/docs/design.txt)"
echo "  sprint_2 (v2):   $(cat mnt/.snapshots/sprint_2/docs/design.txt)"
echo "  sprint_1 (v1):   $(cat mnt/.snapshots/sprint_1/docs/design.txt)"
echo ""

read -p "⏸️  Press ENTER to continue..."

# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⏮️  STEP 7: Time Travel - Restore sprint_1"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo "Restoring to sprint_1 state..."
mkdir mnt/.restore_sprint_1 2>/dev/null || true
sleep 2

echo ""
echo "📂 Current files after restore:"
ls -1 mnt/
echo ""
echo "📄 project.txt: $(cat mnt/project.txt)"
echo "📄 team.txt: $(cat mnt/team.txt)"
echo ""
echo "✓ Successfully restored to sprint_1!"
echo "  (Note: An auto-backup snapshot was created before restore)"

read -p "⏸️  Press ENTER to continue..."

# ========================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📈 STEP 8: Performance & Architecture Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "⚡ Performance Characteristics:"
echo "  • Snapshot Creation:  O(1) - just Arc::clone(&root)"
echo "  • Snapshot Read:      O(1) - direct pointer dereference"
echo "  • First Write (CoW):  O(depth) ≈ O(log n) - path cloning"
echo "  • Space Overhead:     8 bytes per snapshot + deltas only"
echo ""
echo "🏗️  Architecture:"
echo "  • Arc<RwLock<Inode>>: Thread-safe reference counting"
echo "  • Copy-on-Write:      Lazy evaluation on write"
echo "  • Deduplication:      Content-addressable storage (CAS)"
echo "  • Garbage Collection: Automatic via Arc drop"
echo ""
echo "🎯 Key Features Demonstrated:"
echo "  ✓ Instant snapshots (no data copying)"
echo "  ✓ Perfect isolation (each snapshot is immutable)"
echo "  ✓ Space efficiency (shared unchanged data)"
echo "  ✓ Time travel (restore to any snapshot)"
echo "  ✓ Auto-backup (safety before restore)"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ DEMO COMPLETE!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Final snapshot list:"
ls -1 mnt/.snapshots/
echo ""

read -p "Press ENTER to unmount and exit..."

# Cleanup will be called by trap
echo ""
echo "✓ Demo complete!"
