#!/usr/bin/env pwsh
# WebbOS WSL Setup and Run Script
# Run this as Administrator: Right-click -> "Run with PowerShell"

param(
    [switch]$SkipBuild,
    [switch]$SkipImage
)

#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"

function Write-Header($text) {
    Write-Host ""
    Write-Host "============================================" -ForegroundColor Cyan
    Write-Host $text -ForegroundColor Cyan
    Write-Host "============================================" -ForegroundColor Cyan
    Write-Host ""
}

function Write-Success($text) {
    Write-Host "[OK] $text" -ForegroundColor Green
}

function Write-Error($text) {
    Write-Host "[ERROR] $text" -ForegroundColor Red
}

function Write-Info($text) {
    Write-Host $text -ForegroundColor Gray
}

# Change to webbOs directory
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$webbOsDir = Resolve-Path (Join-Path $scriptDir "..")
Set-Location $webbOsDir

Write-Header "WebbOS WSL Setup and Launcher"
Write-Info "Working directory: $webbOsDir"

# ============================================
# Step 1: Check/Install WSL
# ============================================
Write-Header "Step 1: Checking WSL Installation"

try {
    $wslStatus = wsl --status 2>&1
    Write-Success "WSL is already installed"
    Write-Info $wslStatus
} catch {
    Write-Host "WSL is not installed. Installing now..." -ForegroundColor Yellow
    Write-Host "This may take several minutes..." -ForegroundColor Yellow
    Write-Host ""
    
    # Enable WSL
    Write-Info "Enabling WSL feature..."
    dism.exe /online /enable-feature /featurename:Microsoft-Windows-Subsystem-for-Linux /all /norestart
    
    # Enable Virtual Machine Platform
    Write-Info "Enabling Virtual Machine Platform..."
    dism.exe /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart
    
    # Set WSL2 as default
    Write-Info "Setting WSL2 as default..."
    wsl --set-default-version 2
    
    Write-Success "WSL features enabled"
    Write-Host ""
    Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║  IMPORTANT: YOU MUST RESTART YOUR COMPUTER NOW!           ║" -ForegroundColor Red
    Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Red
    Write-Host ""
    Write-Host "After restarting, run this script again to complete setup." -ForegroundColor Yellow
    Write-Host ""
    $response = Read-Host "Restart now? (Y/N)"
    if ($response -eq 'Y' -or $response -eq 'y') {
        Restart-Computer
    } else {
        Write-Host "Please restart manually and run this script again." -ForegroundColor Yellow
        exit 0
    }
}

# ============================================
# Step 2: Check/Install Ubuntu
# ============================================
Write-Header "Step 2: Checking Ubuntu Installation"

$distros = wsl --list --quiet 2>&1
if ($distros -match "Ubuntu") {
    Write-Success "Ubuntu is already installed"
} else {
    Write-Host "Ubuntu not found. Installing..." -ForegroundColor Yellow
    Write-Host "This will take 5-10 minutes..." -ForegroundColor Yellow
    Write-Host ""
    
    try {
        wsl --install -d Ubuntu --no-launch
        Write-Success "Ubuntu installation initiated"
        Write-Host ""
        Write-Host "IMPORTANT: Please complete the Ubuntu setup." -ForegroundColor Yellow
        Write-Host "You'll need to create a username and password." -ForegroundColor Yellow
        Write-Host ""
        
        # Launch Ubuntu for initial setup
        wsl -d Ubuntu
        
        Write-Host ""
        Write-Host "Press Enter after Ubuntu setup is complete..."
        Read-Host
    } catch {
        Write-Error "Failed to install Ubuntu automatically"
        Write-Host "Please install Ubuntu manually from Microsoft Store" -ForegroundColor Yellow
        Start-Process "ms-windows-store://search/?query=Ubuntu"
        exit 1
    }
}

# ============================================
# Step 3: Install mtools
# ============================================
Write-Header "Step 3: Installing Required Tools"

Write-Info "Installing mtools in WSL..."
$mtoolsCheck = wsl -d Ubuntu -e bash -c "which mcopy" 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Success "mtools already installed"
} else {
    wsl -d Ubuntu -e bash -c "sudo apt update && sudo apt install -y mtools"
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to install mtools"
        exit 1
    }
    Write-Success "mtools installed"
}

# ============================================
# Step 4: Build WebbOS
# ============================================
if (-not $SkipBuild) {
    Write-Header "Step 4: Building WebbOS"
    
    $kernelExists = Test-Path "target/x86_64-unknown-none/debug/kernel"
    $bootloaderExists = Test-Path "target/x86_64-unknown-uefi/debug/bootloader.efi"
    
    if (-not $kernelExists -or -not $bootloaderExists) {
        Write-Info "Building kernel..."
        cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
        if ($LASTEXITCODE -ne 0) { exit 1 }
        
        Write-Info "Building bootloader..."
        cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
        if ($LASTEXITCODE -ne 0) { exit 1 }
        
        Write-Success "Build complete"
    } else {
        Write-Success "Kernel and bootloader already built (use -SkipBuild to force rebuild)"
    }
} else {
    Write-Header "Step 4: Building WebbOS (Skipped)"
}

# ============================================
# Step 5: Create Disk Image
# ============================================
if (-not $SkipImage) {
    Write-Header "Step 5: Creating Bootable Disk Image"
    
    # Remove old image
    Remove-Item -Force webbos.img -ErrorAction SilentlyContinue
    
    # Create disk image using WSL
    Write-Info "Creating 64MB FAT32 disk image..."
    $username = $env:USERNAME
    
    wsl -d Ubuntu -e bash -c @"
cd /mnt/c/Users/$username/src/webbOs
rm -f webbos.img
echo 'Creating empty image...'
dd if=/dev/zero of=webbos.img bs=1M count=64 status=progress
echo 'Formatting as FAT32...'
mkfs.fat -F 32 webbos.img
echo 'Creating directories...'
mmd -i webbos.img ::/EFI
mmd -i webbos.img ::/EFI/BOOT
echo 'Copying bootloader...'
mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI
echo 'Copying kernel...'
mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel
echo 'Done! Contents:'
mdir -i webbos.img -s ::
"@
    
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to create disk image"
        exit 1
    }
    
    Write-Success "Disk image created"
} else {
    Write-Header "Step 5: Creating Disk Image (Skipped)"
}

# ============================================
# Step 6: Download OVMF
# ============================================
Write-Header "Step 6: Checking OVMF Firmware"

if (Test-Path "OVMF.fd") {
    Write-Success "OVMF firmware already exists"
} else {
    Write-Info "Downloading OVMF firmware..."
    try {
        Invoke-WebRequest -Uri "https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd" -OutFile "OVMF.fd" -UseBasicParsing
        Write-Success "OVMF downloaded"
    } catch {
        Write-Error "Failed to download OVMF"
        Write-Host "Please download manually from:" -ForegroundColor Yellow
        Write-Host "https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd" -ForegroundColor Yellow
        exit 1
    }
}

# ============================================
# Step 7: Run WebbOS
# ============================================
Write-Header "Step 7: Running WebbOS"

# Find QEMU
$qemu = $null
$qemuPaths = @(
    "qemu-system-x86_64",
    "C:\Program Files\qemu\qemu-system-x86_64.exe",
    "C:\Program Files (x86)\qemu\qemu-system-x86_64.exe"
)

foreach ($path in $qemuPaths) {
    if (Get-Command $path -ErrorAction SilentlyContinue) {
        $qemu = $path
        break
    } elseif (Test-Path $path) {
        $qemu = $path
        break
    }
}

if (-not $qemu) {
    Write-Error "QEMU not found! Please install from https://www.qemu.org/download/#windows"
    exit 1
}

Write-Success "Found QEMU: $qemu"
Write-Host ""
Write-Info "Configuration:"
Write-Info "  Disk: webbos.img"
Write-Info "  Memory: 512M"
Write-Info "  CPUs: 2"
Write-Info "  Graphics: std (1024x768)"
Write-Info "  Network: Enabled (port 8080 forwarded)"
Write-Host ""
Write-Host "Press Ctrl+C to stop WebbOS" -ForegroundColor Yellow
Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Run QEMU
& $qemu `
    -bios OVMF.fd `
    -drive format=raw,file=webbos.img,if=virtio `
    -vga std `
    -m 512M `
    -smp 2 `
    -serial stdio `
    -device virtio-net-pci,netdev=net0 `
    -netdev user,id=net0,hostfwd=tcp::8080-:80

Write-Host ""
Write-Header "WebbOS has stopped"
Write-Host ""
Write-Host "To run again, use: .\scripts\run-qemu.ps1" -ForegroundColor Green
Write-Host ""
Read-Host -Prompt "Press Enter to exit"
