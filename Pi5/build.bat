@echo off
REM WebbOS Raspberry Pi Build Script for Windows
REM Usage: build.bat [release|debug]

setlocal EnableDelayedExpansion

set BUILD_TYPE=release
set CARGO_FLAG=--release
if "%~1"=="debug" set BUILD_TYPE=debug
if "%~1"=="debug" set CARGO_FLAG=

set BOOTLOADER_PATH=target\aarch64-unknown-none\%BUILD_TYPE%\bootloader
set KERNEL_PATH=target\aarch64-unknown-none\%BUILD_TYPE%\kernel
set RAW_IMAGE=webbos-pi-raw.img
set SD_IMAGE=webbos-pi.img

echo ============================================
echo WebbOS Raspberry Pi Build Script
echo Build type: %BUILD_TYPE%
echo ============================================
echo.

REM Check for required tools
echo Checking for required tools...
where cargo >nul 2>&1
if errorlevel 1 (
    echo Error: Rust/Cargo not found in PATH
    exit /b 1
)

where python >nul 2>&1
if errorlevel 1 (
    echo Error: Python not found in PATH
    exit /b 1
)

echo OK
echo.

REM Build bootloader
echo Building bootloader for aarch64...
cargo +nightly-2025-01-15 build -p bootloader --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc %CARGO_FLAG%
if errorlevel 1 (
    echo Bootloader build failed
    exit /b 1
)
echo Bootloader build successful
echo.

REM Build kernel
echo Building kernel for aarch64...
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc %CARGO_FLAG%
if errorlevel 1 (
    echo Kernel build failed
    exit /b 1
)
echo Kernel build successful
echo.

REM Create raw combined image (bootloader + kernel)
echo Creating raw combined image...
python make-raw-image.py %BOOTLOADER_PATH% %KERNEL_PATH% %RAW_IMAGE%
if errorlevel 1 (
    echo Failed to create raw image
    exit /b 1
)
echo Raw image created: %RAW_IMAGE%
echo.

REM Also create SD card image for real hardware
echo Creating SD card image for real hardware...
if exist %SD_IMAGE% (
    echo Updating existing SD card image...
    python scripts\update-sdcard.py %SD_IMAGE% kernel %KERNEL_PATH%
) else (
    echo Creating new SD card image...
    python scripts\create-sdcard.py --output %SD_IMAGE% %BOOTLOADER_PATH%
)
if errorlevel 1 (
    echo Note: SD card image creation may have issues, but raw image is ready
)

echo.
echo ============================================
echo Build complete!
echo.
echo Raw image (for QEMU): %RAW_IMAGE%
echo SD card image (for real Pi): %SD_IMAGE%
echo.
echo To test in QEMU:
echo   run.bat
echo.
echo To write to an SD card:
echo   - Use Raspberry Pi Imager, Rufus, or Etcher
echo   - Or: dd if=%SD_IMAGE% of=\\.\PhysicalDriveN bs=4M
echo ============================================
