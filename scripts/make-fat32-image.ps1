#!/usr/bin/env pwsh
# Create a bootable FAT32 disk image for UEFI

param(
    [string]$Output = "webbos.img",
    [int]$SizeMB = 64
)

$ErrorActionPreference = "Stop"

Write-Host "Creating FAT32 disk image for UEFI boot..."

# Configuration
$bytesPerSector = 512
$sectorsPerCluster = 1
$reservedSectors = 32
$numFATs = 2
$sectorsPerFAT = 256
$rootCluster = 2

$totalSectors = ($SizeMB * 1024 * 1024) / $bytesPerSector
$fatStart = $reservedSectors
$dataStart = $reservedSectors + ($numFATs * $sectorsPerFAT)

# Create image
$sizeBytes = $SizeMB * 1024 * 1024
$img = New-Object byte[] $sizeBytes

# FAT32 Boot Sector
$bootSector = [System.Collections.ArrayList]@()

# Jump instruction
$bootSector.AddRange([byte[]]@(0xEB, 0x58, 0x90))

# OEM Name
$bootSector.AddRange([System.Text.Encoding]::ASCII.GetBytes("MSDOS5.0"))

# Bytes per sector (512)
$bootSector.AddRange([byte[]]@(0x00, 0x02))

# Sectors per cluster
$bootSector.Add(0x01)

# Reserved sectors
$bootSector.AddRange([byte[]]@(0x20, 0x00))

# Number of FATs
$bootSector.Add(0x02)

# Root entries (0 for FAT32)
$bootSector.AddRange([byte[]]@(0x00, 0x00))

# Total sectors (0 for FAT32, use later field)
$bootSector.AddRange([byte[]]@(0x00, 0x00))

# Media descriptor
$bootSector.Add(0xF8)

# Sectors per FAT (0 for FAT32)
$bootSector.AddRange([byte[]]@(0x00, 0x00))

# Sectors per track
$bootSector.AddRange([byte[]]@(0x3F, 0x00))

# Number of heads
$bootSector.AddRange([byte[]]@(0xFF, 0x00))

# Hidden sectors
$bootSector.AddRange([byte[]]@(0x00, 0x00, 0x00, 0x00))

# Total sectors (large)
$totalSectorsBytes = [BitConverter]::GetBytes([uint32]$totalSectors)
$bootSector.AddRange($totalSectorsBytes)

# FAT32 specific fields
# Sectors per FAT
$sectorsPerFATBytes = [BitConverter]::GetBytes([uint32]$sectorsPerFAT)
$bootSector.AddRange($sectorsPerFATBytes)

# Flags
$bootSector.AddRange([byte[]]@(0x00, 0x00))

# FAT version
$bootSector.AddRange([byte[]]@(0x00, 0x00))

# Root cluster
$rootClusterBytes = [BitConverter]::GetBytes([uint32]$rootCluster)
$bootSector.AddRange($rootClusterBytes)

# FSInfo sector
$bootSector.AddRange([byte[]]@(0x01, 0x00))

# Backup boot sector
$bootSector.AddRange([byte[]]@(0x06, 0x00))

# Reserved
$bootSector.AddRange([byte[]]@(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00))

# Drive number
$bootSector.Add(0x80)

# Reserved
$bootSector.Add(0x00)

# Boot signature
$bootSector.Add(0x29)

# Volume serial number
$bootSector.AddRange([byte[]]@(0x01, 0x02, 0x03, 0x04))

# Volume label
$label = [System.Text.Encoding]::ASCII.GetBytes("WEBBOS     ")
$bootSector.AddRange($label)

# File system type
$fsType = [System.Text.Encoding]::ASCII.GetBytes("FAT32   ")
$bootSector.AddRange($fsType)

# Boot code (just infinite loop)
$bootSector.Add(0xF4)  # HLT

# Pad to 510 bytes
while ($bootSector.Count -lt 510) {
    $bootSector.Add(0x00) | Out-Null
}

# Boot signature
$bootSector.Add(0x55)
$bootSector.Add(0xAA)

# Write boot sector
[System.Array]::Copy($bootSector.ToArray(), $img, $bootSector.Count)

# FSInfo sector at sector 1
$fsInfo = [System.Collections.ArrayList]@()
$fsInfo.AddRange([BitConverter]::GetBytes([uint32]0x41615252))  # Signature
$fsInfo.AddRange([byte[]]::new(480))  # Reserved
$fsInfo.AddRange([BitConverter]::GetBytes([uint32]0x61417272))  # Signature
$fsInfo.AddRange([BitConverter]::GetBytes([uint32]0xFFFFFFFF))  # Free clusters
$fsInfo.AddRange([BitConverter]::GetBytes([uint32]0xFFFFFFFF))  # Next free cluster
$fsInfo.AddRange([byte[]]::new(12))  # Reserved
$fsInfo.AddRange([BitConverter]::GetBytes([uint32]0xAA550000))  # Signature

$fsInfoOffset = $bytesPerSector
[System.Array]::Copy($fsInfo.ToArray(), 0, $img, $fsInfoOffset, [Math]::Min($fsInfo.Count, $bytesPerSector))

# FAT1
$fatOffset = $fatStart * $bytesPerSector
# FAT32 media descriptor
[System.Array]::Copy([BitConverter]::GetBytes([uint32]0x0FFFFF8), 0, $img, $fatOffset, 4)
# Root cluster entry
[System.Array]::Copy([BitConverter]::GetBytes([uint32]0x0FFFFFFF), 0, $img, $fatOffset + 8, 4)

# FAT2 (copy of FAT1)
$fat2Offset = ($fatStart + $sectorsPerFAT) * $bytesPerSector
[System.Array]::Copy($img, $fatOffset, $img, $fat2Offset, $sectorsPerFAT * $bytesPerSector)

# Data area starts at cluster 2
$dataOffset = $dataStart * $bytesPerSector

# Create root directory entry for EFI folder
$efiEntry = @(
    0x45, 0x46, 0x49, 0x20, 0x20, 0x20, 0x20, 0x20,  # Name: "EFI     "
    0x20, 0x20, 0x20,                                  # Extension
    0x10,                                              # Attributes: Directory
    0x00,                                              # Reserved
    0x00, 0x00,                                        # Create time
    0x00, 0x00,                                        # Create date
    0x00, 0x00,                                        # Access date
    0x00, 0x00,                                        # High cluster
    0x00, 0x00,                                        # Modify time
    0x00, 0x00,                                        # Modify date
    0x03, 0x00,                                        # Low cluster: 3
    0x00, 0x00, 0x00, 0x00                             # Size
)

[System.Array]::Copy($efiEntry, 0, $img, $dataOffset, $efiEntry.Length)

# Create cluster 3 for EFI directory
$cluster3Offset = $dataOffset + (1 * $bytesPerSector * $sectorsPerCluster)

# Boot entry in EFI directory
$bootEntry = @(
    0x42, 0x4F, 0x4F, 0x54, 0x20, 0x20, 0x20, 0x20,  # Name: "BOOT    "
    0x20, 0x20, 0x20,                                  # Extension
    0x10,                                              # Attributes: Directory
    0x00,                                              # Reserved
    0x00, 0x00,                                        # Create time
    0x00, 0x00,                                        # Create date
    0x00, 0x00,                                        # Access date
    0x00, 0x00,                                        # High cluster
    0x00, 0x00,                                        # Modify time
    0x00, 0x00,                                        # Modify date
    0x04, 0x00,                                        # Low cluster: 4
    0x00, 0x00, 0x00, 0x00                             # Size
)

[System.Array]::Copy($bootEntry, 0, $img, $cluster3Offset, $bootEntry.Length)

# Cluster 4 for BOOT directory - contains BOOTX64.EFI
$cluster4Offset = $dataOffset + (2 * $bytesPerSector * $sectorsPerCluster)

# Read bootloader
$bootloaderBytes = [System.IO.File]::ReadAllBytes("target/x86_64-unknown-uefi/debug/bootloader.efi")
$bootloaderName = [System.Text.Encoding]::ASCII.GetBytes("BOOTX64 EFI")
$bootloaderEntry = @(
    0x42, 0x4F, 0x4F, 0x54, 0x58, 0x36, 0x34, 0x20,  # Name: "BOOTX64 "
    0x45, 0x46, 0x49                                   # Extension: "EFI"
    0x20,                                              # Attributes: Archive
    0x00,                                              # Reserved
    0x00, 0x00,                                        # Create time
    0x00, 0x00,                                        # Create date
    0x00, 0x00,                                        # Access date
    0x00, 0x00,                                        # High cluster
    0x00, 0x00,                                        # Modify time
    0x00, 0x00,                                        # Modify date
    0x05, 0x00                                         # Low cluster: 5
)
$bootloaderEntry += [BitConverter]::GetBytes([uint32]$bootloaderBytes.Length)

[System.Array]::Copy($bootloaderEntry, 0, $img, $cluster4Offset, 32)

# Copy bootloader to cluster 5
$cluster5Offset = $dataOffset + (3 * $bytesPerSector * $sectorsPerCluster)
[System.Array]::Copy($bootloaderBytes, 0, $img, $cluster5Offset, [Math]::Min($bootloaderBytes.Length, 1024 * 1024))

# Copy kernel to cluster 100 (after bootloader space)
$kernelBytes = [System.IO.File]::ReadAllBytes("target/x86_64-unknown-none/debug/kernel")
$kernelOffset = $dataOffset + (98 * $bytesPerSector * $sectorsPerCluster)  # Cluster 100
[System.Array]::Copy($kernelBytes, 0, $img, $kernelOffset, [Math]::Min($kernelBytes.Length, $sizeBytes - $kernelOffset))

# Save image
[System.IO.File]::WriteAllBytes($Output, $img)

Write-Host "Created $Output"
Write-Host "  Bootloader: $($bootloaderBytes.Length) bytes at cluster 5"
Write-Host "  Kernel: $($kernelBytes.Length) bytes at cluster 100"
