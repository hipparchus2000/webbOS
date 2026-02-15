#!/bin/bash
# Test webbOS on QEMU ARM64

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Testing webbOS on QEMU ARM64 ===${NC}"

# Check if QEMU is installed
if ! command -v qemu-system-aarch64 &> /dev/null; then
    echo -e "${RED}Error: qemu-system-aarch64 is not installed.${NC}"
    echo "Install with: sudo apt-get install qemu-system-arm"
    exit 1
fi

# Check if kernel is built
KERNEL_PATH="kernel/target/aarch64-unknown-none/release/webbos-kernel"
if [ ! -f "$KERNEL_PATH" ]; then
    echo -e "${RED}Error: Kernel not found at $KERNEL_PATH${NC}"
    echo "Run ./build-aarch64.sh first"
    exit 1
fi

echo -e "${YELLOW}Starting QEMU ARM64 virtual machine...${NC}"
echo -e "${YELLOW}Press Ctrl+A then X to exit QEMU${NC}"

# Run QEMU with ARM64 virt machine
qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a72 \
    -smp 4 \
    -m 2G \
    -kernel "$KERNEL_PATH" \
    -serial stdio \
    -display none \
    -device virtio-gpu-pci \
    -device virtio-keyboard-pci \
    -device virtio-mouse-pci \
    -device virtio-blk-device,drive=disk0 \
    -drive if=none,id=disk0,file=disk.img,format=raw \
    -device virtio-net-device,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -append "console=ttyAMA0"