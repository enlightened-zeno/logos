#!/bin/bash
# Download and build Limine bootloader
set -e

LIMINE_VERSION="8.6.0"
LIMINE_DIR="build/limine"
LIMINE_SRC="/tmp/limine-src"

mkdir -p "$LIMINE_DIR"

if [ -f "$LIMINE_DIR/BOOTX64.EFI" ]; then
    echo "Limine already available at $LIMINE_DIR"
    exit 0
fi

echo "Fetching Limine v${LIMINE_VERSION}..."

# Clone the binary branch which has pre-built binaries
if [ ! -d "$LIMINE_SRC" ]; then
    git clone --depth=1 --branch=v${LIMINE_VERSION}-binary \
        https://github.com/limine-bootloader/limine.git "$LIMINE_SRC" 2>/dev/null || {
        # Fallback: try the source release and build
        echo "Binary branch not found, trying source build..."
        curl -Lo /tmp/limine.tar.gz \
            "https://github.com/limine-bootloader/limine/archive/refs/tags/v${LIMINE_VERSION}.tar.gz"
        mkdir -p "$LIMINE_SRC"
        tar -xzf /tmp/limine.tar.gz -C "$LIMINE_SRC" --strip-components=1
        rm /tmp/limine.tar.gz
        cd "$LIMINE_SRC"
        make -j$(nproc) 2>/dev/null || true
        cd -
    }
fi

# Copy the EFI binary
if [ -f "$LIMINE_SRC/BOOTX64.EFI" ]; then
    cp "$LIMINE_SRC/BOOTX64.EFI" "$LIMINE_DIR/"
elif [ -f "$LIMINE_SRC/bin/BOOTX64.EFI" ]; then
    cp "$LIMINE_SRC/bin/BOOTX64.EFI" "$LIMINE_DIR/"
else
    echo "ERROR: Could not find BOOTX64.EFI"
    echo "Contents of $LIMINE_SRC:"
    ls "$LIMINE_SRC/" 2>/dev/null
    exit 1
fi

# Copy BIOS files if available
for f in limine-bios.sys limine-bios-cd.bin limine-uefi-cd.bin; do
    cp "$LIMINE_SRC/$f" "$LIMINE_DIR/" 2>/dev/null || true
    cp "$LIMINE_SRC/bin/$f" "$LIMINE_DIR/" 2>/dev/null || true
done

# Copy limine CLI tool if available
cp "$LIMINE_SRC/limine" "$LIMINE_DIR/" 2>/dev/null || true
cp "$LIMINE_SRC/bin/limine" "$LIMINE_DIR/" 2>/dev/null || true

echo "Limine v${LIMINE_VERSION} ready at $LIMINE_DIR/"
ls -la "$LIMINE_DIR/"
