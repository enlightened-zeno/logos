#!/bin/bash
# Create a bootable LogOS disk image without requiring root/sudo.
# Uses dd + mkfs.fat and mtools (or fallback to just FAT image for UEFI boot).
set -e

KERNEL="${1:-target/x86_64-logos/debug/logos-kernel}"
LIMINE_DIR="build/limine"
IMAGE="build/logos.img"

if [ ! -f "$KERNEL" ]; then
    echo "ERROR: Kernel binary not found at $KERNEL"
    echo "Run 'cargo build' first."
    exit 1
fi

if [ ! -f "$LIMINE_DIR/BOOTX64.EFI" ]; then
    echo "ERROR: Limine not found. Run './tools/fetch-limine.sh' first."
    exit 1
fi

mkdir -p build

# Create a FAT32 image large enough for bootloader + kernel
# This is the ESP (EFI System Partition)
FAT_IMG="build/esp.img"
dd if=/dev/zero of="$FAT_IMG" bs=1M count=64 2>/dev/null
mkfs.fat -F 32 -s 1 "$FAT_IMG" >/dev/null

# Use mtools to copy files without mounting
if command -v mcopy &>/dev/null; then
    mmd -i "$FAT_IMG" ::EFI
    mmd -i "$FAT_IMG" ::EFI/BOOT
    mcopy -i "$FAT_IMG" "$LIMINE_DIR/BOOTX64.EFI" "::EFI/BOOT/BOOTX64.EFI"
    mcopy -i "$FAT_IMG" "$KERNEL" "::kernel.elf"

    # Create limine config
    CONF=$(mktemp)
    cat > "$CONF" << 'EOF'
timeout: 0
serial: yes

/LogOS
    protocol: limine
    kernel_path: boot():/kernel.elf
EOF
    mcopy -i "$FAT_IMG" "$CONF" "::limine.conf"
    rm "$CONF"
else
    echo "WARNING: mtools not installed. Creating image requires sudo mount."
    echo "Install mtools: sudo apt-get install -y mtools"

    MNT=$(mktemp -d)
    sudo mount -o loop "$FAT_IMG" "$MNT"
    sudo mkdir -p "$MNT/EFI/BOOT"
    sudo cp "$LIMINE_DIR/BOOTX64.EFI" "$MNT/EFI/BOOT/"
    sudo cp "$KERNEL" "$MNT/kernel.elf"
    sudo tee "$MNT/limine.conf" > /dev/null << 'EOF'
timeout: 0
serial: yes

/LogOS
    protocol: limine
    kernel_path: boot():/kernel.elf
EOF
    sudo umount "$MNT"
    rmdir "$MNT"
fi

# Create GPT disk image with the ESP as the first partition
# We use a simple dd + partition table approach
dd if=/dev/zero of="$IMAGE" bs=1M count=64 2>/dev/null

# Write a simple GPT-like structure: just use the FAT image as a raw drive
# For QEMU testing, we can boot directly from the FAT image using OVMF
cp "$FAT_IMG" "$IMAGE"

echo "Disk image created: $IMAGE"
echo "Boot with: qemu-system-x86_64 -bios /usr/share/OVMF/OVMF_CODE.fd -drive file=$IMAGE,format=raw"
