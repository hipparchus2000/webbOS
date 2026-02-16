# WebbOS Build System

.PHONY: all clean run test bootloader kernel iso qemu qemu-aarch64 run-x64 run-aarch64

# Directories
BUILD_DIR := build
ISO_DIR := $(BUILD_DIR)/iso
OVMF_DIR := $(BUILD_DIR)/ovmf
AARCH64_DIR := $(BUILD_DIR)/aarch64

# Tools
CARGO := cargo
QEMU_X64 := qemu-system-x86_64
QEMU_AARCH64 := qemu-system-aarch64

# QEMU Flags
QEMU_X64_FLAGS := -m 512M -smp 4 -cpu qemu64
QEMU_UEFI_FLAGS := $(QEMU_X64_FLAGS) -bios $(OVMF_DIR)/OVMF.fd
QEMU_DEBUG_FLAGS := -S -s -serial stdio

QEMU_AARCH64_FLAGS := -M virt,highmem=off -cpu cortex-a72 -m 1024M -smp 4

all: $(BUILD_DIR)/webbos.iso

# Create build directories
$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(ISO_DIR):
	mkdir -p $(ISO_DIR)/EFI/BOOT

$(OVMF_DIR):
	mkdir -p $(OVMF_DIR)

# Download OVMF for UEFI testing
$(OVMF_DIR)/OVMF.fd: | $(OVMF_DIR)
	@echo "Downloading OVMF..."
	ifeq ($(OS),Windows_NT)
		powershell -Command "Invoke-WebRequest -Uri 'https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd' -OutFile '$(OVMF_DIR)/OVMF.fd'"
	else
		curl -L -o $(OVMF_DIR)/OVMF.fd https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd
	endif

# Build bootloader
bootloader:
	cd bootloader && $(CARGO) build --target x86_64-unknown-uefi
	cd bootloader && $(CARGO) build --target x86_64-unknown-uefi --release

# Build kernel
kernel:
	cd kernel && $(CARGO) build --target x86_64-unknown-none
	cd kernel && $(CARGO) build --target x86_64-unknown-none --release

# Create bootable ISO
$(BUILD_DIR)/webbos.iso: bootloader kernel | $(ISO_DIR)
	@echo "Creating bootable ISO..."
	# Copy bootloader
	cp target/x86_64-unknown-uefi/release/bootloader.efi $(ISO_DIR)/EFI/BOOT/BOOTX64.EFI || \
		cp target/x86_64-unknown-uefi/debug/bootloader.efi $(ISO_DIR)/EFI/BOOT/BOOTX64.EFI
	# Copy kernel
	cp target/x86_64-unknown-none/release/kernel $(ISO_DIR)/kernel.elf || \
		cp target/x86_64-unknown-none/debug/kernel $(ISO_DIR)/kernel.elf
	# Create initrd
	mkdir -p $(ISO_DIR)/boot
	echo "WebbOS v0.1.0" > $(ISO_DIR)/boot/version.txt
	# Create ISO using xorriso or equivalent
	# For now, just create the directory structure
	@echo "ISO directory prepared at $(ISO_DIR)"

# Run in QEMU with UEFI
run: $(BUILD_DIR)/webbos.iso $(OVMF_DIR)/OVMF.fd
	$(QEMU) $(QEMU_UEFI_FLAGS) -cdrom $(BUILD_DIR)/webbos.iso

# Run with debug output
run-debug: $(BUILD_DIR)/webbos.iso $(OVMF_DIR)/OVMF.fd
	$(QEMU) $(QEMU_UEFI_FLAGS) -cdrom $(BUILD_DIR)/webbos.iso -serial stdio

# Run with GDB debugging
debug: $(BUILD_DIR)/webbos.iso $(OVMF_DIR)/OVMF.fd
	$(QEMU) $(QEMU_UEFI_FLAGS) $(QEMU_DEBUG_FLAGS) -cdrom $(BUILD_DIR)/webbos.iso

# Run tests
test:
	cd shared && $(CARGO) test
	cd kernel && $(CARGO) test --lib
	cd bootloader && $(CARGO) test --lib

# Format code
fmt:
	$(CARGO) fmt --all

# Run clippy
lint:
	$(CARGO) clippy --all -- -D warnings

# Clean build artifacts
clean:
	rm -rf $(BUILD_DIR)
	$(CARGO) clean

# Generate coverage report
coverage:
	cd shared && $(CARGO) tarpaulin --out Html --output-dir ../$(BUILD_DIR)/coverage/shared
	cd kernel && $(CARGO) tarpaulin --out Html --output-dir ../$(BUILD_DIR)/coverage/kernel
	@echo "Coverage reports generated in $(BUILD_DIR)/coverage/"

# AArch64 builds
aarch64-kernel:
	cd kernel && $(CARGO) build --target aarch64-unknown-none --release

aarch64-image: aarch64-kernel | $(AARCH64_DIR)
	objcopy -O binary kernel/target/aarch64-unknown-none/release/kernel $(AARCH64_DIR)/kernel8.img
	cp kernel/target/aarch64-unknown-none/release/kernel $(AARCH64_DIR)/webbos-kernel.elf
	@echo "AArch64 kernel image created: $(AARCH64_DIR)/kernel8.img"

# QEMU runs
run-x64: kernel | $(ISO_DIR)
	@echo "Running x86_64 in QEMU..."
	@cp target/x86_64-unknown-none/release/kernel $(ISO_DIR)/kernel.elf 2>/dev/null || \
		cp target/x86_64-unknown-none/debug/kernel $(ISO_DIR)/kernel.elf
	$(QEMU_X64) $(QEMU_X64_FLAGS) -kernel $(ISO_DIR)/kernel.elf -serial stdio -no-reboot

run-aarch64: aarch64-image
	@echo "Running AArch64 in QEMU..."
	$(QEMU_AARCH64) $(QEMU_AARCH64_FLAGS) -kernel $(AARCH64_DIR)/webbos-kernel.elf -serial stdio -no-reboot

# Direct kernel run (debug builds)
run-x64-debug:
	cd kernel && cargo run --target x86_64-unknown-none 2>&1 | head -100

run-aarch64-debug:
	cd kernel && cargo run --target aarch64-unknown-none 2>&1 | head -100

$(AARCH64_DIR):
	mkdir -p $(AARCH64_DIR)
