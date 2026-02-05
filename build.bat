@echo off
REM WebbOS Build Script for Windows
REM Usage: build.bat [release]

set BUILD_TYPE=debug
if "%~1"=="release" set BUILD_TYPE=release
if "%~1"=="--release" set BUILD_TYPE=release

echo ============================================
echo WebbOS Build Script
echo Build type: %BUILD_TYPE%
echo ============================================
echo.

REM Check if disk image exists, create if not
if not exist webbos.img (
    echo Creating disk image...
    python scripts\create-image.py --size 64
    if errorlevel 1 (
        echo Failed to create disk image
        exit /b 1
    )
)

REM Build kernel
echo Building kernel...
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
if errorlevel 1 (
    echo Kernel build failed
    exit /b 1
)

REM Build bootloader
echo Building bootloader...
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
if errorlevel 1 (
    echo Bootloader build failed
    exit /b 1
)

REM Update disk image
echo Updating disk image...
python scripts\update-image.py webbos.img "EFI/BOOT/BOOTX64.EFI" target/x86_64-unknown-uefi/debug/bootloader.efi
if errorlevel 1 (
    echo Failed to update bootloader
    exit /b 1
)

python scripts\update-image.py webbos.img kernel.elf target/x86_64-unknown-none/debug/kernel
if errorlevel 1 (
    echo Failed to update kernel
    exit /b 1
)

echo.
echo ============================================
echo Build complete!
echo Run: qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
echo ============================================
