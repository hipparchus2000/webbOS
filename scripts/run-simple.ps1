#!/usr/bin/env pwsh
# WebbOS Simple Runner - No UEFI/FAT32 required!
# This uses multiboot2 to load the kernel directly

param(
    [switch]$Debug
)

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "      WebbOS Simple Launcher               " -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Check for kernel
$Kernel = "target/x86_64-unknown-none/debug/kernel"
if (-not (Test-Path $Kernel)) {
    Write-Host "Building kernel..." -ForegroundColor Yellow
    cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
    if ($LASTEXITCODE -ne 0) { exit 1 }
}

# Find QEMU
$qemu = $null
foreach ($path in @("qemu-system-x86_64", "C:\Program Files\qemu\qemu-system-x86_64.exe", "C:\Program Files (x86)\qemu\qemu-system-x86_64.exe")) {
    if (Get-Command $path -ErrorAction SilentlyContinue) { $qemu = $path; break }
    if (Test-Path $path) { $qemu = $path; break }
}

if (-not $qemu) {
    Write-Error "QEMU not found!"
    exit 1
}

Write-Host "Starting WebbOS with direct kernel loading..." -ForegroundColor Green
Write-Host "  Kernel: $Kernel" -ForegroundColor Gray
Write-Host "  Memory: 512M" -ForegroundColor Gray
Write-Host ""
Write-Host "Press Ctrl+C to stop" -ForegroundColor DarkGray
Write-Host ""

# Run QEMU with direct kernel loading (multiboot2)
# This bypasses UEFI entirely!
$qemuArgs = @(
    "-kernel", $Kernel,
    "-m", "512M",
    "-vga", "std",
    "-serial", "stdio",
    "-no-reboot",
    "-append", "console=ttyS0 quiet"
)

if ($Debug) {
    $qemuArgs += @("-s", "-S")
    Write-Host "Debug mode: GDB on port 1234" -ForegroundColor Yellow
}

& $qemu @qemuArgs

Write-Host ""
Write-Host "WebbOS stopped." -ForegroundColor Yellow
