#!/usr/bin/env pwsh
# WebbOS Runner using Alpine Linux as a UEFI template
# This downloads a minimal Linux ISO and replaces the kernel with WebbOS

param(
    [switch]$Debug
)

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  WebbOS Runner (Template Method)          " -ForegroundColor Cyan
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

Write-Host ""
Write-Host "Simplest working method: Direct kernel with serial output" -ForegroundColor Yellow
Write-Host ""
Write-Host "This will attempt to boot the kernel directly." -ForegroundColor Gray
Write-Host "Note: The kernel expects UEFI environment, so this may not fully work," -ForegroundColor Gray
Write-Host "but it should show early boot messages." -ForegroundColor Gray
Write-Host ""
Write-Host "For full functionality, please complete WSL setup:" -ForegroundColor Yellow
Write-Host "  1. Reboot your computer" -ForegroundColor White
Write-Host "  2. Install Ubuntu from Microsoft Store" -ForegroundColor White  
Write-Host "  3. Run: wsl -d Ubuntu -e sudo apt install mtools" -ForegroundColor White
Write-Host "  4. Then run: .\scripts\run-qemu.ps1" -ForegroundColor White
Write-Host ""

# Try direct kernel loading (may show early output)
Write-Host "Attempting direct kernel load..." -ForegroundColor Green
Write-Host ""

$qemuArgs = @(
    "-kernel", $Kernel,
    "-m", "512M",
    "-vga", "std",
    "-serial", "stdio",
    "-no-reboot",
    "-cpu", "qemu64",
    "-machine", "type=q35"
)

if ($Debug) {
    $qemuArgs += @("-s", "-S")
}

& $qemu @qemuArgs

Write-Host ""
Write-Host "Kernel load attempt complete." -ForegroundColor Yellow
Write-Host ""
Write-Host "If you see early boot messages, the kernel started but may need" -ForegroundColor Gray
Write-Host "proper UEFI environment for full functionality." -ForegroundColor Gray
Write-Host ""
Write-Host "To fix this completely, complete the WSL setup as described above." -ForegroundColor Yellow
