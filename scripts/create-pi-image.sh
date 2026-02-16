#!/bin/bash
# Create Raspberry Pi bootable image
# Combines bootloader and kernel into kernel8.img

set -e

echo "=== WebbOS Raspberry Pi Image Creator ==="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Directories
BUILD_DIR="build/aarch64"
mkdir -p "$BUILD_DIR"

# Build bootloader
echo -e "${YELLOW}Building bootloader...${NC}"
cargo build --target aarch64-unknown-none --release -p bootloader-pi 2>&1 | tail -3

# Build kernel
echo -e "${YELLOW}Building kernel...${NC}"
cargo build --target aarch64-unknown-none --release -p kernel 2>&1 | tail -3

# Get binary files
BOOTLOADER_BIN="target/aarch64-unknown-none/release/bootloader-pi"
KERNEL_BIN="target/aarch64-unknown-none/release/kernel"

# Convert to binary format
echo -e "${YELLOW}Creating binary images...${NC}"
aarch64-linux-gnu-objcopy -O binary "$BOOTLOADER_BIN" "$BUILD_DIR/bootloader.bin"
aarch64-linux-gnu-objcopy -O binary "$KERNEL_BIN" "$BUILD_DIR/kernel.bin"

# Create combined image
# Bootloader at 0x80000, kernel at 0x100000
# We need to create a 16MB image with proper offsets

echo -e "${YELLOW}Creating combined kernel8.img...${NC}"

# Create empty image (16MB)
IMAGE_SIZE=$((16 * 1024 * 1024))
dd if=/dev/zero of="$BUILD_DIR/kernel8.img" bs=1 count=0 seek=$IMAGE_SIZE 2>/dev/null

# Copy bootloader at offset 0x80000 (512KB)
dd if="$BUILD_DIR/bootloader.bin" of="$BUILD_DIR/kernel8.img" bs=1 seek=524288 conv=notrunc 2>/dev/null

# Copy kernel at offset 0x100000 (1MB)
dd if="$BUILD_DIR/kernel.bin" of="$BUILD_DIR/kernel8.img" bs=1 seek=1048576 conv=notrunc 2>/dev/null

echo -e "${GREEN}Image created: $BUILD_DIR/kernel8.img${NC}"
ls -lh "$BUILD_DIR/kernel8.img"

# Create config.txt
cat > "$BUILD_DIR/config.txt" << 'EOF'
# WebbOS Configuration for Raspberry Pi 4/5

# Load the combined bootloader+kernel image
kernel=kernel8.img
arm_64bit=1

# Enable UART for early boot messages
enable_uart=1
uart_2ndstage=1

# Memory
gpu_mem=16

# Overclock (optional)
# arm_freq=2000
# over_voltage=6
EOF

echo -e "${GREEN}Config created: $BUILD_DIR/config.txt${NC}"

# Create cmdline.txt
echo "console=ttyAMA0,115200" > "$BUILD_DIR/cmdline.txt"

# Instructions
echo ""
echo "=== Instructions ==="
echo "1. Format SD card with FAT32"
echo "2. Copy these files to the SD card:"
echo "   - $BUILD_DIR/kernel8.img"
echo "   - $BUILD_DIR/config.txt"
echo "   - $BUILD_DIR/cmdline.txt"
echo "3. Boot on Raspberry Pi 4/5"
echo ""
echo "Or test in QEMU:"
echo "  qemu-system-aarch64 -M raspi3b -kernel $BUILD_DIR/kernel8.img -serial stdio -display none"
