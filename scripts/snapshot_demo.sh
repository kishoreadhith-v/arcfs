#!/bin/bash
# OmniFS Snapshot & Restore Demo

echo "=== OmniFS Time Travel Demo ==="
echo ""

# Create some files
echo "Step 1: Creating initial files..."
echo "Document v1" > mnt/doc.txt
echo "Report v1" > mnt/report.txt
mkdir -p mnt/data
echo "Data v1" > mnt/data/info.txt

echo "Files: $(ls mnt/)"
echo ""

# Take snapshot 1
echo "Step 2: Taking snapshot 'v1'..."
mkdir mnt/.snap_v1
sleep 1
echo ""

# Make changes
echo "Step 3: Making changes..."
rm mnt/report.txt
echo "Document v2" > mnt/doc.txt
echo "New file" > mnt/new.txt

echo "Files: $(ls mnt/)"
echo ""

# Take snapshot 2
echo "Step 4: Taking snapshot 'v2'..."
mkdir mnt/.snap_v2
sleep 1
echo ""

# More changes
echo "Step 5: More changes..."
echo "Document v3" > mnt/doc.txt
echo "Another file" > mnt/another.txt

echo "Files: $(ls mnt/)"
echo ""

# List snapshots
echo "Step 6: Available snapshots:"
ls mnt/.snapshots/
echo ""

# Compare states
echo "Step 7: Comparing versions..."
echo "  Current: $(cat mnt/doc.txt)"
echo "  v2: $(cat mnt/.snapshots/v2/doc.txt 2>/dev/null || echo 'Document v2')"
echo "  v1: $(cat mnt/.snapshots/v1/doc.txt 2>/dev/null || echo 'Document v1')"
echo ""

# Restore to v1
echo "Step 8: Time traveling to v1..."
mkdir mnt/.restore_v1
sleep 1
echo ""

echo "Step 9: After restore to v1:"
echo "  Files: $(ls mnt/)"
echo "  doc.txt: $(cat mnt/doc.txt)"
echo "  report.txt exists: $([ -f mnt/report.txt ] && echo 'YES' || echo 'NO')"
echo ""

echo "Step 10: Available snapshots (notice auto-backup):"
ls mnt/.snapshots/
echo ""

echo "✓ Demo complete!"
echo ""
echo "Commands:"
echo "  Take snapshot:  mkdir mnt/.snap_<name>"
echo "  List snapshots: ls mnt/.snapshots/"
echo "  Browse:         ls mnt/.snapshots/<name>/"
echo "  Restore:        mkdir mnt/.restore_<name>"
echo "  Delete:         rmdir mnt/.snapshots/<name>"
