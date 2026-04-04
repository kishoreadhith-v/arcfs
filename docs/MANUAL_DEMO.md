# ArcFS Time Travel Demo - Manual Commands

**IMPORTANT**: You need TWO terminals open at the same time!

---

## Setup (Terminal 1 - Keep this running!)

```bash
# Clean up any existing mounts
pkill -9 better-fs 2>/dev/null || true
fusermount -u mnt 2>/dev/null || true
rm -rf my_storage
mkdir -p mnt

# Build and mount
cargo build --release
target/release/better-fs mount mnt
```

**DO NOT CLOSE THIS TERMINAL OR PRESS CTRL+C UNTIL DEMO IS DONE!**

The terminal will show "Mounting BetterFS to mnt..." and stay running.

Now open a **NEW** terminal (Terminal 2) for the demo commands below.

---

## Demo Commands (Terminal 2)

```bash
cd ~/better-fs

# Wait for mount to be ready
sleep 2

# Step 1: Create initial version
echo "Document V1 - Initial draft" > mnt/document.txt
cat mnt/document.txt

# Step 2: Take snapshot v1
echo "snap_v1" > mnt/.snapshots/.create
ls mnt/.snapshots/

# Step 3: Modify to v2
echo "Document V2 - Added intro section" > mnt/document.txt
cat mnt/document.txt

# Step 4: Take snapshot v2
echo "snap_v2" > mnt/.snapshots/.create
ls mnt/.snapshots/

# Step 5: Modify to v3
echo "Document V3 - Final version with conclusion" > mnt/document.txt
cat mnt/document.txt

# Step 6: Verify time travel - Read all 3 versions!
echo "=== Current Version ==="
cat mnt/document.txt

echo "=== Snapshot V2 ==="
cat mnt/.snapshots/snap_v2/document.txt

echo "=== Snapshot V1 ==="
cat mnt/.snapshots/snap_v1/document.txt
```

---

## Cleanup

```bash
# In Terminal 1 (where filesystem is running)
# Press Ctrl+C

# Then run:
fusermount -u mnt
```

---

## Key Points to Highlight

- **O(1) Snapshots**: Creating snapshots is instant (just write a name)
- **Copy-on-Write**: Modifications after snapshot don't affect historical versions
- **Zero Duplication**: All versions share unchanged data chunks in CAS
- **Independent Access**: Read all versions simultaneously without conflicts
