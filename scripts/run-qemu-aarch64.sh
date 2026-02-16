#!/bin/bash
# Run webbOS aarch64 in QEMU (Raspberry Pi 4/5 emulation)

set -e

echo "=== WebbOS AArch64 QEMU Launcher ==="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Build release kernel
echo -e "${YELLOW}Building kernel (release)...${NC}"
cd kernel
cargo build --target aarch64-unknown-none --release 2>&1 | tail -5
cd ..

# Create kernel image
echo -e "${YELLOW}Creating kernel image...${NC}"
mkdir -p build/aarch64

# Copy and convert kernel
cp target/aarch64-unknown-none/release/kernel build/aarch64/webbos-kernel.elf 2>/dev/null || \
    cp target/aarch64-unknown-none/debug/kernel build/aarch64/webbos-kernel.elf

# Create binary image
if command -v objcopy &> /dev/null; then
    objcopy -O binary build/aarch64/webbos-kernel.elf build/aarch64/kernel8.img
    echo -e "${GREEN}Created kernel8.img${NC}"
else
    echo "Warning: objcopy not found, using ELF directly"
fi

# Create config.txt for QEMU
cat > build/aarch64/config.txt << 'EOF'
# WebbOS Configuration for QEMU ARM64
kernel=kernel8.img
arm_64bit=1
enable_uart=1
uart_2ndstage=1
core_freq_min=500
gpu_mem=16
EOF

# Run QEMU
echo -e "${GREEN}Starting QEMU with virt machine...${NC}"
echo "Press Ctrl+A then X to exit"
echo ""

# Use virt machine for better compatibility
qemu-system-aarch64 \
    -M virt,highmem=off \
    -cpu cortex-a72 \
    -m 1024M \
    -smp 4 \
    -kernel build/aarch64/webbos-kernel.elf \
    -serial stdio \
    -netdev user,id=net0 \
    -device virtio-net-device,netdev=net0 \
    -device virtio-rng-pci \
    -no-reboot \
    -no-shutdown \
    "$@"
