@echo off
REM WebbOS Raspberry Pi Build Script for Windows
REM Usage: build.bat [release|debug]

setlocal EnableDelayedExpansion

set BUILD_TYPE=release
set CARGO_FLAG=--release
if "%~1"=="debug" set BUILD_TYPE=debug
if "%~1"=="debug" set CARGO_FLAG=

set KERNEL_PATH=target\aarch64-unknown-none\%BUILD_TYPE%\kernel
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

REM Build kernel
echo Building kernel for aarch64...
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc %CARGO_FLAG%
if errorlevel 1 (
    echo Kernel build failed
    exit /b 1
)
echo Kernel build successful
echo.

REM Check if SD card image exists
if exist %SD_IMAGE% (
    echo Existing SD card image found, updating kernel...
    python scripts\update-sdcard.py %SD_IMAGE% kernel %KERNEL_PATH%
    if errorlevel 1 (
        echo Failed to update kernel in SD card image
        exit /b 1
    )
) else (
    echo Creating new SD card image...
    python scripts\create-sdcard.py --output %SD_IMAGE% %KERNEL_PATH%
    if errorlevel 1 (
        echo Failed to create SD card image
        exit /b 1
    )
)

echo.
echo ============================================
echo Build complete!
echo.
echo SD card image: %SD_IMAGE%
echo.
echo To write to an SD card:
echo   - Use Raspberry Pi Imager, Rufus, or Etcher
echo   - Or: dd if=%SD_IMAGE% of=\\.\PhysicalDriveN bs=4M
echo.
echo To test in QEMU:
echo   run.bat
echo ============================================
