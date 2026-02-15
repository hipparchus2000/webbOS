#!/bin/bash
# Build script for ARM64 drivers (Week 3 deliverables)
#
# This script builds the webbOS kernel with GPIO, UART, USB, and Ethernet drivers
# for the Raspberry Pi 5 ARM64 port.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== WebbOS ARM64 Driver Build System (Week 3) ===${NC}"
echo ""

# Function to print section headers
print_section() {
    echo -e "${BLUE}>>> $1${NC}"
}

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo -e "${RED}Error: Rust is not installed. Please install Rust first.${NC}"
    exit 1
fi

# Add ARM64 target
print_section "Adding aarch64-unknown-none target..."
rustup target add aarch64-unknown-none 2>/dev/null || echo "Target already installed"

# Check for required tools
print_section "Checking build dependencies..."
for tool in objcopy objdump; do
    if ! command -v $tool &> /dev/null; then
        echo -e "${YELLOW}Warning: $tool not found. Install binutils.${NC}"
    fi
done

# Create output directories
mkdir -p build/aarch64
echo "Output directory: build/aarch64/"

# Build kernel with driver support
print_section "Building kernel with driver support..."
cd kernel

# Set build flags for driver features
export RUSTFLAGS="-C target-cpu=cortex-a76 -C target-feature=+crc,+lse"

echo "Building for target: aarch64-unknown-none"
echo "Features: GPIO, UART, USB (research), Ethernet (research)"

# Build kernel
cargo build --target aarch64-unknown-none --release \
    --features "" \
    2>&1 | tee ../build/aarch64/build.log

cd ..

# Check if build succeeded
if [ ! -f "kernel/target/aarch64-unknown-none/release/webbos-kernel" ]; then
    echo -e "${RED}Error: Kernel build failed. Check build.log${NC}"
    exit 1
fi

echo -e "${GREEN}Kernel build successful!${NC}"

# Create kernel image for Raspberry Pi 5
print_section "Creating kernel image..."

# Copy kernel binary
cp kernel/target/aarch64-unknown-none/release/webbos-kernel build/aarch64/webbos-kernel.elf

# Create binary image for SD card
if command -v objcopy &> /dev/null; then
    objcopy -O binary kernel/target/aarch64-unknown-none/release/webbos-kernel build/aarch64/kernel8.img
    echo -e "${GREEN}Created kernel8.img for Raspberry Pi 5${NC}"
    
    # Display image info
    ls -lh build/aarch64/kernel8.img
else
    echo -e "${YELLOW}Warning: objcopy not found. Cannot create kernel8.img${NC}"
fi

# Generate disassembly for debugging
if command -v objdump &> /dev/null; then
    print_section "Generating disassembly..."
    objdump -d kernel/target/aarch64-unknown-none/release/webbos-kernel > build/aarch64/kernel.dis 2>/dev/null || true
    echo "Disassembly saved to: build/aarch64/kernel.dis"
fi

# Print driver information
print_section "Driver Build Summary"
echo ""
echo "Drivers included in this build:"
echo "  [✓] GPIO Driver (Raspberry Pi 5 RP1/BCM2711)"
echo "  [✓] UART Driver (PL011 + Mini UART)"
echo "  [✓] HAL (Hardware Abstraction Layer)"
echo "  [R] USB Driver (Research/Skeleton)"
echo "  [R] Ethernet Driver (Research/Skeleton)"
echo ""
echo "Legend: [✓] Implemented  [R] Research Phase"

# Print output files
print_section "Output Files"
echo ""
echo "Build outputs in build/aarch64/:"
ls -lh build/aarch64/ 2>/dev/null || echo "  (directory listing failed)"
echo ""

# Create config.txt for SD card
print_section "Creating SD card configuration..."
cat > build/aarch64/config.txt << 'EOF'
# WebbOS Configuration for Raspberry Pi 5
# Place this file and kernel8.img on the boot partition

# Kernel configuration
kernel=kernel8.img
arm_64bit=1
enable_uart=1

# UART configuration
uart_2ndstage=1
core_freq_min=500

# Memory configuration
gpu_mem=16

# Optional: Overclock (uncomment if needed)
# arm_freq=2400
# over_voltage=6

# Logging
cmdline=cmdline.txt
EOF

echo "Created config.txt for SD card boot"

# Create a simple cmdline.txt
echo "console=ttyAMA0,115200 root=/dev/mmcblk0p2 rw rootwait" > build/aarch64/cmdline.txt
echo "Created cmdline.txt"

# Create deployment package
print_section "Creating deployment package..."
cd build/aarch64
tar czf webbos-drivers-week3.tar.gz kernel8.img config.txt cmdline.txt 2>/dev/null || true
cd ../..

if [ -f "build/aarch64/webbos-drivers-week3.tar.gz" ]; then
    echo -e "${GREEN}Deployment package created: build/aarch64/webbos-drivers-week3.tar.gz${NC}"
fi

echo ""
echo -e "${GREEN}=== Build Complete! ===${NC}"
echo ""
echo "To deploy to Raspberry Pi 5:"
echo "  1. Format SD card with FAT32 boot partition"
echo "  2. Copy build/aarch64/kernel8.img to SD card"
echo "  3. Copy build/aarch64/config.txt to SD card"
echo "  4. Insert SD card and power on Pi 5"
echo ""
echo "To test in QEMU:"
echo "  ./test-qemu-aarch64.sh"
echo ""
echo "Documentation available in: docs/drivers/"
