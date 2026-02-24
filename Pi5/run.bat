@echo off
REM WebbOS Raspberry Pi QEMU Run Script for Windows
REM Usage: run.bat [raspi3|raspi4]

setlocal EnableDelayedExpansion

set PI_MODEL=%~1
if "%PI_MODEL%"=="" set PI_MODEL=raspi3b

set RAW_IMAGE=webbos-pi-raw.img

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

REM Check for raw image
if not exist %RAW_IMAGE% (
    echo Error: Raw image not found: %RAW_IMAGE%
    echo Please build first: build.bat
    exit /b 1
)

REM Set up machine-specific options
if "%PI_MODEL%"=="raspi3" (
    set MACHINE=raspi3b
    set MEM=1G
) else if "%PI_MODEL%"=="raspi3b" (
    set MACHINE=raspi3b
    set MEM=1G
) else if "%PI_MODEL%"=="raspi4" (
    set MACHINE=raspi4b
    set MEM=2G
) else if "%PI_MODEL%"=="raspi4b" (
    set MACHINE=raspi4b
    set MEM=2G
) else (
    echo Unknown Pi model: %PI_MODEL%
    echo Supported models: raspi3, raspi3b, raspi4, raspi4b
    exit /b 1
)

echo Configuration:
echo   Machine: %MACHINE%
echo   Memory: %MEM%
echo   Image: %RAW_IMAGE%
echo.

REM Check for available display options
echo Checking display options...
where xvfb-run >nul 2>&1
if errorlevel 1 (
    echo Display: SDL (GUI window)
    set DISPLAY_OPTS=-display sdl
) else (
    echo Display: SDL (GUI window)
    set DISPLAY_OPTS=-display sdl
)

echo.
echo Starting QEMU...
echo ============================================
echo.
echo NOTE: The Pi version uses mailbox framebuffer interface
echo which is not fully emulated in QEMU. The OS is running
echo but display output may not be visible.
echo.
echo For a working desktop display, use the PC version:
echo   cd PC && run.bat
echo.
echo Press Ctrl+C to stop QEMU
echo.

REM Run QEMU
qemu-system-aarch64 ^
    -M %MACHINE% ^
    -cpu cortex-a53 ^
    -m %MEM% ^
    -kernel %RAW_IMAGE% ^
    %DISPLAY_OPTS% ^
    -device usb-kbd ^
    -device usb-mouse ^
    -snapshot ^
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
