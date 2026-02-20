#!/usr/bin/env pwsh
# WebbOS QEMU Runner Script
# Usage: .\scripts\run-qemu.ps1 [-Network] [-Debug] [-Release] [-Rebuild] [-NoGraphic]
#
# Prerequisites:
#   1. Rust with nightly toolchain
#   2. QEMU for x86_64
#   3. Python 3 (for disk image management)
#
# Quick start: python create-image.py (if you don't have webbos.img yet)

param(
    [switch]$Network,
    [switch]$Debug,
    [switch]$Release,
    [switch]$Rebuild,
    [switch]$NoGraphic
)

$ErrorActionPreference = "Stop"

# Configuration
$QEMU = "qemu-system-x86_64"
$OVMF = "OVMF.fd"
$ImageFile = "webbos.img"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "           WebbOS QEMU Launcher             " -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Check if running from correct directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Error "Please run this script from the webbOs root directory"
    exit 1
}

# Build configuration
$BuildType = if ($Release) { "release" } else { "debug" }
$KernelTarget = "target/x86_64-unknown-none/$BuildType/kernel"
$BootloaderTarget = "target/x86_64-unknown-uefi/$BuildType/bootloader.efi"

# Build the project if needed or requested
if ($Rebuild -or -not (Test-Path $KernelTarget) -or -not (Test-Path $BootloaderTarget)) {
    Write-Host "Building WebbOS..." -ForegroundColor Yellow
    Write-Host "  Build type: $BuildType" -ForegroundColor Gray
    
    Write-Host "  Building kernel..." -ForegroundColor Gray
    $kernelArgs = @("+nightly-2025-01-15", "build", "-p", "kernel", "--target", "x86_64-unknown-none", "-Z", "build-std=core,compiler_builtins,alloc")
    if ($Release) { $kernelArgs += "--release" }
    & cargo @kernelArgs
    if ($LASTEXITCODE -ne 0) { exit 1 }
    
    Write-Host "  Building bootloader..." -ForegroundColor Gray
    $bootloaderArgs = @("+nightly-2025-01-15", "build", "-p", "bootloader", "--target", "x86_64-unknown-uefi", "-Z", "build-std=core,compiler_builtins,alloc")
    if ($Release) { $bootloaderArgs += "--release" }
    & cargo @bootloaderArgs
    if ($LASTEXITCODE -ne 0) { exit 1 }
    
    Write-Host "Build complete" -ForegroundColor Green
} else {
    Write-Host "Using existing build" -ForegroundColor Gray
    Write-Host "  Build type: $BuildType" -ForegroundColor DarkGray
    Write-Host "  Use -Rebuild to force rebuild" -ForegroundColor DarkGray
}

# Create or update disk image
if ($Rebuild -or -not (Test-Path $ImageFile)) {
    Write-Host ""
    Write-Host "Creating bootable disk image..." -ForegroundColor Yellow
    
    # Create new image using Python script
    python scripts/create-image.py --output $ImageFile --size 64
    
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to create disk image"
        exit 1
    }
    
    Write-Host "Disk image created" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "Updating disk image..." -ForegroundColor Yellow
    
    # Update bootloader
    python scripts/update-image.py $ImageFile "EFI/BOOT/BOOTX64.EFI" $BootloaderTarget
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to update bootloader"
        exit 1
    }
    
    # Update kernel
    python scripts/update-image.py $ImageFile "kernel.elf" $KernelTarget
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to update kernel"
        exit 1
    }
    
    Write-Host "Disk image updated" -ForegroundColor Green
}

# Download OVMF if needed
if (-not (Test-Path $OVMF)) {
    Write-Host ""
    Write-Host "Downloading OVMF firmware..." -ForegroundColor Yellow
    try {
        Invoke-WebRequest -Uri "https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd" -OutFile $OVMF -UseBasicParsing
        Write-Host "OVMF downloaded" -ForegroundColor Green
    } catch {
        Write-Error "Failed to download OVMF"
        Write-Host "Please download manually from:" -ForegroundColor Yellow
        Write-Host "  https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd" -ForegroundColor Gray
        Write-Host "  Save as: OVMF.fd" -ForegroundColor Gray
        exit 1
    }
}

# Find QEMU
$qemuPath = $null
$qemuPaths = @(
    "qemu-system-x86_64",
    "C:\Program Files\qemu\qemu-system-x86_64.exe",
    "C:\Program Files (x86)\qemu\qemu-system-x86_64.exe"
)

foreach ($path in $qemuPaths) {
    if (Get-Command $path -ErrorAction SilentlyContinue) {
        $qemuPath = $path
        break
    } elseif (Test-Path $path) {
        $qemuPath = $path
        break
    }
}

if (-not $qemuPath) {
    Write-Error "QEMU not found! Please install QEMU first."
    Write-Host "  Download from: https://www.qemu.org/download/#windows" -ForegroundColor Yellow
    exit 1
}

# Build QEMU arguments
$qemuArgs = @(
    "-bios", $OVMF,
    "-drive", "format=raw,file=$ImageFile",
    "-m", "512M",
    "-smp", "2",
    "-vga", "std"
)

if ($NoGraphic) {
    $qemuArgs += "-nographic"
} else {
    $qemuArgs += "-serial", "stdio"
}

if ($Network) {
    Write-Host "Enabling network..." -ForegroundColor Gray
    $qemuArgs += @(
        "-device", "virtio-net-pci,netdev=net0",
        "-netdev", "user,id=net0,hostfwd=tcp::8080-:80"
    )
}

if ($Debug) {
    Write-Host "Debug mode: GDB server on port 1234" -ForegroundColor Gray
    $qemuArgs += @("-s", "-S")
}

Write-Host ""
Write-Host "Launching WebbOS..." -ForegroundColor Cyan
Write-Host "  QEMU: $qemuPath" -ForegroundColor Gray
Write-Host "  Mode: $BuildType" -ForegroundColor Gray
Write-Host "  Image: $ImageFile" -ForegroundColor Gray
Write-Host "  Memory: 512M" -ForegroundColor Gray
Write-Host "  CPUs: 2" -ForegroundColor Gray
if ($Network) { Write-Host "  Network: Enabled (port 8080 -> 80)" -ForegroundColor Gray }
if ($Debug) { Write-Host "  Debug: GDB on :1234" -ForegroundColor Gray }
Write-Host ""
Write-Host "Press Ctrl+C to stop" -ForegroundColor DarkGray
Write-Host ""

# Run QEMU
& $qemuPath @qemuArgs

Write-Host ""
Write-Host "WebbOS stopped." -ForegroundColor Yellow
