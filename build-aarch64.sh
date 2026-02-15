#!/bin/bash
# Build script for ARM64 (aarch64) target

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Building webbOS for ARM64 (aarch64) ===${NC}"

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo -e "${RED}Error: Rust is not installed. Please install Rust first.${NC}"
    exit 1
fi

# Add ARM64 target
echo -e "${YELLOW}Adding aarch64-unknown-none target...${NC}"
rustup target add aarch64-unknown-none

# Build kernel
echo -e "${YELLOW}Building kernel for aarch64-unknown-none...${NC}"
cd kernel
cargo build --target aarch64-unknown-none --release
cd ..

# Build bootloader (ARM64 version)
echo -e "${YELLOW}Building bootloader for aarch64-unknown-none...${NC}"
cd bootloader
cargo build --target aarch64-unknown-none --release
cd ..

echo -e "${GREEN}=== Build complete! ===${NC}"
echo "Output files:"
echo "  - kernel/target/aarch64-unknown-none/release/webbos-kernel"
echo "  - bootloader/target/aarch64-unknown-none/release/webbos-bootloader"

# Create kernel image for Raspberry Pi 5
echo -e "${YELLOW}Creating kernel8.img for Raspberry Pi 5...${NC}"
objcopy -O binary kernel/target/aarch64-unknown-none/release/webbos-kernel kernel8.img

echo -e "${GREEN}Kernel image created: kernel8.img${NC}"
echo -e "${YELLOW}Copy this file to a FAT32 SD card along with config.txt${NC}"