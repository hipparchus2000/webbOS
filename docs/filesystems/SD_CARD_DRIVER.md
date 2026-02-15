# SD Card Block Device Driver

## Overview

The SD Card driver provides block-level access to SD/SDHC/SDXC cards via the SDHCI (SD Host Controller Interface) on Raspberry Pi 5.

## Hardware Interface

### SDHCI Registers

The Raspberry Pi 5 uses the RP1 I/O controller with standard SDHCI registers:

| Register | Offset | Description |
|----------|--------|-------------|
| SDMA System Address | 0x00 | DMA buffer address |
| Block Size | 0x04 | Transfer block size |
| Block Count | 0x06 | Number of blocks |
| Argument | 0x08 | Command argument |
| Transfer Mode | 0x0C | Transfer direction |
| Command | 0x0E | Command register |
| Response 0-3 | 0x10-0x1C | Command response |
| Buffer Data Port | 0x20 | Data FIFO |
| Present State | 0x24 | Controller state |
| Host Control | 0x28 | Host settings |
| Clock Control | 0x2C | Clock configuration |
| Software Reset | 0x2F | Controller reset |
| Normal Int Status | 0x30 | Normal interrupts |
| Error Int Status | 0x32 | Error interrupts |

### Initialization Sequence

1. **Software Reset**
   - Reset all (bit 0 of Software Reset register)
   - Wait for reset to complete

2. **Power On**
   - Set power control (3.3V)
   - Wait for power stable

3. **Clock Setup**
   - Set initialization clock (400 KHz)
   - Enable SD clock

4. **Card Initialization**
   - CMD0: Go idle state
   - CMD8: Send interface condition (SD 2.0+)
   - ACMD41: Send operating condition (repeat until ready)
   - CMD2: Get CID (Card ID)
   - CMD3: Get RCA (Relative Card Address)
   - CMD9: Get CSD (Card Specific Data)

5. **High Speed Mode**
   - Switch to 25 MHz clock
   - Optional: Enable 4-bit mode (ACMD6)

## Block Operations

### Reading a Sector

```rust
let mut sd = unsafe { SdCardBlockDevice::new(0x1F000000) }?;
sd.init()?;

let mut buffer = vec![0u8; 512];
sd.read_sector(0, &mut buffer)?;
```

### Writing a Sector

```rust
let data = vec![0xABu8; 512];
sd.write_sector(0, &data)?;
```

### Multi-Block Operations

```rust
// Read multiple sectors
let mut buffer = vec![0u8; 512 * 10];
sd.read_sectors(0, 10, &mut buffer)?;

// Write multiple sectors
let data = vec![0xCDu8; 512 * 10];
sd.write_sectors(0, 10, &data)?;
```

## Error Handling

### Retry Logic

All operations include configurable retry logic:

```rust
// Default: 3 retries
let mut sd = unsafe { SdCardBlockDevice::new(0x1F000000) }?;
sd.init()?;
```

### Error Types

| Error | Cause | Resolution |
|-------|-------|------------|
| Timeout | Card not responding | Retry |
| CRC Error | Data corruption | Retry |
| Command Error | Invalid command | Check command |
| Write Protect | Physical lock | Remove protection |

## Performance

### Clock Speeds

| Mode | Clock | Transfer Rate |
|------|-------|---------------|
| Initialization | 400 KHz | 200 KB/s |
| Default Speed | 25 MHz | 12.5 MB/s |
| High Speed | 50 MHz | 25 MB/s |

### Optimization Tips

1. **Use multi-block operations** when possible
2. **Align buffers** to 4-byte boundary
3. **Enable 4-bit mode** for 4x speed improvement
4. **Use DMA** for large transfers (future enhancement)

## Testing

### QEMU Testing

For testing without hardware:

```bash
# Create virtual SD image
dd if=/dev/zero of=sd_image.img bs=512 count=10000

# Format with FAT32
mkfs.vfat -F 32 sd_image.img

# Use with QEMU
qemu-system-aarch64 -M virt -drive file=sd_image.img,format=raw,if=sd
```

### Unit Tests

```rust
#[test]
fn test_sd_read_write() {
    // Use virtual block device for testing
    let mut dev = VirtualBlockDevice::new(10000);
    
    let write_data = vec![0xABu8; 512];
    dev.write_block(0, &write_data).unwrap();
    
    let mut read_data = vec![0u8; 512];
    dev.read_block(0, &mut read_data).unwrap();
    
    assert_eq!(write_data, read_data);
}
```

## Safety Considerations

### Unsafe Code

The `SdBlockDevice::new()` function is unsafe because:
- Requires valid SDHCI base address
- Raw pointer operations on MMIO registers
- No runtime validation of hardware presence

### Safe Wrapper

Use `SdCardBlockDevice` for safe access:

```rust
// Safe wrapper handles initialization
let sd = unsafe { SdCardBlockDevice::new(BASE_ADDR) }?;
```

## Troubleshooting

### Card Not Detected

```rust
if !sd.is_card_present() {
    println!("No SD card inserted");
}
```

### Write Protection

```rust
if sd.is_write_protected() {
    println!("Card is write-protected");
}
```

### Capacity Detection

```rust
let capacity_sectors = sd.capacity();
let capacity_mb = capacity_sectors * 512 / (1024 * 1024);
println!("Card capacity: {} MB", capacity_mb);
```
