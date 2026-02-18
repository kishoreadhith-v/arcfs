# Terminal 1: Mount
cargo run -- mount mnt

# Terminal 2: Test CoW refinements
echo "Version 1" > mnt/file.txt
mkdir mnt/docs
echo "Data" > mnt/docs/report.txt

# Take snapshot
mkdir mnt/.snap_v1
# Output: [CHRONOS] Taking Snapshot: v1
#         [GC] Root Inode ref_count: 2

# Test CoW on write
echo "Version 2" > mnt/file.txt
# Output: [WRITE] Request to modify 'file.txt'
#         [CoW] Node 'file.txt' is shared! Cloning...

# Test CoW on create (NEW)
touch mnt/newfile.txt
# Output: [CREATE] Ensuring parent '/' is mutable

# Test CoW on nested mkdir (NEW)
mkdir mnt/docs/subdir
# Output: [MKDIR] Ensuring parent 'docs' is mutable
#         [CoW] Node 'docs' is shared! Cloning...

# Verify snapshot isolation
cat mnt/file.txt                    # "Version 2"
cat mnt/.snapshots/v1/file.txt     # "Version 1"
ls mnt/                             # Shows newfile.txt
ls mnt/.snapshots/v1/               # Doesn't show newfile.txt

# Test read-only snapshot (should fail)
echo "hack" > mnt/.snapshots/v1/file.txt  # Permission denied
