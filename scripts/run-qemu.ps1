#!/usr/bin/env pwsh
# WebbOS QEMU Runner Script
# Usage: .\run-qemu.ps1 [-Network] [-Debug] [-Release]

param(
    [switch]$Network,
    [switch]$Debug,
    [switch]$Release,
    [switch]$NoGraphic
)

$ErrorActionPreference = "Stop"

# Configuration
$QEMU = "qemu-system-x86_64"
$OVMF = "OVMF.fd"
$ImageFile = "webbos.img"

# Build configuration
$BuildType = if ($Release) { "release" } else { "debug" }
$KernelTarget = "target/x86_64-unknown-none/$BuildType/kernel"
$BootloaderTarget = "target/x86_64-unknown-uefi/$BuildType/bootloader.efi"

Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                WebbOS QEMU Launcher                        ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# Check if QEMU is installed
$qemuPath = Get-Command $QEMU -ErrorAction SilentlyContinue
if (-not $qemuPath) {
    Write-Error "QEMU not found! Please install QEMU first."
    Write-Host "  Windows: choco install qemu"
    Write-Host "  Linux:   sudo apt install qemu-system-x86"
    Write-Host "  macOS:   brew install qemu"
    exit 1
}

Write-Host "✓ QEMU found: $($qemuPath.Source)" -ForegroundColor Green

# Build the project if needed
Write-Host ""
Write-Host "Building WebbOS ($BuildType mode)..." -ForegroundColor Yellow

$kernelExists = Test-Path $KernelTarget
$bootloaderExists = Test-Path $BootloaderTarget

if (-not $kernelExists -or -not $bootloaderExists) {
    Write-Host "  Building kernel..." -ForegroundColor Gray
    cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc $(if ($Release) { "--release" })
    
    Write-Host "  Building bootloader..." -ForegroundColor Gray
    cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc $(if ($Release) { "--release" })
}

Write-Host "✓ Build complete" -ForegroundColor Green

# Create disk image if needed
if (-not (Test-Path $ImageFile)) {
    Write-Host ""
    Write-Host "Creating disk image..." -ForegroundColor Yellow
    .\scripts\create-image.ps1 -Release:$Release
}

# Download OVMF if needed
if (-not (Test-Path $OVMF)) {
    Write-Host ""
    Write-Host "Downloading OVMF (UEFI firmware)..." -ForegroundColor Yellow
    $url = "https://github.com/retrage/edk2-nightly/raw/master/bin/RELEASEX64_OVMF.fd"
    try {
        Invoke-WebRequest -Uri $url -OutFile $OVMF -UseBasicParsing
        Write-Host "✓ OVMF downloaded" -ForegroundColor Green
    } catch {
        Write-Warning "Failed to download OVMF. Trying alternative..."
        # Create a dummy file for now - user should install OVMF manually
        Write-Host "Please install OVMF manually:"
        Write-Host "  - Download from: $url"
        Write-Host "  - Save as: $OVMF"
        exit 1
    }
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
    Write-Host "  Enabling network..." -ForegroundColor Gray
    $qemuArgs += @(
        "-netdev", "user,id=net0,hostfwd=tcp::8080-:80",
        "-device", "virtio-net-pci,netdev=net0"
    )
}

if ($Debug) {
    Write-Host "  Debug mode: GDB server on port 1234" -ForegroundColor Gray
    $qemuArgs += @("-s", "-S")
}

Write-Host ""
Write-Host "Launching WebbOS..." -ForegroundColor Cyan
Write-Host "  Mode: $BuildType" -ForegroundColor Gray
Write-Host "  Image: $ImageFile" -ForegroundColor Gray
Write-Host "  Memory: 512M" -ForegroundColor Gray
Write-Host "  CPUs: 2" -ForegroundColor Gray
if ($Network) { Write-Host "  Network: Enabled" -ForegroundColor Gray }
if ($Debug) { Write-Host "  Debug: GDB on :1234" -ForegroundColor Gray }
Write-Host ""
Write-Host "Press Ctrl+C to stop" -ForegroundColor DarkGray
Write-Host ""

# Run QEMU
& $QEMU @qemuArgs

Write-Host ""
Write-Host "WebbOS stopped." -ForegroundColor Yellow
