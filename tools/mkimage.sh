#!/bin/bash
# Create a bootable LogOS disk image with Limine bootloader
set -e

KERNEL="target/x86_64-logos/debug/logos-kernel"
LIMINE_DIR="build/limine"
IMAGE="build/logos.img"
SIZE_MB=64

if [ ! -f "$KERNEL" ]; then
    echo "ERROR: Kernel binary not found at $KERNEL"
    echo "Run 'cargo build' first."
    exit 1
fi

if [ ! -f "$LIMINE_DIR/BOOTX64.EFI" ]; then
    echo "ERROR: Limine bootloader not found. Run './tools/fetch-limine.sh' first."
    exit 1
fi

mkdir -p build

echo "Creating ${SIZE_MB}MiB disk image..."
dd if=/dev/zero of="$IMAGE" bs=1M count=$SIZE_MB 2>/dev/null

echo "Creating GPT partition table..."
parted -s "$IMAGE" mklabel gpt
parted -s "$IMAGE" mkpart ESP fat32 1MiB 33MiB
parted -s "$IMAGE" set 1 esp on
parted -s "$IMAGE" mkpart root ext2 33MiB 100%

echo "Setting up loop device..."
LOOP=$(sudo losetup -fP --show "$IMAGE")

echo "Formatting ESP (FAT32)..."
sudo mkfs.fat -F 32 "${LOOP}p1" >/dev/null

echo "Formatting root (ext2)..."
sudo mkfs.ext2 -q "${LOOP}p2"

echo "Mounting ESP..."
MNT=$(mktemp -d)
sudo mount "${LOOP}p1" "$MNT"

echo "Installing Limine and kernel..."
sudo mkdir -p "$MNT/EFI/BOOT"
sudo cp "$LIMINE_DIR/BOOTX64.EFI" "$MNT/EFI/BOOT/"

# Limine configuration
sudo tee "$MNT/limine.conf" > /dev/null << 'LIMINE_EOF'
timeout: 0
serial: yes

/LogOS
    protocol: limine
    kernel_path: boot():/kernel.elf
LIMINE_EOF

sudo cp "$KERNEL" "$MNT/kernel.elf"

echo "Unmounting..."
sudo umount "$MNT"
rmdir "$MNT"

echo "Installing root filesystem..."
MNT=$(mktemp -d)
sudo mount "${LOOP}p2" "$MNT"
sudo mkdir -p "$MNT"/{sbin,bin,dev,proc,tmp,etc}
echo "LogOS v0.1.0" | sudo tee "$MNT/etc/version" > /dev/null
sudo umount "$MNT"
rmdir "$MNT"

sudo losetup -d "$LOOP"

echo "Disk image created: $IMAGE"
