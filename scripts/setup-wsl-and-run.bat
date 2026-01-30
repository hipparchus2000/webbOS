@echo off
:: WebbOS WSL Setup and Run Script
:: This script sets up WSL, installs Ubuntu, creates a proper bootable disk image, and runs WebbOS
:: 
:: IMPORTANT: Run this script as Administrator!
:: Right-click -> "Run as administrator"

setlocal EnableDelayedExpansion

echo ============================================
echo     WebbOS WSL Setup and Launcher
echo ============================================
echo.

:: Check for administrator privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo ERROR: This script must be run as Administrator!
    echo.
    echo Please right-click on this file and select "Run as administrator"
    echo.
    pause
    exit /b 1
)

echo [OK] Running with administrator privileges
echo.

:: Change to script directory
cd /d "%~dp0"
cd ..
set "WEBBOS_DIR=%CD%"
echo Working directory: %WEBBOS_DIR%
echo.

:: ============================================
:: Step 1: Check/Install WSL
:: ============================================
echo ============================================
echo Step 1: Checking WSL Installation
echo ============================================
echo.

wsl --status >nul 2>&1
if %errorLevel% equ 0 (
    echo [OK] WSL is already installed
    goto :CHECK_UBUNTU
) else (
    echo WSL is not installed. Installing now...
    echo This may take several minutes...
    echo.
    
    :: Enable WSL feature
    echo Enabling WSL feature...
    dism.exe /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart
    if %errorLevel% neq 0 (
        echo [ERROR] Failed to enable WSL feature
        pause
        exit /b 1
    )
    
    :: Enable Virtual Machine Platform
    echo Enabling Virtual Machine Platform...
    dism.exe /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart
    if %errorLevel% neq 0 (
        echo [ERROR] Failed to enable Virtual Machine Platform
        pause
        exit /b 1
    )
    
    :: Set WSL default version to 2
    echo Setting WSL default version to 2...
    wsl --set-default-version 2
    
    echo.
    echo [OK] WSL features enabled
    echo ============================================
    echo  IMPORTANT: YOU MUST RESTART YOUR COMPUTER!
    echo ============================================
    echo.
    echo After restarting, run this script again.
    echo.
    choice /C YN /M "Restart now?"
    if errorlevel 2 exit /b 0
    if errorlevel 1 shutdown /r /t 0
)

:: ============================================
:: Step 2: Check/Install Ubuntu
:: ============================================
:CHECK_UBUNTU
echo ============================================
echo Step 2: Checking Ubuntu Installation
echo ============================================
echo.

wsl --list --quiet | findstr /I "Ubuntu" >nul
if %errorLevel% equ 0 (
    echo [OK] Ubuntu is already installed
    goto :CHECK_TOOLS
) else (
    echo Ubuntu not found. Installing Ubuntu...
    echo This will take several minutes...
    echo.
    
    :: Install Ubuntu from Microsoft Store (silently)
    echo Installing Ubuntu (this may take 5-10 minutes)...
    wsl --install -d Ubuntu --no-launch
    
    if %errorLevel% neq 0 (
        echo [WARNING] Automated install failed. Trying alternative method...
        echo.
        echo Please install Ubuntu manually:
        echo 1. Open Microsoft Store
        echo 2. Search for "Ubuntu"
        echo 3. Click "Get" or "Install"
        echo 4. After installation, run this script again
        echo.
        start ms-windows-store://search/?query=Ubuntu
        pause
        exit /b 1
    )
    
    echo.
    echo [OK] Ubuntu installation initiated
    echo.
    echo IMPORTANT: Please complete the Ubuntu setup when prompted.
    echo You'll need to create a username and password.
    echo.
    wsl -d Ubuntu
    echo.
    echo After Ubuntu setup is complete, press any key to continue...
    pause >nul
)

:: ============================================
:: Step 3: Install Required Tools
:: ============================================
:CHECK_TOOLS
echo ============================================
echo Step 3: Installing Required Tools
echo ============================================
echo.

echo Updating package lists and installing mtools...
wsl -d Ubuntu -e bash -c "sudo apt update && sudo apt install -y mtools"

if %errorLevel% neq 0 (
    echo [ERROR] Failed to install mtools
    echo Please run this command manually in WSL:
    echo   sudo apt update ^&^& sudo apt install -y mtools
    pause
    exit /b 1
)

echo.
echo [OK] mtools installed successfully
echo.

:: ============================================
:: Step 4: Build WebbOS (if needed)
:: ============================================
:BUILD
echo ============================================
echo Step 4: Building WebbOS
echo ============================================
echo.

if not exist "%WEBBOS_DIR%\target\x86_64-unknown-none\debug\kernel" (
    echo Building kernel...
    cd /d "%WEBBOS_DIR%"
    cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
    if %errorLevel% neq 0 (
        echo [ERROR] Kernel build failed
        pause
        exit /b 1
    )
) else (
    echo [OK] Kernel already built
)

if not exist "%WEBBOS_DIR%\target\x86_64-unknown-uefi\debug\bootloader.efi" (
    echo Building bootloader...
    cd /d "%WEBBOS_DIR%"
    cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
    if %errorLevel% neq 0 (
        echo [ERROR] Bootloader build failed
        pause
        exit /b 1
    )
) else (
    echo [OK] Bootloader already built
)

echo.
echo [OK] Build complete
echo.

:: ============================================
:: Step 5: Create Disk Image
:: ============================================
:CREATE_IMAGE
echo ============================================
echo Step 5: Creating Bootable Disk Image
echo ============================================
echo.

cd /d "%WEBBOS_DIR%"

:: Delete old image
del /f webbos.img 2>nul

:: Create disk image using WSL
echo Creating 64MB FAT32 disk image...
wsl -d Ubuntu -e bash -c "
cd /mnt/c/Users/%USERNAME%/src/webbOs
rm -f webbos.img
dd if=/dev/zero of=webbos.img bs=1M count=64
mkfs.fat -F 32 webbos.img
echo 'Disk image created, copying files...'
"

if %errorLevel% neq 0 (
    echo [ERROR] Failed to create disk image
    pause
    exit /b 1
)

:: Create directory structure and copy files
echo Creating EFI directory structure...
wsl -d Ubuntu -e bash -c "
cd /mnt/c/Users/%USERNAME%/src/webbOs
mmd -i webbos.img ::/EFI
mmd -i webbos.img ::/EFI/BOOT
mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel
echo 'Files copied successfully'
mdir -i webbos.img -s ::
"

if %errorLevel% neq 0 (
    echo [ERROR] Failed to copy files to disk image
    pause
    exit /b 1
)

echo.
echo [OK] Disk image created successfully
echo.

:: ============================================
:: Step 6: Download OVMF (if needed)
:: ============================================
:DOWNLOAD_OVMF
echo ============================================
echo Step 6: Checking OVMF Firmware
echo ============================================
echo.

if exist "OVMF.fd" (
    echo [OK] OVMF firmware already exists
    goto :RUN
)

echo Downloading OVMF firmware...
powershell -Command "Invoke-WebRequest -Uri 'https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd' -OutFile 'OVMF.fd' -UseBasicParsing"

if %errorLevel% neq 0 (
    echo [WARNING] Failed to download OVMF automatically
    echo Please download manually from:
    echo   https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd
    echo And save it as OVMF.fd in the webbOs directory
    pause
    exit /b 1
)

echo [OK] OVMF firmware downloaded
echo.

:: ============================================
:: Step 7: Run WebbOS
:: ============================================
:RUN
echo ============================================
echo Step 7: Running WebbOS
echo ============================================
echo.

cd /d "%WEBBOS_DIR%"

echo Configuration:
echo   Disk: webbos.img
echo   Memory: 512M
echo   CPUs: 2
echo   Graphics: std (1024x768)
echo.
echo Press Ctrl+C to stop WebbOS
echo.
echo ============================================
echo.

:: Check if QEMU is in PATH
where qemu-system-x86_64 >nul 2>&1
if %errorLevel% equ 0 (
    set "QEMU=qemu-system-x86_64"
) else if exist "C:\Program Files\qemu\qemu-system-x86_64.exe" (
    set "QEMU=C:\Program Files\qemu\qemu-system-x86_64.exe"
) else if exist "C:\Program Files (x86)\qemu\qemu-system-x86_64.exe" (
    set "QEMU=C:\Program Files (x86)\qemu\qemu-system-x86_64.exe"
) else (
    echo [ERROR] QEMU not found!
    echo Please install QEMU from https://www.qemu.org/download/#windows
    pause
    exit /b 1
)

echo Starting QEMU: %QEMU%
echo.

:: Run WebbOS
"%QEMU%" ^
    -bios OVMF.fd ^
    -drive format=raw,file=webbos.img,if=virtio ^
    -vga std ^
    -m 512M ^
    -smp 2 ^
    -serial stdio ^
    -device virtio-net-pci,netdev=net0 ^
    -netdev user,id=net0,hostfwd=tcp::8080-:80

echo.
echo ============================================
echo WebbOS has stopped
echo ============================================
echo.

:: ============================================
:: Cleanup and Exit
:: ============================================
:CLEANUP
echo.
echo To run WebbOS again, just run this script again.
echo.
pause
exit /b 0
