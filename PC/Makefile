# WebbOS Build System

.PHONY: all clean run test bootloader kernel iso qemu

# Directories
BUILD_DIR := build
ISO_DIR := $(BUILD_DIR)/iso
OVMF_DIR := $(BUILD_DIR)/ovmf

# Tools
CARGO := cargo
QEMU := qemu-system-x86_64

# QEMU Flags
QEMU_FLAGS := -m 512M -smp 4 -cpu qemu64
QEMU_UEFI_FLAGS := $(QEMU_FLAGS) -bios $(OVMF_DIR)/OVMF.fd
QEMU_DEBUG_FLAGS := -S -s -serial stdio

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
