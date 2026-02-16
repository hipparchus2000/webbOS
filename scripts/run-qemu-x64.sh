#!/bin/bash
# Run webbOS x86_64 in QEMU

set -e

echo "=== WebbOS x86_64 QEMU Launcher ==="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Build release kernel
echo -e "${YELLOW}Building kernel (release)...${NC}"
cd kernel
cargo build --target x86_64-unknown-none --release 2>&1 | tail -5
cd ..

# Check for OVMF
if [ ! -f "OVMF.fd" ]; then
    echo -e "${YELLOW}Downloading OVMF (UEFI firmware)...${NC}"
    curl -L -o OVMF.fd https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd
fi

# Create disk image with kernel
echo -e "${YELLOW}Creating disk image...${NC}"
mkdir -p build/iso/EFI/BOOT

# Copy kernel
cp target/x86_64-unknown-none/release/kernel build/iso/kernel.elf 2>/dev/null || \
    cp target/x86_64-unknown-none/debug/kernel build/iso/kernel.elf

# Create a simple boot script
cat > build/iso/EFI/BOOT/startup.nsh << 'EOF'
echo "Loading webbOS kernel..."
kernel.elf
EOF

# Run QEMU
echo -e "${GREEN}Starting QEMU...${NC}"
echo "Press Ctrl+A then X to exit"
echo ""

qemu-system-x86_64 \
    -m 512M \
    -smp 4 \
    -cpu qemu64 \
    -bios OVMF.fd \
    -kernel build/iso/kernel.elf \
    -serial stdio \
    -vga std \
    -netdev user,id=net0 \
    -device virtio-net-pci,netdev=net0 \
    -drive format=raw,file=fat:rw:build/iso \
    -no-reboot \
    -no-shutdown \
    "$@"
