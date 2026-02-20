# WebbOS Makefile for Raspberry Pi ARM64

# Toolchain
CARGO := cargo
RUSTC := rustc
OBJCOPY := rust-objcopy
OBJDUMP := rust-objdump

# Targets
BOOTLOADER_TARGET := aarch64-unknown-none
KERNEL_TARGET := aarch64-unknown-none

# Output files
KERNEL_ELF := target/aarch64-unknown-none/debug/kernel
KERNEL_BIN := target/aarch64-unknown-none/debug/kernel.bin
BOOTLOADER_BIN := target/aarch64-unknown-none/debug/bootloader.bin
KERNEL_IMG := kernel8.img

# QEMU settings
QEMU := qemu-system-aarch64
QEMU_MACHINE := raspi3b
QEMU_MEMORY := 1G

.PHONY: all build clean run debug bootloader kernel

all: build

build: $(KERNEL_IMG)

# Build kernel
kernel:
	$(CARGO) +nightly-2025-01-15 build -p kernel --target $(KERNEL_TARGET) -Z build-std=core,compiler_builtins,alloc

# Build bootloader
bootloader:
	$(CARGO) +nightly-2025-01-15 build -p bootloader --target $(BOOTLOADER_TARGET) -Z build-std=core,compiler_builtins,alloc

# Create kernel8.img (raw binary for Pi)
$(KERNEL_IMG): kernel
	$(OBJCOPY) -O binary $(KERNEL_ELF) $(KERNEL_IMG)
	@echo "Created $(KERNEL_IMG)"

# Create bootloader binary
$(BOOTLOADER_BIN): bootloader
	$(OBJCOPY) -O binary target/aarch64-unknown-none/debug/bootloader $(BOOTLOADER_BIN)

# Run in QEMU
run: $(KERNEL_IMG)
	$(QEMU) -M $(QEMU_MACHINE) -m $(QEMU_MEMORY) -serial stdio -kernel $(KERNEL_IMG) -dtb bcm2710-rpi-3-b.dtb

# Debug with QEMU
debug: $(KERNEL_IMG)
	$(QEMU) -M $(QEMU_MACHINE) -m $(QEMU_MEMORY) -serial stdio -kernel $(KERNEL_IMG) -S -s

# Disassemble kernel
disasm: kernel
	$(OBJDUMP) -d $(KERNEL_ELF) | head -200

# Clean build artifacts
clean:
	$(CARGO) clean
	rm -f $(KERNEL_IMG)

# Check code
check:
	$(CARGO) +nightly-2025-01-15 check -p kernel --target $(KERNEL_TARGET) -Z build-std=core,compiler_builtins,alloc
	$(CARGO) +nightly-2025-01-15 check -p bootloader --target $(BOOTLOADER_TARGET) -Z build-std=core,compiler_builtins,alloc

# Format code
fmt:
	$(CARGO) +nightly-2025-01-15 fmt

# Clippy lint
clippy:
	$(CARGO) +nightly-2025-01-15 clippy -p kernel --target $(KERNEL_TARGET) -Z build-std=core,compiler_builtins,alloc
	$(CARGO) +nightly-2025-01-15 clippy -p bootloader --target $(BOOTLOADER_TARGET) -Z build-std=core,compiler_builtins,alloc

# Create SD card image (for real hardware)
sd-image: $(KERNEL_IMG)
	@echo "Creating SD card image..."
	@echo "Copy $(KERNEL_IMG) to SD card as kernel8.img"
	@echo "Also copy necessary .dtb files and config.txt"
