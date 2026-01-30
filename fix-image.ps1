# Create disk image with FAT32 using PowerShell and Windows formatting tools
$imgFile = "webbos.img"
$sizeMB = 64

# Remove old file
Remove-Item $imgFile -ErrorAction SilentlyContinue

# Create file
$sizeBytes = $sizeMB * 1024 * 1024
$buffer = New-Object byte[] $sizeBytes
[System.IO.File]::WriteAllBytes($imgFile, $buffer)

Write-Host "Created $imgFile (${sizeMB}MB)"
Write-Host ""
Write-Host "To make this bootable, you need to:"
Write-Host "1. Mount this as a loopback device (requires administrator)"
Write-Host "2. Format as FAT32"
Write-Host "3. Copy bootloader to EFI/BOOT/BOOTX64.EFI"
Write-Host "4. Copy kernel to /kernel"
Write-Host ""
Write-Host "On Windows with WSL:"
Write-Host "  wsl -d Ubuntu"
Write-Host "  cd /mnt/c/users/hippa/src/webbOs"
Write-Host "  sudo apt install mtools"
Write-Host "  mformat -i webbos.img -F ::"
Write-Host "  mmd -i webbos.img ::/EFI"
Write-Host "  mmd -i webbos.img ::/EFI/BOOT"
Write-Host "  mcopy -i webbos.img target/x86_64-unknown-uefi/debug/bootloader.efi ::/EFI/BOOT/BOOTX64.EFI"
Write-Host "  mcopy -i webbos.img target/x86_64-unknown-none/debug/kernel ::/kernel"
