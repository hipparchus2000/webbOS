# Week 4 Research Report: Filesystem Integration for Raspberry Pi 5

**Research Specialist:** Sofia La Savante  
**Date:** February 15, 2026  
**Project:** webbOS Raspberry Pi 5 Porting  
**Research Duration:** 1-hour sprint  

---

## Executive Summary

This report provides comprehensive research findings on filesystem integration options for the webbOS Raspberry Pi 5 porting project. Key findings include:

1. **Raspberry Pi 5 Storage:** Multiple storage interfaces available (SD card UHS-I, NVMe via PCIe 2.0 x1, USB 3.0)
2. **FAT32 Write Support:** webbOS has partial implementation; needs completion for full write operations
3. **EXT4 Feasibility:** Highly suitable for Pi 5 with journaling support and better performance than FAT32
4. **Block Device Drivers:** SDHCI/SDHOST controller for SD card; NVMe driver for PCIe SSD
5. **Performance Optimization:** SD card speed classes, caching strategies, and wear leveling critical for flash storage

---

## 1. Raspberry Pi 5 Storage Options

### 1.1 SD Card Interface (SDHOST/SDIO)

#### Hardware Specifications
- **Controller:** Arasan SDHCI-compliant controller
- **Interface:** SD1 (Arasan SD card/MMC interface) - also supports SDIO
- **Standard:** UHS-I SDR104 (Single Data Rate, 104 MB/s theoretical max)
- **Bus Width:** 4-bit data bus
- **Clock:** Up to 208 MHz (SDR104 mode)

#### Performance Characteristics
| Metric | Value |
|--------|-------|
| Theoretical Max Speed | 104 MB/s |
| Practical Sequential Read | 40-80 MB/s |
| Practical Sequential Write | 40-80 MB/s |
| Random 4K Read | 1,000-2,000 IOPS |
| Random 4K Write | 500-1,000 IOPS |
| Block Size | 512 bytes (typical) |

#### SD Card Speed Classes
| Speed Class | Min Write Speed | Use Case |
|-------------|-----------------|----------|
| Class 4 | 4 MB/s | Basic storage |
| Class 10 | 10 MB/s | Full HD video |
| UHS-I U1 | 10 MB/s | Full HD video |
| UHS-I U3 | 30 MB/s | 4K video |
| Video Speed V30 | 30 MB/s | 4K video |
| Video Speed V60 | 60 MB/s | 8K video |
| Video Speed V90 | 90 MB/s | 8K video |

**Recommendation:** Use UHS-I U3 (30 MB/s) or V30 minimum for webbOS to ensure adequate performance.

### 1.2 eMMC Support

#### Availability
- **Raspberry Pi 5 (SBC):** No on-board eMMC
- **Compute Module 5 (CM5):** Optional eMMC (16GB, 32GB, 64GB variants)
- **Raspberry Pi 500+:** 256GB internal M.2 SSD (not eMMC)

#### eMMC Specifications (CM5)
| Feature | Specification |
|---------|---------------|
| Standard | MMC 5.1 |
| Bus Width | 8-bit |
| Clock Speed | 200 MHz (HS400 mode) |
| Theoretical Max | 400 MB/s (HS400) |
| Boot Partitions | 2x independent boot partitions |
| RPMB | Replay Protected Memory Block support |

**Note:** For standard Pi 5 SBC, eMMC is not available. Consider this for CM5-based deployments only.

### 1.3 USB Storage via USB 3.0

#### Hardware Capabilities
- **USB 3.2 Gen 1:** 5 Gbps (2x Type-A ports)
- **USB 2.0:** 480 Mbps (2x ports)
- **Power:** 600mA peripheral limit (with 5V/3A PSU) or 1.2A (with 5V/5A PSU)

#### USB Storage Performance
| Device Type | Interface | Read Speed | Write Speed |
|-------------|-----------|------------|-------------|
| USB 3.0 Flash Drive | USB 3.0 | 100-200 MB/s | 50-150 MB/s |
| USB 3.0 SSD (SATA) | USB 3.0 | 400-500 MB/s | 300-400 MB/s |
| USB 2.0 Flash Drive | USB 2.0 | 30-40 MB/s | 10-20 MB/s |

**Considerations:**
- USB boot requires updated bootloader EEPROM
- USB storage requires USB host controller driver (xHCI)
- Power consumption may require powered USB hub

### 1.4 NVMe via PCIe

#### Hardware Interface
- **PCIe Version:** 2.0 x1 (single lane)
- **Connector:** 16-pin FFC (Flexible Flat Cable) connector
- **Form Factor:** Requires M.2 HAT or adapter board
- **Certified Speed:** Gen 2.0 (5 GT/s)
- **Overclockable:** Gen 3.0 (10 GT/s) - unsupported but possible

#### Performance Specifications
| Metric | Gen 2.0 (Certified) | Gen 3.0 (Overclocked) |
|--------|---------------------|----------------------|
| Theoretical Bandwidth | 500 MB/s | ~900-1000 MB/s |
| Practical Sequential Read | 350-450 MB/s | 700-850 MB/s |
| Practical Sequential Write | 300-400 MB/s | 600-750 MB/s |
| Random 4K Read | 50,000-80,000 IOPS | 80,000-120,000 IOPS |
| Random 4K Write | 20,000-40,000 IOPS | 40,000-60,000 IOPS |

#### Compatible NVMe SSDs
**Known Working:**
- Samsung 970 EVO/EVO Plus
- Samsung 980/980 Pro
- WD Blue SN570
- Kingston NV1, NV2
- Crucial P3, P3 Plus

**Known Issues (Phison Controller):**
- WD Blue SN550/SN580
- WD Black SN850/SN770
- WD SN740
- Corsair MP600

**Recommendation:** Samsung 970 EVO Plus 250GB/500GB for best compatibility and performance.

### 1.5 Storage Comparison Summary

| Storage Type | Speed | Reliability | Cost | Boot Support | Recommendation |
|--------------|-------|-------------|------|--------------|----------------|
| SD Card (UHS-I) | Low | Medium | Low | Yes | Development only |
| USB 3.0 SSD | Medium | Medium | Medium | Yes | Budget option |
| NVMe SSD (PCIe) | High | High | Higher | Yes | **Production** |
| eMMC (CM5 only) | Medium | High | Medium | Yes | Embedded deployments |

---

## 2. FAT32 Filesystem Analysis

### 2.1 Current webbOS VFAT32 Implementation Status

#### Existing Components (✅ Complete)
- **FAT32 Reader:** Full read-only FAT32 support
- **Boot Sector Parsing:** BPB (BIOS Parameter Block) parsing
- **Directory Traversal:** 8.3 and Long File Name (LFN) support
- **Cluster Chain Following:** FAT table traversal for file reads
- **VFS Integration:** `FileSystem` trait implementation

#### Write Support Status (⚠️ Partial)
Current implementation in `/kernel/src/fs/fat32/`:

| Component | File | Status | Notes |
|-----------|------|--------|-------|
| Core FAT32 | `mod.rs` | ✅ Read-only | Main filesystem implementation |
| Write Operations | `write_ops.rs` | ⚠️ Partial | Basic write functions defined |
| FAT Table Mgmt | `fat_table.rs` | ⚠️ Partial | Cluster allocation implemented |
| Directory Ops | `directory_ops.rs` | ⚠️ Partial | Entry creation stubbed |

#### Current Write Limitations
```rust
// From mod.rs - Write operations return ReadOnly error
fn write(&self, _inode: INode, _offset: u64, _buf: &[u8]) -> FsResult<usize> {
    Err(FsError::ReadOnly)
}

fn create(&self, _parent: INode, _name: &str, _file_type: FileType) -> FsResult<INode> {
    Err(FsError::ReadOnly)
}

fn remove(&self, _parent: INode, _name: &str) -> FsResult<()> {
    Err(FsError::ReadOnly)
}
```

### 2.2 Write Support Implementation Plan

#### Phase 1: FAT Table Operations (Week 1)
```rust
// fat_table.rs - Required additions
impl FatTableManager {
    /// Allocate new cluster chain
    pub fn allocate_cluster_chain(&mut self, count: u32) -> FsResult<u32>
    
    /// Extend existing chain
    pub fn extend_cluster_chain(&mut self, last_cluster: u32, count: u32) -> FsResult<u32>
    
    /// Deallocate cluster chain
    pub fn deallocate_cluster_chain(&mut self, start_cluster: u32) -> FsResult<()>
    
    /// Mark cluster as bad
    pub fn mark_bad_cluster(&mut self, cluster: u32) -> FsResult<()>
}
```

#### Phase 2: Directory Entry Management (Week 2)
```rust
// directory_ops.rs - Required additions
impl DirectoryManager {
    /// Create new directory entry
    pub fn create_entry(&mut self, parent_cluster: u32, name: &str, 
                       file_type: FileType, start_cluster: u32, size: u32) -> FsResult<()>
    
    /// Delete directory entry
    pub fn delete_entry(&mut self, parent_cluster: u32, name: &str) -> FsResult<()>
    
    /// Update entry size/timestamp
    pub fn update_entry(&mut self, parent_cluster: u32, name: &str, 
                       size: u32, timestamp: u32) -> FsResult<()>
    
    /// Find free directory slot
    fn find_free_slot(&self, parent_cluster: u32) -> FsResult<(u32, usize)>
}
```

#### Phase 3: File Write Operations (Week 3)
```rust
// write_ops.rs - Required implementations
impl WriteManager {
    /// Write data at specific offset
    pub fn write_to_file(&mut self, start_cluster: u32, offset: u64, 
                         data: &[u8]) -> FsResult<u32>
    
    /// Append data to file
    pub fn append_to_file(&mut self, start_cluster: u32, current_size: u32,
                          data: &[u8]) -> FsResult<u32>
    
    /// Truncate file
    pub fn truncate_file(&mut self, start_cluster: u32, new_size: u32) -> FsResult<()>
}
```

#### Phase 4: VFS Integration (Week 4)
Update `mod.rs` to use write managers:
```rust
impl FileSystem for Fat32Fs {
    fn write(&self, inode: INode, offset: u64, buf: &[u8]) -> FsResult<usize> {
        // Use WriteManager for actual write operations
        let cluster = inode.as_u64() as u32;
        let mut writer = self.get_write_manager()?;
        writer.write_to_file(cluster, offset, buf)
    }
    
    fn create(&self, parent: INode, name: &str, file_type: FileType) -> FsResult<INode> {
        let parent_cluster = parent.as_u64() as u32;
        let mut writer = self.get_write_manager()?;
        let new_cluster = writer.create_file_or_directory(parent_cluster, name, file_type)?;
        Ok(INode::new(new_cluster as u64))
    }
    
    fn remove(&self, parent: INode, name: &str) -> FsResult<()> {
        let parent_cluster = parent.as_u64() as u32;
        let mut writer = self.get_write_manager()?;
        writer.remove_file_or_directory(parent_cluster, name)
    }
}
```

### 2.3 FAT32 Implementation Requirements

#### Data Structures
```rust
/// FAT32 Boot Sector (BPB)
#[repr(C)]
pub struct BootSector {
    pub jmp: [u8; 3],
    pub oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub root_entries: u16,
    pub total_sectors_16: u16,
    pub media_type: u8,
    pub sectors_per_fat_16: u16,
    pub sectors_per_track: u16,
    pub head_count: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    // FAT32 extended fields
    pub sectors_per_fat_32: u32,
    pub ext_flags: u16,
    pub fs_version: u16,
    pub root_cluster: u32,
    pub fs_info_sector: u16,
    pub backup_boot_sector: u16,
    pub reserved: [u8; 12],
    pub drive_num: u8,
    pub reserved1: u8,
    pub boot_sig: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fs_type: [u8; 8],
}

/// Directory Entry (32 bytes)
#[repr(C)]
pub struct DirEntry {
    pub name: [u8; 11],           // 8.3 filename
    pub attrs: u8,                // Attributes
    pub reserved: u8,
    pub create_time_tenths: u8,
    pub create_time: u16,
    pub create_date: u16,
    pub access_date: u16,
    pub cluster_high: u16,        // High 16 bits of cluster
    pub modify_time: u16,
    pub modify_date: u16,
    pub cluster_low: u16,         // Low 16 bits of cluster
    pub size: u32,                // File size in bytes
}
```

#### FAT Entry Values
```rust
const FAT_ENTRY_FREE: u32 = 0x00000000;
const FAT_ENTRY_RESERVED: u32 = 0x00000001;
const FAT_ENTRY_MIN: u32 = 0x00000002;
const FAT_ENTRY_MAX: u32 = 0x0FFFFFF6;
const FAT_ENTRY_BAD: u32 = 0x0FFFFFF7;
const FAT_ENTRY_EOF: u32 = 0x0FFFFFFF;
```

### 2.4 Cluster Allocation Strategy

#### First-Fit Allocation (Recommended)
```rust
/// Find first free cluster starting from cluster 2
pub fn find_free_cluster(&self) -> Option<u32> {
    for cluster in 2..self.fat_cache.len() as u32 {
        if self.fat_cache[cluster as usize] == FAT_ENTRY_FREE {
            return Some(cluster);
        }
    }
    None
}
```

#### FAT Synchronization
- **Dual FAT Support:** Update both FAT copies on write
- **Dirty Flag:** Track modifications for lazy sync
- **Atomic Updates:** Write new entry before marking old as free

---

## 3. EXT4 Filesystem Analysis

### 3.1 EXT4 for Embedded Systems Overview

EXT4 (Fourth Extended Filesystem) is the default filesystem for most Linux distributions and is highly suitable for Raspberry Pi 5.

#### Key Advantages for webbOS
| Feature | Benefit |
|---------|---------|
| Journaling | Data integrity on power loss |
| Extents | Reduced fragmentation, better large file performance |
| Delayed Allocation | Improved write performance |
| Multi-block Allocation | Reduced fragmentation |
| Fast fsck | Faster filesystem checks |
| Large Files | Up to 16 TB files, 1 EB volumes |

### 3.2 EXT4 vs FAT32 Comparison

| Feature | EXT4 | FAT32 | Winner |
|---------|------|-------|--------|
| **Reliability** | Journaling, checksums | No journaling | EXT4 |
| **Max File Size** | 16 TB | 4 GB | EXT4 |
| **Max Volume** | 1 EB | 2 TB (16 TB with hacks) | EXT4 |
| **Fragmentation** | Extents minimize | High fragmentation | EXT4 |
| **Performance** | Optimized for Linux | Slower on Linux | EXT4 |
| **Compatibility** | Linux native | Universal | FAT32 |
| **Boot Support** | Yes (bootloader) | Required for UEFI | Both |
| **Complexity** | Higher | Lower | FAT32 |
| **Overhead** | Higher | Lower | FAT32 |

### 3.3 Journaling Considerations

#### Journaling Modes
```rust
/// EXT4 journal modes
pub enum JournalMode {
    /// Journal metadata and data (safest, slowest)
    DataOrdered,
    
    /// Journal metadata only, data ordered (balanced)
    Writeback,
    
    /// No journaling (fastest, least safe)
    None,
}
```

#### Recommendations for webbOS
- **Root Filesystem:** `data=ordered` for safety
- **Boot Partition:** FAT32 required for firmware
- **Data Partition:** `data=writeback` for performance

### 3.4 EXT4 Feasibility Assessment

#### ✅ Highly Feasible
- **Mature Implementation:** Linux kernel reference available
- **Suitable for Pi 5:** Optimized for ARM64 architecture
- **Bootloader Support:** Raspberry Pi firmware supports EXT4 boot
- **Performance:** Significantly better than FAT32 for general use

#### ⚠️ Considerations
- **Complexity:** More complex than FAT32 to implement
- **Memory Usage:** Requires more RAM for metadata caching
- **Boot Requirements:** Boot partition must still be FAT32

#### Implementation Priority
1. **Phase 1:** Complete FAT32 write support (required for boot)
2. **Phase 2:** Implement EXT4 read-only support
3. **Phase 3:** Add EXT4 journaling write support
4. **Phase 4:** Optimize for Pi 5 storage characteristics

### 3.5 EXT4 Implementation Requirements

#### Core Data Structures
```rust
/// EXT4 Superblock
#[repr(C)]
pub struct Ext4Superblock {
    pub s_inodes_count: u32,
    pub s_blocks_count_lo: u32,
    pub s_r_blocks_count_lo: u32,
    pub s_free_blocks_count_lo: u32,
    pub s_free_inodes_count: u32,
    pub s_first_data_block: u32,
    pub s_log_block_size: u32,
    pub s_log_cluster_size: u32,
    pub s_blocks_per_group: u32,
    pub s_clusters_per_group: u32,
    pub s_inodes_per_group: u32,
    pub s_mtime: u32,
    pub s_wtime: u32,
    pub s_mnt_count: u16,
    pub s_max_mnt_count: u16,
    pub s_magic: u16,  // 0xEF53
    pub s_state: u16,
    pub s_errors: u16,
    pub s_minor_rev_level: u16,
    pub s_lastcheck: u32,
    pub s_checkinterval: u32,
    pub s_creator_os: u32,
    pub s_rev_level: u32,
    pub s_def_resuid: u16,
    pub s_def_resgid: u16,
    // ... more fields
}

/// EXT4 Inode
#[repr(C)]
pub struct Ext4Inode {
    pub i_mode: u16,
    pub i_uid: u16,
    pub i_size_lo: u32,
    pub i_atime: u32,
    pub i_ctime: u32,
    pub i_mtime: u32,
    pub i_dtime: u32,
    pub i_gid: u16,
    pub i_links_count: u16,
    pub i_blocks_lo: u32,
    pub i_flags: u32,
    pub i_block: [u32; 15],  // Block pointers
    // ... more fields
}
```

---

## 4. Filesystem Drivers

### 4.1 Block Device Drivers for SD Card

#### SDHCI (Secure Digital Host Controller Interface)

The Raspberry Pi 5 uses an Arasan SDHCI-compliant controller for SD card access.

##### Register Layout
```rust
/// SDHCI Register Offsets
pub const SDHCI_DMA_ADDRESS: usize = 0x00;
pub const SDHCI_BLOCK_SIZE: usize = 0x04;
pub const SDHCI_BLOCK_COUNT: usize = 0x06;
pub const SDHCI_ARGUMENT: usize = 0x08;
pub const SDHCI_TRANSFER_MODE: usize = 0x0C;
pub const SDHCI_COMMAND: usize = 0x0E;
pub const SDHCI_RESPONSE: usize = 0x10;
pub const SDHCI_BUFFER: usize = 0x20;
pub const SDHCI_PRESENT_STATE: usize = 0x24;
pub const SDHCI_HOST_CONTROL: usize = 0x28;
pub const SDHCI_POWER_CONTROL: usize = 0x29;
pub const SDHCI_BLOCK_GAP_CONTROL: usize = 0x2A;
pub const SDHCI_WAKE_UP_CONTROL: usize = 0x2B;
pub const SDHCI_CLOCK_CONTROL: usize = 0x2C;
pub const SDHCI_TIMEOUT_CONTROL: usize = 0x2E;
pub const SDHCI_SOFTWARE_RESET: usize = 0x2F;
pub const SDHCI_INT_STATUS: usize = 0x30;
pub const SDHCI_INT_ENABLE: usize = 0x34;
pub const SDHCI_SIGNAL_ENABLE: usize = 0x38;
pub const SDHCI_ACMD12_ERR: usize = 0x3C;
pub const SDHCI_HOST_CONTROL2: usize = 0x3E;
pub const SDHCI_CAPABILITIES: usize = 0x40;
pub const SDHCI_CAPABILITIES_1: usize = 0x44;
pub const SDHCI_MAX_CURRENT: usize = 0x48;
```

##### SDHCI Driver Implementation Outline
```rust
/// SDHCI Host Controller Driver
pub struct SdhciHost {
    /// MMIO base address
    base: usize,
    /// Current clock frequency
    clock: u32,
    /// Current bus width (1, 4, or 8)
    bus_width: u8,
    /// DMA support flags
    dma_support: DmaSupport,
}

impl SdhciHost {
    /// Initialize SDHCI controller
    pub fn init(base: usize) -> Result<Self, SdError> {
        // 1. Reset controller
        // 2. Enable interrupts
        // 3. Set initial clock (400kHz for initialization)
        // 4. Power on SD card
    }
    
    /// Send command to SD card
    pub fn send_command(&mut self, cmd: u8, arg: u32, resp_type: RespType) -> Result<CommandResponse, SdError> {
        // 1. Wait for command inhibit clear
        // 2. Set command argument
        // 3. Set command register
        // 4. Wait for command complete interrupt
        // 5. Read response
    }
    
    /// Read data blocks
    pub fn read_blocks(&mut self, start_block: u32, count: usize, buf: &mut [u8]) -> Result<(), SdError> {
        // 1. Set block size and count
        // 2. Send read command (CMD17 for single, CMD18 for multiple)
        // 3. Transfer data via DMA or PIO
        // 4. Wait for transfer complete
    }
    
    /// Write data blocks
    pub fn write_blocks(&mut self, start_block: u32, count: usize, buf: &[u8]) -> Result<(), SdError> {
        // 1. Set block size and count
        // 2. Send write command (CMD24 for single, CMD25 for multiple)
        // 3. Transfer data via DMA or PIO
        // 4. Wait for transfer complete
        // 5. Wait for programming complete
    }
}
```

##### SD Card Initialization Sequence
```rust
/// SD Card initialization sequence
pub fn sd_card_init(host: &mut SdhciHost) -> Result<SdCardInfo, SdError> {
    // 1. CMD0: Go idle state
    host.send_command(0, 0, RespType::None)?;
    
    // 2. CMD8: Check voltage range (SDHC/SDXC)
    let response = host.send_command(8, 0x1AA, RespType::R7)?;
    let supports_sdhc = response.check_pattern() == 0xAA;
    
    // 3. ACMD41: Initialize card (repeat until ready)
    let mut ocr = 0;
    loop {
        host.send_command(55, 0, RespType::R1)?;  // APP_CMD
        let resp = host.send_command(41, 0x40FF8000, RespType::R3)?;
        ocr = resp.ocr();
        if ocr & 0x80000000 != 0 {
            break;  // Card ready
        }
        sleep_ms(10);
    }
    
    // 4. CMD2: Get CID
    let cid = host.send_command(2, 0, RespType::R2)?;
    
    // 5. CMD3: Get RCA (Relative Card Address)
    let rca_resp = host.send_command(3, 0, RespType::R6)?;
    let rca = rca_resp.rca();
    
    // 6. CMD9: Get CSD
    let csd = host.send_command(9, (rca as u32) << 16, RespType::R2)?;
    
    // 7. CMD7: Select card
    host.send_command(7, (rca as u32) << 16, RespType::R1)?;
    
    // 8. ACMD6: Set bus width to 4-bit
    host.send_command(55, (rca as u32) << 16, RespType::R1)?;
    host.send_command(6, 0x02, RespType::R1)?;
    host.set_bus_width(4);
    
    // 9. Switch to high speed mode if supported
    // CMD6: SWITCH_FUNC
    
    Ok(SdCardInfo { cid, csd, ocr, rca })
}
```

### 4.2 MBR/GPT Partition Table Support

#### Master Boot Record (MBR)
```rust
/// MBR Partition Entry
#[repr(C)]
pub struct MbrPartitionEntry {
    pub boot_indicator: u8,     // 0x80 = bootable
    pub start_chs: [u8; 3],     // Starting CHS (ignored on UEFI)
    pub partition_type: u8,     // Filesystem type
    pub end_chs: [u8; 3],       // Ending CHS (ignored on UEFI)
    pub start_lba: u32,         // Starting LBA
    pub size_lba: u32,          // Size in sectors
}

/// Common MBR Partition Types
pub const MBR_FAT32: u8 = 0x0C;      // FAT32 (LBA)
pub const MBR_FAT32_ALT: u8 = 0x0B;  // FAT32
pub const MBR_LINUX: u8 = 0x83;      // Linux native (EXT4)
pub const MBR_LINUX_SWAP: u8 = 0x82; // Linux swap
pub const MBR_EFI: u8 = 0xEF;        // EFI System Partition
```

#### GPT (GUID Partition Table)
```rust
/// GPT Partition Entry
#[repr(C)]
pub struct GptPartitionEntry {
    pub partition_type_guid: [u8; 16],
    pub unique_guid: [u8; 16],
    pub start_lba: u64,
    pub end_lba: u64,
    pub attributes: u64,
    pub name: [u16; 36],  // UTF-16LE partition name
}

/// Common GPT Partition Type GUIDs
pub const GPT_ESP: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";  // EFI System
pub const GPT_LINUX_FS: &str = "0FC63DAF-8483-4772-8E79-3D69D8477DE4"; // Linux FS
pub const GPT_LINUX_ROOT_ARM64: &str = "B921B045-1DF0-41C3-AF44-4C6F280D3FAE";
```

#### Partition Table Detection
```rust
/// Detect and parse partition table
pub fn parse_partition_table(device: &dyn BlockDevice) -> Result<PartitionTable, PartitionError> {
    let mut sector0 = [0u8; 512];
    device.read_blocks(0, 1, &mut sector0)?;
    
    // Check for GPT first (look for protective MBR)
    if is_protective_mbr(&sector0) {
        // Read GPT header (LBA 1)
        let mut gpt_header = [0u8; 512];
        device.read_blocks(1, 1, &mut gpt_header)?;
        return parse_gpt(device, &gpt_header);
    }
    
    // Check for MBR signature
    if sector0[510] == 0x55 && sector0[511] == 0xAA {
        return parse_mbr(&sector0);
    }
    
    Err(PartitionError::UnknownPartitionTable)
}
```

### 4.3 Multiple Filesystem Support

#### Filesystem Registry
```rust
/// Filesystem type registry
pub struct FilesystemRegistry {
    filesystems: BTreeMap<&'static str, Box<dyn FilesystemDriver>>,
}

impl FilesystemRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            filesystems: BTreeMap::new(),
        };
        
        // Register built-in filesystems
        registry.register("fat32", Box::new(Fat32Driver));
        registry.register("ext4", Box::new(Ext4Driver));
        registry.register("ext2", Box::new(Ext2Driver));
        
        registry
    }
    
    pub fn mount(&self, fs_type: &str, device: Box<dyn BlockDevice>) -> Result<Box<dyn FileSystem>, FsError> {
        let driver = self.filesystems.get(fs_type)
            .ok_or(FsError::UnsupportedFilesystem)?;
        driver.mount(device)
    }
    
    pub fn auto_mount(&self, device: Box<dyn BlockDevice>) -> Result<Box<dyn FileSystem>, FsError> {
        // Try to detect filesystem type from superblock
        let mut buf = [0u8; 512];
        device.read_blocks(0, 1, &mut buf)?;
        
        // Check for EXT4
        if &buf[1024..1026] == &[0x53, 0xEF] {  // EXT4 magic
            return self.mount("ext4", device);
        }
        
        // Check for FAT32
        if &buf[510..512] == &[0x55, 0xAA] && buf[82] == 0x29 {  // FAT32 signature
            return self.mount("fat32", device);
        }
        
        Err(FsError::UnknownFilesystem)
    }
}
```

---

## 5. Performance Considerations

### 5.1 SD Card Speed Classes

#### Speed Class Selection Guide
| Use Case | Minimum Class | Recommended |
|----------|--------------|-------------|
| OS Boot | Class 10 | U3/V30 |
| Web Browsing | U1 | U3 |
| Video Playback | U1 | U3 |
| File Server | U3 | V60 |
| Database | U3 | V60 |
| Development | U3 | V30 |

#### Application Performance Class (A1/A2)
For running applications from SD card:
- **A1:** 1500 read IOPS, 500 write IOPS minimum
- **A2:** 4000 read IOPS, 2000 write IOPS minimum (requires driver support)

**Recommendation:** Use A2-rated SD cards for webbOS if SD card is primary storage.

### 5.2 Caching Strategies

#### Page Cache Implementation
```rust
/// Page cache for block devices
pub struct PageCache {
    /// Cache entries indexed by (device_id, block_num)
    pages: LruCache<(u32, u64), CachePage>,
    /// Dirty pages to be written back
    dirty_pages: BTreeSet<(u32, u64)>,
    /// Maximum cache size
    max_size: usize,
}

pub struct CachePage {
    data: Box<[u8; 4096]>,
    dirty: bool,
    access_count: u32,
    last_access: Instant,
}

impl PageCache {
    /// Read from cache or fetch from device
    pub fn read(&mut self, device: &dyn BlockDevice, block: u64, buf: &mut [u8]) -> Result<(), CacheError> {
        let key = (device.id(), block);
        
        if let Some(page) = self.pages.get_mut(&key) {
            // Cache hit
            page.access_count += 1;
            page.last_access = Instant::now();
            buf.copy_from_slice(&page.data[..buf.len()]);
            return Ok(());
        }
        
        // Cache miss - fetch from device
        let mut page = CachePage::new();
        device.read_blocks(block, 1, &mut page.data)?;
        buf.copy_from_slice(&page.data[..buf.len()]);
        
        // Insert into cache (evicts oldest if full)
        self.pages.put(key, page);
        Ok(())
    }
    
    /// Write to cache (mark as dirty)
    pub fn write(&mut self, device: &dyn BlockDevice, block: u64, buf: &[u8]) -> Result<(), CacheError> {
        let key = (device.id(), block);
        
        let page = self.pages.get_mut(&key)
            .map(|p| {
                p.data.copy_from_slice(buf);
                p.dirty = true;
                p
            })
            .or_else(|| {
                // Allocate new page
                let mut page = CachePage::new();
                page.data.copy_from_slice(buf);
                page.dirty = true;
                self.pages.put(key, page);
                self.pages.get_mut(&key)
            });
        
        if let Some(p) = page {
            p.access_count += 1;
            p.last_access = Instant::now();
            self.dirty_pages.insert(key);
        }
        
        Ok(())
    }
    
    /// Flush dirty pages to device
    pub fn flush(&mut self, device: &dyn BlockDevice) -> Result<(), CacheError> {
        for key in &self.dirty_pages {
            if let Some(page) = self.pages.get(key) {
                device.write_blocks(key.1, 1, &page.data)?;
            }
        }
        self.dirty_pages.clear();
        Ok(())
    }
}
```

#### Write-Back vs Write-Through
| Strategy | Latency | Durability | Use Case |
|----------|---------|------------|----------|
| Write-Through | Higher | Immediate | Critical data |
| Write-Back | Lower | Delayed | General purpose |
| Write-Around | Medium | N/A | Large sequential writes |

**Recommendation:** Use write-back caching with periodic sync for general use, write-through for journal/metadata.

### 5.3 Wear Leveling for Flash Storage

#### Flash Memory Characteristics
- **Write/Erase Cycles:**
  - SLC: ~100,000 cycles
  - MLC: ~10,000 cycles
  - TLC: ~3,000 cycles
  - QLC: ~1,000 cycles
- **Erase Block Size:** Typically 128KB - 4MB
- **Write Amplification:** SSDs internally handle wear leveling

#### Wear Leveling Strategies

##### 1. Dynamic Wear Leveling
```rust
/// Dynamic wear leveling for SD cards
pub struct WearLevelingManager {
    /// Block erase counts
    erase_counts: HashMap<u64, u32>,
    /// Free block pool
    free_blocks: Vec<u64>,
    /// Threshold for triggering wear leveling
    threshold: f32,  // e.g., 0.2 (20% deviation)
}

impl WearLevelingManager {
    /// Select block for writing with wear leveling
    pub fn select_write_block(&mut self) -> u64 {
        // Find block with minimum erase count
        let min_erase = self.erase_counts.values().min().copied().unwrap_or(0);
        
        // Select from free blocks preferring lower erase counts
        self.free_blocks.iter()
            .filter(|&&b| self.erase_counts.get(&b).copied().unwrap_or(0) as f32 
                      <= min_erase as f32 * (1.0 + self.threshold))
            .next()
            .copied()
            .unwrap_or_else(|| self.free_blocks.pop().unwrap())
    }
    
    /// Record erase operation
    pub fn record_erase(&mut self, block: u64) {
        *self.erase_counts.entry(block).or_insert(0) += 1;
    }
}
```

##### 2. Static Wear Leveling
- Move static (read-only) data to high-wear blocks
- Migrate frequently written data to low-wear blocks
- Requires background garbage collection

#### Optimizations for webbOS

##### FAT32 Specific
1. **FAT Table Placement:** Keep FAT at fixed location (blocks 1-N)
2. **Cluster Allocation:** Implement wear-aware allocation
3. **Directory Entries:** Cache and batch updates
4. **FSInfo Sector:** Update only when necessary

##### EXT4 Specific
1. **Journal Mode:** Use `data=writeback` for better performance
2. **Block Groups:** Spread data across groups
3. **Delayed Allocation:** Reduces fragmentation
4. **Discard/Trim:** Enable for SSDs (not applicable to SD cards without controller support)

### 5.4 Performance Optimization Recommendations

#### SD Card Optimizations
```rust
/// SD card performance tuning
pub struct SdPerformanceTuning {
    /// Use 4KB block alignment
    block_alignment: usize,  // 8 sectors (4096 bytes)
    
    /// Enable high-speed mode (50 MHz)
    high_speed: bool,
    
    /// Enable UHS-I SDR104 (208 MHz)
    uhs_mode: UhsMode,
    
    /// Use 4-bit bus width
    bus_width: u8,  // 4
    
    /// Cache configuration
    cache_size: usize,  // 1 MB
    
    /// Read-ahead sectors
    read_ahead: usize,  // 8 sectors
}
```

#### NVMe Optimizations
1. **Enable MSI-X interrupts** for lower latency
2. **Use multiple I/O queues** (up to 64K queues)
3. **Enable Write Combining** for small writes
4. **Align to 4KB boundaries** for best performance
5. **Use PRP (Physical Region Page) lists** for scatter-gather DMA

#### Filesystem Mount Options
```bash
# SD Card - FAT32 (if using Linux for testing)
mount -o noatime,flush,fmask=0022,dmask=0022 /dev/mmcblk0p1 /mnt/sd

# NVMe SSD - EXT4
mount -o noatime,nodiratime,discard,data=writeback /dev/nvme0n1p1 /mnt/nvme
```

---

## 6. Deliverables Summary

### 6.1 Research Report on Raspberry Pi 5 Storage Options

**Completed:** Comprehensive analysis of:
- SD card interface (UHS-I, 104 MB/s theoretical, 40-80 MB/s practical)
- eMMC support (CM5 only, up to 400 MB/s with HS400)
- USB 3.0 storage (5 Gbps, ~400 MB/s practical)
- NVMe via PCIe 2.0 x1 (500 MB/s certified, ~900 MB/s overclocked)

**Recommendations:**
- **Development:** Use high-quality SD card (U3/V30)
- **Production:** Use NVMe SSD (Samsung 970 EVO Plus recommended)

### 6.2 FAT32 Write Implementation Plan

**4-Week Implementation Plan:**
1. **Week 1:** FAT table operations (allocate, extend, deallocate cluster chains)
2. **Week 2:** Directory entry management (create, delete, update entries)
3. **Week 3:** File write operations (write, append, truncate)
4. **Week 4:** VFS integration and testing

**Key Files to Modify:**
- `/kernel/src/fs/fat32/fat_table.rs` - Cluster management
- `/kernel/src/fs/fat32/directory_ops.rs` - Directory operations
- `/kernel/src/fs/fat32/write_ops.rs` - Write operations
- `/kernel/src/fs/fat32/mod.rs` - VFS integration

### 6.3 EXT4 Feasibility Assessment

**Verdict:** ✅ **Highly Feasible**

- Mature, well-documented filesystem
- Superior performance and reliability vs FAT32
- Journaling for data integrity
- Native Linux support
- Bootloader compatibility

**Implementation Priority:** Phase 2 (after FAT32 write support complete)

### 6.4 Block Device Driver Requirements

#### Required Drivers
| Driver | Priority | Complexity | Notes |
|--------|----------|------------|-------|
| SDHCI (SD Card) | High | Medium | Required for boot from SD |
| NVMe (PCIe SSD) | High | Medium | Required for production |
| USB Mass Storage | Medium | High | Requires USB host controller |

#### SDHCI Driver Components
- Register-level SDHCI controller driver
- SD card protocol layer (CMD/ACMD handling)
- Block device abstraction
- DMA support (SDMA/ADMA2)

### 6.5 Performance Optimization Recommendations

#### Storage Selection
- **SD Cards:** U3/V30 minimum, A2-rated for application performance
- **NVMe SSDs:** Samsung 970 EVO Plus for best compatibility

#### Caching Strategy
- Implement page cache with LRU eviction
- Use write-back caching with periodic sync
- Cache size: 1-4 MB for embedded use

#### Wear Leveling
- Dynamic wear leveling for SD cards
- Enable TRIM/discard for NVMe SSDs
- Use EXT4 for better wear distribution than FAT32

---

## 7. References

### Hardware Documentation
1. [Raspberry Pi 5 Documentation](https://www.raspberrypi.com/documentation/computers/raspberry-pi-5.html)
2. [BCM2712 Datasheet](https://datasheets.raspberrypi.com/)
3. [SD Association Specifications](https://www.sdcard.org/downloads/pls/)
4. [PCI Express Base Specification 2.0](https://pcisig.com/specifications)

### Software References
5. [Linux SDHCI Driver](https://github.com/torvalds/linux/tree/master/drivers/mmc/host)
6. [Linux EXT4 Documentation](https://www.kernel.org/doc/html/latest/filesystems/ext4.html)
7. [Microsoft FAT32 Specification](https://docs.microsoft.com/en-us/windows/win32/fileio/fat32-specification)
8. [UEFI Specification - GPT](https://uefi.org/specifications)

### Research Papers
9. [Flash Memory Wear Leveling Survey](https://dl.acm.org/doi/full/10.1145/3723167)
10. [Cache Management for Flash Storage](https://arxiv.org/pdf/1209.3099)

---

## 8. Appendices

### Appendix A: SD Card Pinout
```
SD Card Pinout (MicroSD):
 1 - DAT2 (Data Line 2)
 2 - CD/DAT3 (Card Detect / Data Line 3)
 3 - CMD (Command)
 4 - VDD (Power)
 5 - CLK (Clock)
 6 - VSS (Ground)
 7 - DAT0 (Data Line 0)
 8 - DAT1 (Data Line 1)
```

### Appendix B: NVMe Command Structure
```rust
/// NVMe Submission Queue Entry
#[repr(C)]
pub struct NvmeCommand {
    pub opcode: u8,         // Command opcode
    pub flags: u8,          // Flags
    pub command_id: u16,    // Command identifier
    pub nsid: u32,          // Namespace ID
    pub reserved: u64,
    pub metadata: u64,      // Metadata pointer
    pub data_ptr: DataPtr,  // Data pointer (PRP or SGL)
    pub cdw10: u32,         // Command-specific
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

/// Common Opcodes
pub const NVME_CMD_READ: u8 = 0x02;
pub const NVME_CMD_WRITE: u8 = 0x01;
pub const NVME_CMD_FLUSH: u8 = 0x00;
pub const NVME_CMD_IDENTIFY: u8 = 0x06;
```

### Appendix C: Recommended Hardware

#### SD Cards
- **Budget:** SanDisk Ultra 64GB U1 (~$10)
- **Recommended:** Samsung EVO Select 128GB U3 (~$15)
- **Premium:** SanDisk Extreme Pro 128GB V30 A2 (~$25)

#### NVMe SSDs
- **Budget:** Kingston NV2 250GB (~$30)
- **Recommended:** Samsung 970 EVO Plus 500GB (~$60)
- **Premium:** Samsung 980 Pro 1TB (~$100)

#### M.2 HATs for Pi 5
- **Official:** Raspberry Pi M.2 HAT+ (~$12)
- **Alternative:** Pimoroni NVMe Base (~$15)
- **Alternative:** Geekworm X1001 (~$15)

---

*Report compiled by Sofia La Savante, Research & Architecture Specialist*  
*For the webbOS Raspberry Pi 5 Porting Project*
