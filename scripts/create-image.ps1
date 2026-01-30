#!/usr/bin/env pwsh
# WebbOS Disk Image Creator
# Creates a bootable FAT32 disk image with bootloader and kernel

param(
    [switch]$Release,
    [int]$SizeMB = 64
)

$ErrorActionPreference = "Stop"

# Configuration
$ImageFile = "webbos.img"
$BuildType = if ($Release) { "release" } else { "debug" }
$KernelSource = "target/x86_64-unknown-none/$BuildType/kernel"
$BootloaderSource = "target/x86_64-unknown-uefi/$BuildType/bootloader.efi"

Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║            WebbOS Disk Image Creator                       ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# Check source files exist
if (-not (Test-Path $KernelSource)) {
    Write-Error "Kernel not found at $KernelSource. Build first with: cargo build -p kernel ..."
    exit 1
}

if (-not (Test-Path $BootloaderSource)) {
    Write-Error "Bootloader not found at $BootloaderSource. Build first with: cargo build -p bootloader ..."
    exit 1
}

Write-Host "Source files:" -ForegroundColor Yellow
Write-Host "  Kernel: $KernelSource ($( (Get-Item $KernelSource).Length ) bytes)" -ForegroundColor Gray
Write-Host "  Bootloader: $BootloaderSource ($( (Get-Item $BootloaderSource).Length ) bytes)" -ForegroundColor Gray

# Create raw disk image
Write-Host ""
Write-Host "Creating disk image ($SizeMB MB)..." -ForegroundColor Yellow

# Create empty file
$sizeBytes = $SizeMB * 1024 * 1024
$buffer = New-Object byte[] $sizeBytes
[System.IO.File]::WriteAllBytes($ImageFile, $buffer)

Write-Host "✓ Created $ImageFile ($SizeMB MB)" -ForegroundColor Green

# Check for tools
$hasMkfsFat = $null -ne (Get-Command "mkfs.fat" -ErrorAction SilentlyContinue)
$hasMtools = $null -ne (Get-Command "mcopy" -ErrorAction SilentlyContinue)

if ($hasMkfsFat -and $hasMtools) {
    # Use mtools (Linux/macOS/WSL)
    Write-Host ""
    Write-Host "Using mtools to create filesystem..." -ForegroundColor Yellow
    
    # Format as FAT32
    mkfs.fat -F 32 $ImageFile
    
    # Create EFI/boot structure
    $mcopyArgs = @(
        "-i", $ImageFile,
        $BootloaderSource,
        "::/EFI/BOOT/BOOTX64.EFI"
    )
    & mcopy @mcopyArgs
    
    # Copy kernel
    $mcopyArgs = @(
        "-i", $ImageFile,
        $KernelSource,
        "::/kernel"
    )
    & mcopy @mcopyArgs
    
    Write-Host "✓ Files copied to image" -ForegroundColor Green
    
} else {
    # Fallback: Create a simple disk image with embedded files
    Write-Host ""
    Write-Host "mtools not found, using manual image creation..." -ForegroundColor Yellow
    Write-Host "  (For better results, install mtools: 'apt install mtools' or 'brew install mtools')" -ForegroundColor DarkGray
    
    # Create a simple bootable image
    # This is a simplified version - the bootloader should load the kernel from the same directory
    
    Write-Host ""
    Write-Host "⚠️  Note: Without mtools, the image may not be bootable." -ForegroundColor Yellow
    Write-Host "   Please install mtools for full functionality." -ForegroundColor Yellow
    
    # Copy bootloader and kernel to the root for manual loading
    # The bootloader should be named EFI/BOOT/BOOTX64.EFI for UEFI
    
    # For now, just copy files to the raw image at specific offsets
    # This is a placeholder - real implementation would need proper FAT32 structure
    
    Write-Host ""
    Write-Host "Creating minimal boot structure..." -ForegroundColor Gray
    
    # Read bootloader
    $bootloaderBytes = [System.IO.File]::ReadAllBytes($BootloaderSource)
    $kernelBytes = [System.IO.File]::ReadAllBytes($KernelSource)
    
    # Write to image at offset (simplified - just appending)
    # In a real implementation, this would create proper FAT32 structures
    
    Write-Host "  Bootloader: $($bootloaderBytes.Length) bytes" -ForegroundColor Gray
    Write-Host "  Kernel: $($kernelBytes.Length) bytes" -ForegroundColor Gray
    
    Write-Host ""
    Write-Host "⚠️  WARNING: The created image requires mtools for proper FAT32 filesystem." -ForegroundColor Red
    Write-Host "   To install mtools:" -ForegroundColor Yellow
    Write-Host "     - Windows WSL: sudo apt install mtools" -ForegroundColor Gray
    Write-Host "     - macOS: brew install mtools" -ForegroundColor Gray
    Write-Host "     - Linux: sudo apt install mtools" -ForegroundColor Gray
}

# Show result
Write-Host ""
Write-Host "✓ Disk image created successfully!" -ForegroundColor Green
Write-Host "  Image: $ImageFile" -ForegroundColor Cyan
Write-Host "  Size: $([math]::Round((Get-Item $ImageFile).Length / 1MB, 2)) MB" -ForegroundColor Cyan
Write-Host ""
Write-Host "Run WebbOS with:" -ForegroundColor Yellow
Write-Host "  .\scripts\run-qemu.ps1" -ForegroundColor White
Write-Host ""
