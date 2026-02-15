//! Partition table support
//!
//! Supports both MBR (Master Boot Record) and GPT (GUID Partition Table)
//! partition schemes with automatic detection.

use crate::error::VFatError;
use crate::fs::block::BlockDevice;

/// Maximum number of MBR partitions
pub const MBR_MAX_PARTITIONS: usize = 4;

/// Maximum number of GPT partitions
pub const GPT_MAX_PARTITIONS: usize = 128;

/// MBR partition type constants
pub mod partition_types {
    pub const EMPTY: u8 = 0x00;
    pub const FAT12: u8 = 0x01;
    pub const FAT16_LT32M: u8 = 0x04;
    pub const EXTENDED: u8 = 0x05;
    pub const FAT16_GT32M: u8 = 0x06;
    pub const NTFS_EXFAT: u8 = 0x07;
    pub const FAT32_CHS: u8 = 0x0B;
    pub const FAT32_LBA: u8 = 0x0C;
    pub const FAT16_LBA: u8 = 0x0E;
    pub const LINUX_NATIVE: u8 = 0x83;
    pub const LINUX_EXTENDED: u8 = 0x85;
    pub const LINUX_LVM: u8 = 0x8E;
    pub const EFI_SYSTEM: u8 = 0xEF;
}

/// GPT partition type GUIDs
pub mod gpt_types {
    /// EFI System Partition
    pub const EFI_SYSTEM: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";
    /// Microsoft Basic Data
    pub const MICROSOFT_BASIC_DATA: &str = "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7";
    /// Linux Filesystem Data
    pub const LINUX_FILESYSTEM: &str = "0FC63DAF-8483-4772-8E79-3D69D8477DE4";
    /// Linux Swap
    pub const LINUX_SWAP: &str = "0657FD6D-A4AB-43C4-84E5-0933C84B4F4F";
    /// Linux LVM
    pub const LINUX_LVM: &str = "E6D6D379-F507-44C2-A23C-238F2A3DF928";
}

/// Partition information
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Partition {
    /// Partition number (1-based)
    pub number: u32,
    /// Starting LBA (Logical Block Address)
    pub start_lba: u64,
    /// Ending LBA (inclusive)
    pub end_lba: u64,
    /// Partition size in sectors
    pub sector_count: u64,
    /// Partition type identifier
    pub partition_type: PartitionType,
    /// Bootable flag
    pub bootable: bool,
    /// Partition name (for GPT)
    pub name: [u16; 36],
}

impl Partition {
    /// Create a new partition info
    pub fn new(number: u32, start_lba: u64, end_lba: u64, partition_type: PartitionType) -> Self {
        Self {
            number,
            start_lba,
            end_lba,
            sector_count: end_lba - start_lba + 1,
            partition_type,
            bootable: false,
            name: [0u16; 36],
        }
    }

    /// Check if this is a FAT32 partition
    pub fn is_fat32(&self) -> bool {
        matches!(self.partition_type, 
            PartitionType::Mbr(partition_types::FAT32_CHS) |
            PartitionType::Mbr(partition_types::FAT32_LBA) |
            PartitionType::Gpt(guid) if guid == gpt_types::MICROSOFT_BASIC_DATA)
    }

    /// Check if this is the boot partition
    pub fn is_boot(&self) -> bool {
        self.bootable || matches!(self.partition_type,
            PartitionType::Mbr(partition_types::FAT32_LBA) |
            PartitionType::Gpt(guid) if guid == gpt_types::EFI_SYSTEM)
    }

    /// Get partition name as string
    pub fn name_str(&self) -> String {
        self.name.iter()
            .take_while(|&&c| c != 0)
            .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
            .collect()
    }
}

/// Partition type
#[derive(Debug, Clone, PartialEq)]
pub enum PartitionType {
    /// MBR partition type byte
    Mbr(u8),
    /// GPT partition type GUID
    Gpt(String),
    /// Unknown type
    Unknown,
}

impl Default for PartitionType {
    fn default() -> Self {
        PartitionType::Unknown
    }
}

/// MBR partition entry (16 bytes)
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct MbrPartitionEntry {
    /// Boot indicator (0x80 = bootable)
    boot_indicator: u8,
    /// Starting CHS address
    start_chs: [u8; 3],
    /// Partition type
    partition_type: u8,
    /// Ending CHS address
    end_chs: [u8; 3],
    /// Starting LBA (little-endian)
    start_lba: u32,
    /// Number of sectors (little-endian)
    sector_count: u32,
}

impl MbrPartitionEntry {
    /// Check if this entry is valid (not empty)
    fn is_valid(&self) -> bool {
        self.partition_type != 0
    }

    /// Check if this is an extended partition
    fn is_extended(&self) -> bool {
        self.partition_type == partition_types::EXTENDED ||
        self.partition_type == partition_types::LINUX_EXTENDED
    }
}

/// MBR (Master Boot Record) structure
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct MasterBootRecord {
    /// Boot code (446 bytes)
    boot_code: [u8; 446],
    /// Partition table entries (4 * 16 = 64 bytes)
    partitions: [MbrPartitionEntry; 4],
    /// Boot signature (0x55, 0xAA)
    signature: u16,
}

/// GPT header structure
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct GptHeader {
    /// Signature "EFI PART"
    signature: [u8; 8],
    /// Revision
    revision: u32,
    /// Header size
    header_size: u32,
    /// CRC32 of header
    header_crc32: u32,
    /// Reserved
    reserved: u32,
    /// Current LBA
    current_lba: u64,
    /// Backup LBA
    backup_lba: u64,
    /// First usable LBA
    first_usable_lba: u64,
    /// Last usable LBA
    last_usable_lba: u64,
    /// Disk GUID
    disk_guid: [u8; 16],
    /// Starting LBA of partition entries
    partition_entry_lba: u64,
    /// Number of partition entries
    num_partition_entries: u32,
    /// Size of each partition entry
    partition_entry_size: u32,
    /// CRC32 of partition entry array
    partition_entry_crc32: u32,
}

/// GPT partition entry (128 bytes typically)
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct GptPartitionEntry {
    /// Partition type GUID
    type_guid: [u8; 16],
    /// Unique partition GUID
    unique_guid: [u8; 16],
    /// Starting LBA
    start_lba: u64,
    /// Ending LBA
    end_lba: u64,
    /// Attributes
    attributes: u64,
    /// Partition name (UTF-16LE, 72 bytes = 36 characters)
    name: [u16; 36],
}

/// Partition table types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PartitionTableType {
    /// MBR (Master Boot Record)
    Mbr,
    /// GPT (GUID Partition Table)
    Gpt,
    /// Unknown or no partition table
    Unknown,
}

/// Partition table parser
pub struct PartitionTable {
    /// Type of partition table detected
    pub table_type: PartitionTableType,
    /// Partitions found
    pub partitions: Vec<Partition>,
    /// Disk GUID (for GPT)
    pub disk_guid: Option<String>,
}

impl PartitionTable {
    /// Read and parse partition table from block device
    pub fn read<B: BlockDevice>(device: &mut B) -> Result<Self, VFatError> {
        let mut sector = vec![0u8; device.block_size()];
        
        // Read first sector (protective MBR or actual MBR)
        device.read_block(0, &mut sector)?;

        // Check for GPT first (GPT has protective MBR at LBA 0)
        if Self::is_gpt(device, &sector)? {
            return Self::parse_gpt(device);
        }

        // Parse as MBR
        Self::parse_mbr(device, &sector)
    }

    /// Check if disk uses GPT
    fn is_gpt<B: BlockDevice>(_device: &mut B, first_sector: &[u8]) -> Result<bool, VFatError> {
        // Check for protective MBR signature
        if first_sector.len() < 512 {
            return Ok(false);
        }

        // Check MBR signature
        let signature = u16::from_le_bytes([first_sector[510], first_sector[511]]);
        if signature != 0xAA55 {
            return Ok(false);
        }

        // Check for EFI protective partition type (0xEE)
        let partition_type = first_sector[450];
        if partition_type == 0xEE {
            return Ok(true);
        }

        // Check GPT signature at offset 512 (LBA 1)
        // For now, rely on protective partition type
        Ok(false)
    }

    /// Parse MBR partition table
    fn parse_mbr<B: BlockDevice>(device: &mut B, sector: &[u8]) -> Result<Self, VFatError> {
        // Check MBR signature
        let signature = u16::from_le_bytes([sector[510], sector[511]]);
        if signature != 0xAA55 {
            return Err(VFatError::Corruption(
                "Invalid MBR signature".to_string()
            ));
        }

        let mut partitions = Vec::new();

        // Parse primary partitions
        for i in 0..MBR_MAX_PARTITIONS {
            let offset = 446 + i * 16;
            let entry = Self::parse_mbr_entry(&sector[offset..offset + 16], (i + 1) as u32)?;
            
            if let Some(partition) = entry {
                partitions.push(partition);
            }
        }

        // Parse extended partitions if any
        for i in 0..MBR_MAX_PARTITIONS {
            let offset = 446 + i * 16;
            let entry = Self::parse_mbr_entry_raw(&sector[offset..offset + 16]);
            
            if let Some(entry) = entry {
                if entry.is_extended() {
                    Self::parse_extended_partitions(
                        device,
                        entry.start_lba as u64,
                        &mut partitions,
                    )?;
                }
            }
        }

        Ok(Self {
            table_type: PartitionTableType::Mbr,
            partitions,
            disk_guid: None,
        })
    }

    /// Parse a single MBR partition entry
    fn parse_mbr_entry(data: &[u8], number: u32) -> Result<Option<Partition>, VFatError> {
        if data.len() < 16 {
            return Err(VFatError::Corruption(
                "MBR entry too short".to_string()
            ));
        }

        let entry = Self::parse_mbr_entry_raw(data);
        
        if let Some(entry) = entry {
            if !entry.is_valid() || entry.is_extended() {
                return Ok(None);
            }

            let start_lba = entry.start_lba as u64;
            let sector_count = entry.sector_count as u64;
            let end_lba = start_lba + sector_count.saturating_sub(1);

            Ok(Some(Partition {
                number,
                start_lba,
                end_lba,
                sector_count,
                partition_type: PartitionType::Mbr(entry.partition_type),
                bootable: entry.boot_indicator == 0x80,
                name: [0u16; 36],
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse raw MBR entry structure
    fn parse_mbr_entry_raw(data: &[u8]) -> Option<MbrPartitionEntry> {
        if data.len() < 16 {
            return None;
        }

        Some(MbrPartitionEntry {
            boot_indicator: data[0],
            start_chs: [data[1], data[2], data[3]],
            partition_type: data[4],
            end_chs: [data[5], data[6], data[7]],
            start_lba: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            sector_count: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
        })
    }

    /// Parse extended partition chain
    fn parse_extended_partitions<B: BlockDevice>(
        device: &mut B,
        extended_start: u64,
        partitions: &mut Vec<Partition>,
    ) -> Result<(), VFatError> {
        let mut current_ebr = extended_start;
        let mut logical_number = 5u32; // Logical partitions start at 5

        loop {
            let mut sector = vec![0u8; device.block_size()];
            device.read_block(current_ebr, &mut sector)?;

            // Parse first entry (actual logical partition)
            if let Some(entry) = Self::parse_mbr_entry_raw(&sector[446..462]) {
                if entry.is_valid() && !entry.is_extended() {
                    let start_lba = extended_start + entry.start_lba as u64;
                    let sector_count = entry.sector_count as u64;
                    let end_lba = start_lba + sector_count.saturating_sub(1);

                    partitions.push(Partition {
                        number: logical_number,
                        start_lba,
                        end_lba,
                        sector_count,
                        partition_type: PartitionType::Mbr(entry.partition_type),
                        bootable: entry.boot_indicator == 0x80,
                        name: [0u16; 36],
                    });

                    logical_number += 1;
                }
            }

            // Parse second entry (next EBR in chain)
            if let Some(entry) = Self::parse_mbr_entry_raw(&sector[462..478]) {
                if entry.is_extended() {
                    current_ebr = extended_start + entry.start_lba as u64;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Parse GPT partition table
    fn parse_gpt<B: BlockDevice>(device: &mut B) -> Result<Self, VFatError> {
        let block_size = device.block_size();
        let mut sector = vec![0u8; block_size];

        // Read GPT header (LBA 1)
        device.read_block(1, &mut sector)?;
        
        // Parse header
        let header = Self::parse_gpt_header(&sector)?;

        // Verify signature
        if &header.signature != b"EFI PART" {
            return Err(VFatError::Corruption(
                "Invalid GPT signature".to_string()
            ));
        }

        let disk_guid = Self::guid_to_string(&header.disk_guid);

        // Read partition entries
        let mut partitions = Vec::new();
        let entries_per_sector = block_size / header.partition_entry_size as usize;
        let num_sectors = (header.num_partition_entries as usize + entries_per_sector - 1) / entries_per_sector;

        for sector_idx in 0..num_sectors {
            device.read_block(
                header.partition_entry_lba + sector_idx as u64,
                &mut sector,
            )?;

            for entry_idx in 0..entries_per_sector {
                let global_idx = sector_idx * entries_per_sector + entry_idx;
                if global_idx >= header.num_partition_entries as usize {
                    break;
                }

                let offset = entry_idx * header.partition_entry_size as usize;
                if let Some(partition) = Self::parse_gpt_entry(
                    &sector[offset..offset + header.partition_entry_size as usize],
                    (global_idx + 1) as u32,
                )? {
                    partitions.push(partition);
                }
            }
        }

        Ok(Self {
            table_type: PartitionTableType::Gpt,
            partitions,
            disk_guid: Some(disk_guid),
        })
    }

    /// Parse GPT header
    fn parse_gpt_header(data: &[u8]) -> Result<GptHeader, VFatError> {
        if data.len() < 92 {
            return Err(VFatError::Corruption(
                "GPT header too short".to_string()
            ));
        }

        Ok(GptHeader {
            signature: [data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]],
            revision: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            header_size: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            header_crc32: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            reserved: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            current_lba: u64::from_le_bytes([
                data[24], data[25], data[26], data[27],
                data[28], data[29], data[30], data[31],
            ]),
            backup_lba: u64::from_le_bytes([
                data[32], data[33], data[34], data[35],
                data[36], data[37], data[38], data[39],
            ]),
            first_usable_lba: u64::from_le_bytes([
                data[40], data[41], data[42], data[43],
                data[44], data[45], data[46], data[47],
            ]),
            last_usable_lba: u64::from_le_bytes([
                data[48], data[49], data[50], data[51],
                data[52], data[53], data[54], data[55],
            ]),
            disk_guid: [
                data[56], data[57], data[58], data[59],
                data[60], data[61], data[62], data[63],
                data[64], data[65], data[66], data[67],
                data[68], data[69], data[70], data[71],
            ],
            partition_entry_lba: u64::from_le_bytes([
                data[72], data[73], data[74], data[75],
                data[76], data[77], data[78], data[79],
            ]),
            num_partition_entries: u32::from_le_bytes([data[80], data[81], data[82], data[83]]),
            partition_entry_size: u32::from_le_bytes([data[84], data[85], data[86], data[87]]),
            partition_entry_crc32: u32::from_le_bytes([data[88], data[89], data[90], data[91]]),
        })
    }

    /// Parse GPT partition entry
    fn parse_gpt_entry(data: &[u8], number: u32) -> Result<Option<Partition>, VFatError> {
        if data.len() < 128 {
            return Err(VFatError::Corruption(
                "GPT entry too short".to_string()
            ));
        }

        // Check if entry is empty (all zeros in type GUID)
        if data[0..16].iter().all(|&b| b == 0) {
            return Ok(None);
        }

        let type_guid = Self::guid_to_string(&[
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
            data[8], data[9], data[10], data[11],
            data[12], data[13], data[14], data[15],
        ]);

        let start_lba = u64::from_le_bytes([
            data[32], data[33], data[34], data[35],
            data[36], data[37], data[38], data[39],
        ]);

        let end_lba = u64::from_le_bytes([
            data[40], data[41], data[42], data[43],
            data[44], data[45], data[46], data[47],
        ]);

        let attributes = u64::from_le_bytes([
            data[48], data[49], data[50], data[51],
            data[52], data[53], data[54], data[55],
        ]);

        // Parse name (UTF-16LE)
        let mut name = [0u16; 36];
        for i in 0..36 {
            let offset = 56 + i * 2;
            if offset + 1 < data.len() {
                name[i] = u16::from_le_bytes([data[offset], data[offset + 1]]);
            }
        }

        Ok(Some(Partition {
            number,
            start_lba,
            end_lba,
            sector_count: end_lba - start_lba + 1,
            partition_type: PartitionType::Gpt(type_guid),
            bootable: (attributes & 0x04) != 0, // Legacy BIOS bootable
            name,
        }))
    }

    /// Convert GUID bytes to string
    fn guid_to_string(guid: &[u8; 16]) -> String {
        format!(
            "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            guid[3], guid[2], guid[1], guid[0],
            guid[5], guid[4],
            guid[7], guid[6],
            guid[8], guid[9],
            guid[10], guid[11], guid[12], guid[13], guid[14], guid[15]
        )
    }

    /// Find the boot partition
    pub fn find_boot_partition(&self) -> Option<&Partition> {
        self.partitions.iter().find(|p| p.is_boot())
    }

    /// Find the first FAT32 partition
    pub fn find_fat32_partition(&self) -> Option<&Partition> {
        self.partitions.iter().find(|p| p.is_fat32())
    }

    /// Get partition by number
    pub fn get_partition(&self, number: u32) -> Option<&Partition> {
        self.partitions.iter().find(|p| p.number == number)
    }
}

/// Partition-aware block device wrapper
pub struct PartitionBlockDevice<'a, B: BlockDevice> {
    inner: &'a mut B,
    partition: Partition,
}

impl<'a, B: BlockDevice> PartitionBlockDevice<'a, B> {
    /// Create a new partition block device
    pub fn new(inner: &'a mut B, partition: Partition) -> Self {
        Self { inner, partition }
    }

    /// Get partition info
    pub fn partition_info(&self) -> &Partition {
        &self.partition
    }
}

impl<'a, B: BlockDevice> BlockDevice for PartitionBlockDevice<'a, B> {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), VFatError> {
        if block >= self.partition.sector_count {
            return Err(VFatError::InvalidParameter(
                format!("Block {} beyond partition boundary", block)
            ));
        }
        self.inner.read_block(self.partition.start_lba + block, buffer)
    }

    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), VFatError> {
        if block >= self.partition.sector_count {
            return Err(VFatError::InvalidParameter(
                format!("Block {} beyond partition boundary", block)
            ));
        }
        self.inner.write_block(self.partition.start_lba + block, buffer)
    }

    fn read_blocks(&mut self, start_block: u64, count: usize, buffer: &mut [u8]) -> Result<(), VFatError> {
        if start_block + count as u64 > self.partition.sector_count {
            return Err(VFatError::InvalidParameter(
                "Block range beyond partition boundary".to_string()
            ));
        }
        self.inner.read_blocks(self.partition.start_lba + start_block, count, buffer)
    }

    fn write_blocks(&mut self, start_block: u64, count: usize, buffer: &[u8]) -> Result<(), VFatError> {
        if start_block + count as u64 > self.partition.sector_count {
            return Err(VFatError::InvalidParameter(
                "Block range beyond partition boundary".to_string()
            ));
        }
        self.inner.write_blocks(self.partition.start_lba + start_block, count, buffer)
    }

    fn capacity(&self) -> u64 {
        self.partition.sector_count
    }

    fn block_size(&self) -> usize {
        self.inner.block_size()
    }

    fn flush(&mut self) -> Result<(), VFatError> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::block::VirtualBlockDevice;

    fn create_mbr_disk() -> VirtualBlockDevice {
        let mut disk = VirtualBlockDevice::new(100);
        let mut data = disk.data_mut();

        // Create MBR with signature
        data[510] = 0x55;
        data[511] = 0xAA;

        // Create first partition entry (FAT32, starting at sector 2048, 65536 sectors)
        let entry_offset = 446;
        data[entry_offset] = 0x80; // Bootable
        data[entry_offset + 4] = partition_types::FAT32_LBA;
        data[entry_offset + 8..12].copy_from_slice(&2048u32.to_le_bytes());
        data[entry_offset + 12..16].copy_from_slice(&65536u32.to_le_bytes());

        disk
    }

    #[test]
    fn test_mbr_parsing() {
        let mut disk = create_mbr_disk();
        let table = PartitionTable::read(&mut disk).unwrap();

        assert_eq!(table.table_type, PartitionTableType::Mbr);
        assert_eq!(table.partitions.len(), 1);
        
        let part = &table.partitions[0];
        assert_eq!(part.number, 1);
        assert_eq!(part.start_lba, 2048);
        assert_eq!(part.sector_count, 65536);
        assert!(part.bootable);
        assert!(part.is_fat32());
    }

    #[test]
    fn test_partition_operations() {
        let mut disk = create_mbr_disk();
        let table = PartitionTable::read(&mut disk).unwrap();
        let partition = table.partitions[0].clone();

        let mut part_dev = PartitionBlockDevice::new(&mut disk, partition);
        
        // Write to partition
        let write_data = vec![0xABu8; 512];
        part_dev.write_block(0, &write_data).unwrap();

        // Read back
        let mut read_data = vec![0u8; 512];
        part_dev.read_block(0, &mut read_data).unwrap();
        assert_eq!(write_data, read_data);
    }
}
