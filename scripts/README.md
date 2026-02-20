#!/bin/bash
# ArcFS Demo Scripts README
# 
# This directory contains demonstration scripts for ArcFS features.
# Each script is self-contained and educational for new users.

# ============================================================
# QUICK START
# ============================================================

# To understand ArcFS capabilities, run these in order:
#
# 1. START HERE - Learn the concepts:
#    bash scripts/demo_tagfs.sh
#
# 2. See time-travel in action:
#    bash scripts/snapshot_demo.sh
#
# 3. Understand deduplication:
#    bash scripts/demo_versioning.sh
#
# 4. Run comprehensive tests:
#    cargo test --test tagfs_test -- --nocapture

# ============================================================
# SCRIPT DESCRIPTIONS
# ============================================================

# demo_tagfs.sh
# ─────────────────────────────────────────────────────────
# Status: ✅ RECOMMENDED FOR NEW USERS
# Phase: 3 (TagFS - Semantic Tagging)
# 
# Demonstrates:
#   • Automatic tag generation from directory hierarchy
#   • Order-independent file access via tag permutations
#   • Tag-based file discovery and queries
#   • Real-world project organization with tags
#   • Integration with FUSE filesystem
#
# Key Concept:
#   Files at /projects/backend/2026/api.rs are auto-tagged
#   with [projects, backend, 2026] and can be accessed as:
#   • /projects/backend/2026/api.rs (original)
#   • /backend/2026/projects/api.rs (permutation)
#   • /2026/projects/backend/api.rs (permutation)
#   ... and 3 more permutations!
#
# Running Time: ~3-5 seconds
# Output: Highly educational with example queries and use cases
#
# Usage:
#   bash scripts/demo_tagfs.sh
#

# snapshot_demo.sh
# ─────────────────────────────────────────────────────────
# Status: ✅ STABLE
# Phase: 2 (Chronos - Time Travel)
#
# Demonstrates:
#   • Creating point-in-time snapshots
#   • Restoring filesystem state from snapshots
#   • Comparing file versions across time
#   • Copy-on-Write (CoW) divergence behavior
#   • Virtual .snapshots/ directory for browsing
#
# Key Concept:
#   Take instant O(1) snapshots of the filesystem.
#   Make changes. Restore to any previous state.
#   No data copying until writes trigger CoW.
#
# Running Time: ~3-5 seconds
# Output: Shows before/after states and version comparisons
#
# Usage:
#   bash scripts/snapshot_demo.sh
#

# demo_versioning.sh
# ─────────────────────────────────────────────────────────
# Status: ✅ STABLE
# Phase: 1 (Core Engine - Content-defined Chunking)
#
# Demonstrates:
#   • Content-defined chunking algorithm
#   • Automatic deduplication via SHA256 hashing
#   • Storage savings from duplicate data
#   • Content-Addressed Storage (CAS) organization
#   • Rolling hash boundary detection
#
# Key Concept:
#   Files are split into variable-sized chunks based
#   on content patterns. Identical chunks stored once.
#   Example: 2 identical 50KB files → 50KB stored total
#
# Running Time: ~2-3 seconds
# Output: Storage statistics and deduplication metrics
#
# Usage:
#   bash scripts/demo_versioning.sh
#

# test_phase2.sh
# ─────────────────────────────────────────────────────────
# Status: ✅ DEVELOPMENT
# Phase: 2 (Chronos - Time Travel)
#
# Demonstrates:
#   • Enhanced CoW (Copy-on-Write) behavior
#   • File creation in snapshots
#   • Directory operations with CoW
#   • Snapshot isolation verification
#   • Edge cases and refinements
#
# Key Concept:
#   When a snapshot exists, modifications trigger CoW.
#   The divergence path is cloned, creating new inodes.
#   Snapshot remains unchanged, live fs is modified.
#
# Running Time: ~2-3 seconds
# Output: Technical logs showing CoW operations
#
# Usage:
#   bash scripts/test_phase2.sh
#

# quick_test.sh
# ─────────────────────────────────────────────────────────
# Status: ? (Check locally)
# Purpose: Quick sanity checks
#
# Usage:
#   bash scripts/quick_test.sh
#

# ============================================================
# RUNNING THE SCRIPTS
# ============================================================

# All scripts:
#   1. Build the project automatically
#   2. Mount FUSE filesystem at ./mnt
#   3. Run demonstrations
#   4. Unmount on exit (Ctrl+C)
#   5. Leave output for review

# Example Session:
#   $ bash scripts/demo_tagfs.sh
#   [Script runs... mounts at ./mnt]
#   [Shows TagFS capabilities]
#   [Filesystem stays mounted]
#   $ ls mnt/
#   projects/  work/
#   $ cat mnt/projects/backend/2026/api.rs
#   [file content]
#   $ Ctrl+C
#   [Unmounts and exits]

# ============================================================
# TROUBLESHOOTING
# ============================================================

# Issue: "fusermount: mount failed" or permission errors
# Solution: 
#   1. Ensure FUSE is installed: apt install libfuse-dev
#   2. Check /etc/fuse.conf allows user_allow_other
#   3. Run: sudo usermod -a -G fuse $USER && reboot

# Issue: "Address already in use" or mount point busy
# Solution:
#   fusermount -u mnt  # Unmount manually
#   pkill -f "better-fs"  # Kill any lingering processes

# Issue: Script hangs or doesn't complete
# Solution:
#   Ctrl+C to interrupt
#   Check dmesg for FUSE-related errors
#   Try: dmesg | tail -20

# ============================================================
# ARCHITECTURE OVERVIEW
# ============================================================

# ArcFS = Phases 1 + 2 + 3 (+ 4 planned)

# Phase 1: Core Engine (Complete ✅)
#   • FUSE virtual filesystem
#   • Inode tree with Arc<RwLock> synchronization
#   • Content-Addressed Storage (CAS)
#   • Content-defined chunking with rolling hash
#   • Automatic deduplication via SHA256
#   • Persistence via sled embedded database
#
#   Demo: demo_versioning.sh

# Phase 2: Chronos (Complete ✅)
#   • Instant O(1) snapshots via lazy cloning
#   • Copy-on-Write (CoW) for divergence
#   • Virtual .snapshots/ directory
#   • Full filesystem state restoration
#   • Snapshot isolation verification
#
#   Demos: snapshot_demo.sh, test_phase2.sh

# Phase 3: TagFS (Complete ✅)
#   • Automatic tagging from directory hierarchy
#   • Tag-based file discovery and queries
#   • Order-independent access via permutations
#   • Multi-tag intersection queries
#   • Tag persistence across snapshots
#
#   Demo: demo_tagfs.sh
#   Tests: tests/tagfs_test.rs (16 passing tests)

# Phase 4: ZipFS (Planned)
#   • Archive mounting (.zip, .tar.gz)
#   • Transparent archive exploration
#   • Offset-based decompression
#   • Virtual inode creation for archive contents

# ============================================================
# LEARNING PATH FOR NEW USERS
# ============================================================

# Step 1: Understand the Core
#   Read: README.md (project overview)
#   Run: bash scripts/demo_versioning.sh
#   Understand: How files are chunked and deduplicated

# Step 2: Learn Time Travel
#   Read: PHASE2_SUMMARY.md
#   Run: bash scripts/snapshot_demo.sh
#   Understand: Snapshots and Copy-on-Write behavior

# Step 3: Master TagFS
#   Read: TAGFS_COMPLETE.md
#   Run: bash scripts/demo_tagfs.sh
#   Test: cargo test --test tagfs_test -- --nocapture
#   Understand: Tag-based organization and queries

# Step 4: Deep Dive
#   Read: Code files (src/fuse_handler.rs, src/file_manager.rs)
#   Run: Tests with output (cargo test -- --nocapture)
#   Explore: ./mnt/ filesystem while scripts run

# ============================================================
# DOCUMENTATION REFERENCE
# ============================================================

# README.md
#   • Project overview
#   • Quick start guide
#   • Feature descriptions
#   • Architecture diagrams
#   • Timeline and phases

# TAGFS_COMPLETE.md
#   • TagFS specification
#   • Implementation details
#   • Tag storage architecture
#   • Lookup algorithms
#   • Testing strategy
#   • Known limitations

# PHASE2_SUMMARY.md
#   • Chronos specification
#   • Snapshot mechanics
#   • Copy-on-Write details
#   • Virtual directory design
#   • Performance characteristics

# TAGFS_TESTS.md
#   • Test suite documentation
#   • 16 test cases breakdown
#   • Coverage analysis
#   • Running instructions
#   • Future enhancements

# ============================================================
# COMMAND REFERENCE
# ============================================================

# Running All Tests:
#   cargo test                              # All tests
#   cargo test -- --nocapture             # With output
#   cargo test --test tagfs_test           # TagFS only
#   cargo test --test backend_stress       # Deduplication
#   cargo test --test gc_test              # Garbage collection

# Running a Specific Test:
#   cargo test tagfs_1_basic_tag_storage -- --nocapture
#   cargo test snapshot -- --nocapture

# Building:
#   cargo build                             # Debug build
#   cargo build --release                   # Optimized

# Running Manually:
#   ./target/release/better-fs mount mnt   # Mount filesystem
#   # In another terminal:
#   ls mnt/
#   mkdir mnt/test/example/2026
#   echo "data" > mnt/test/example/2026/file.txt
#   fusermount -u mnt                       # Unmount

# Cleaning Up:
#   rm -rf mnt my_storage test_*            # Full cleanup
#   cargo clean                             # Remove build artifacts

# ============================================================
# USEFUL NOTES
# ============================================================

# • All demo scripts auto-cleanup on exit (Ctrl+C)
# • Filesystem remains mounted for manual exploration
# • FUSE prints detailed logs (useful for understanding)
# • Storage is in ./my_storage/ (sled database + CAS)
# • Each run starts fresh (previous data deleted)
# • Error messages are informative - don't ignore them!

# ============================================================
# NEXT STEPS
# ============================================================

# 1. Run demo_tagfs.sh to understand the features
# 2. Create files and explore the filesystem
# 3. Read TAGFS_COMPLETE.md for details
# 4. Run test suite to validate functionality
# 5. Check source code for implementation details
# 6. Experiment with complex directory structures

# Example Experiments:
#
#   # Try deep hierarchies
#   mkdir -p mnt/a/b/c/d/e/f && echo "test" > mnt/a/b/c/d/e/f/deep.txt
#
#   # Create files with similar names in different dirs
#   echo "1" > mnt/x/y/file.txt
#   echo "2" > mnt/y/x/file.txt
#
#   # Explore FUSE behavior
#   stat mnt/x/y/file.txt        # View metadata
#   ls -i mnt/x/y/               # View inode numbers
#
#   # Look at storage structure
#   ls -la my_storage/           # Database contents
#   find my_storage/ -type f     # All stored chunks

echo "Scripts directory contains demonstration scripts for ArcFS features."
echo "See this file for detailed documentation."
echo ""
echo "Quick start: bash scripts/demo_tagfs.sh"
