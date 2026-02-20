@echo off
REM WebbOS Raspberry Pi QEMU Run Script for Windows
REM Usage: run.bat [raspi3|raspi4]

setlocal EnableDelayedExpansion

set PI_MODEL=%~1
if "%PI_MODEL%"=="" set PI_MODEL=raspi3b

set KERNEL_PATH=target\aarch64-unknown-none\release\kernel
set SD_IMAGE=webbos-pi.img

echo ============================================
echo WebbOS Raspberry Pi QEMU Run Script
echo Model: %PI_MODEL%
echo ============================================
echo.

REM Check for QEMU
where qemu-system-aarch64 >nul 2>&1
if errorlevel 1 (
    echo Error: qemu-system-aarch64 not found in PATH
    echo Please install QEMU and add it to your PATH
    echo.
    echo To install on Windows:
    echo   choco install qemu
    echo   or download from https://www.qemu.org/download/#windows
    exit /b 1
)

REM Check for kernel or SD image
if not exist %KERNEL_PATH% (
    echo Error: Kernel not found at %KERNEL_PATH%
    echo Please build first: build.bat
    exit /b 1
)

REM Set up machine-specific options
if "%PI_MODEL%"=="raspi3" (
    set MACHINE=raspi3b
    set DTB=bcm2710-rpi-3-b-plus.dtb
    set MEM=1G
    set SERIAL=ttyAMA0
) else if "%PI_MODEL%"=="raspi3b" (
    set MACHINE=raspi3b
    set DTB=bcm2710-rpi-3-b-plus.dtb
    set MEM=1G
    set SERIAL=ttyAMA0
) else if "%PI_MODEL%"=="raspi4" (
    set MACHINE=raspi4b
    set DTB=bcm2711-rpi-4-b.dtb
    set MEM=2G
    set SERIAL=ttyAMA0
) else if "%PI_MODEL%"=="raspi4b" (
    set MACHINE=raspi4b
    set DTB=bcm2711-rpi-4-b.dtb
    set MEM=2G
    set SERIAL=ttyAMA0
) else (
    echo Unknown Pi model: %PI_MODEL%
    echo Supported models: raspi3, raspi3b, raspi4, raspi4b
    exit /b 1
)

echo Configuration:
echo   Machine: %MACHINE%
echo   Memory: %MEM%
echo   Kernel: %KERNEL_PATH%
echo   Serial: %SERIAL%
echo.

REM Check if we have device tree blob
set DTB_ARGS=
if exist %DTB% (
    echo Using device tree blob: %DTB%
    set DTB_ARGS=-dtb %DTB%
) else (
    echo Warning: Device tree blob not found: %DTB%
    echo For best results, provide the DTB from Raspberry Pi firmware
)

REM Check if we have SD card image for more complete emulation
set DRIVE_ARGS=
if exist %SD_IMAGE% (
    echo Using SD card image: %SD_IMAGE%
    set DRIVE_ARGS=-drive format=raw,file=%SD_IMAGE%,if=sd
)

echo.
echo Starting QEMU...
echo ============================================
echo.

REM Run QEMU
qemu-system-aarch64 ^
    -M %MACHINE% ^
    -cpu cortex-a53 ^
    -m %MEM% ^
    -kernel %KERNEL_PATH% ^
    %DTB_ARGS% ^
    %DRIVE_ARGS% ^
    -serial stdio ^
    -display none ^
    -no-reboot

REM QEMU exit code
set EXIT_CODE=%errorlevel%

echo.
echo ============================================
if %EXIT_CODE%==0 (
    echo QEMU exited normally
) else (
    echo QEMU exited with code %EXIT_CODE%
)
echo ============================================

exit /b %EXIT_CODE%
