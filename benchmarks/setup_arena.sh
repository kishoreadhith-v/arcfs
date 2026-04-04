#!/bin/bash
set -e

ARENA_DIR="/home/kishore/benchmark_arena"

echo "========================================"
echo " Allocating Benchmark Arena Loopbacks"
echo "========================================"

mkdir -p "$ARENA_DIR"/{ext4_mount,btrfs_mount,arcfs_mount,arcfs_backend}
cd "$ARENA_DIR"

if [ ! -f "ext4_disk.img" ]; then
    echo "[+] Generating 5GB Ext4 Loopback Image..."
    dd if=/dev/zero of=ext4_disk.img bs=1M count=5120 status=progress
    mkfs.ext4 -F ext4_disk.img
fi

if [ ! -f "btrfs_disk.img" ]; then
    echo "[+] Generating 5GB Btrfs Loopback Image..."
    dd if=/dev/zero of=btrfs_disk.img bs=1M count=5120 status=progress
    mkfs.btrfs -f btrfs_disk.img
fi

echo "========================================"
echo " Mounting Loopbacks (Requires Sudo)"
echo "========================================"

# Mount Ext4
if ! mountpoint -q "$ARENA_DIR/ext4_mount"; then
    sudo mount -o loop ext4_disk.img ext4_mount
    echo "[+] Ext4 mounted."
else
    echo "[-] Ext4 already mounted."
fi

# Mount Btrfs
if ! mountpoint -q "$ARENA_DIR/btrfs_mount"; then
    sudo mount -o loop btrfs_disk.img btrfs_mount
    echo "[+] Btrfs mounted."
else
    echo "[-] Btrfs already mounted."
fi

echo "========================================"
echo " Setting Permissions"
echo "========================================"
sudo chown -R $USER:$USER "$ARENA_DIR"
sudo chmod -R 777 "$ARENA_DIR/ext4_mount" "$ARENA_DIR/btrfs_mount" "$ARENA_DIR/arcfs_mount" "$ARENA_DIR/arcfs_backend"

echo "[+] Arena setup complete! Ready for FIO."
df -h | grep _mount
