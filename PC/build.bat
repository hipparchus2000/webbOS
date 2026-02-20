@echo off
REM WebbOS Build Script for Windows
REM Usage: build.bat [release]

set BUILD_TYPE=debug
set CARGO_FLAG=
if "%~1"=="release" set BUILD_TYPE=release
if "%~1"=="release" set CARGO_FLAG=--release
if "%~1"=="--release" set BUILD_TYPE=release
if "%~1"=="--release" set CARGO_FLAG=--release

set BOOTLOADER_PATH=target/x86_64-unknown-uefi/%BUILD_TYPE%/bootloader.efi
set KERNEL_PATH=target/x86_64-unknown-none/%BUILD_TYPE%/kernel

echo ============================================
echo WebbOS Build Script
echo Build type: %BUILD_TYPE%
echo ============================================
echo.

REM Build kernel
echo Building kernel...
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc %CARGO_FLAG%
if errorlevel 1 (
    echo Kernel build failed
    exit /b 1
)

REM Build bootloader
echo Building bootloader...
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc %CARGO_FLAG%
if errorlevel 1 (
    echo Bootloader build failed
    exit /b 1
)

REM Backup existing image if present
if exist webbos.img (
    echo Backing up existing disk image...
    copy /Y webbos.img webbos.img.bak >nul
)

REM Create disk image with system files and built binaries
echo.
echo Creating disk image with system files...
python scripts\create-image.py --size 64 --output webbos.img "%BOOTLOADER_PATH%" "%KERNEL_PATH%"
if errorlevel 1 (
    echo Failed to create disk image
    echo Restoring backup...
    if exist webbos.img.bak (
        move /Y webbos.img.bak webbos.img >nul
    )
    exit /b 1
)

REM Remove backup on success
if exist webbos.img.bak (
    del webbos.img.bak
)

echo.
echo ============================================
echo Build complete!
echo Run: qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -m 128M -smp 1 -nographic -serial stdio
echo ============================================
