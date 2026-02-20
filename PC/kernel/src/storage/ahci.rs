//! AHCI (Advanced Host Controller Interface) Driver
//!
//! Supports SATA drives in AHCI mode.

use alloc::vec::Vec;
use alloc::boxed::Box;
use core::mem::size_of;

use crate::storage::{BlockDevice, StorageError};
use crate::drivers::pci::{self, PciDevice};
use crate::mm::virt_to_phys_u64;
use crate::println;

/// AHCI PCI class/subclass
const SATA_CLASS: u8 = 0x01;
const SATA_SUBCLASS: u8 = 0x06;
const AHCI_PROGIF: u8 = 0x01;

/// AHCI memory registers offsets
const REG_GHC: usize = 0x04;     // Global Host Control
const REG_IS: usize = 0x08;      // Interrupt Status
const REG_PI: usize = 0x0C;      // Ports Implemented
const REG_VS: usize = 0x10;      // Version
const REG_CAP: usize = 0x00;     // Host Capabilities
const REG_CAP2: usize = 0x24;    // Host Capabilities Extended

/// Port registers (relative to port base)
const PORT_CLB: usize = 0x00;    // Command List Base Address
const PORT_CLBU: usize = 0x04;   // Command List Base Address Upper 32-bits
const PORT_FB: usize = 0x08;     // FIS Base Address
const PORT_FBU: usize = 0x0C;    // FIS Base Address Upper 32-bits
const PORT_IS: usize = 0x10;     // Interrupt Status
const PORT_IE: usize = 0x14;     // Interrupt Enable
const PORT_CMD: usize = 0x18;    // Command and Status
const PORT_TFD: usize = 0x20;    // Task File Data
const PORT_SIG: usize = 0x24;    // Signature
const PORT_SSTS: usize = 0x28;   // SATA Status
const PORT_SCTL: usize = 0x2C;   // SATA Control
const PORT_SERR: usize = 0x30;   // SATA Error
const PORT_SACT: usize = 0x34;   // SATA Active
const PORT_CI: usize = 0x38;     // Command Issue

/// Port command bits
const PORT_CMD_ST: u32 = 0x0001;  // Start
const PORT_CMD_FRE: u32 = 0x0010; // FIS Receive Enable
const PORT_CMD_FR: u32 = 0x4000;  // FIS Receive Running
const PORT_CMD_CR: u32 = 0x8000;  // Command List Running

/// Port signature values
const SIG_SATA: u32 = 0x00000101;
const SIG_ATAPI: u32 = 0xEB140101;
const SIG_SEMB: u32 = 0xC33C0101;
const SIG_PM: u32 = 0x96690101;

/// FIS types
const FIS_TYPE_REG_H2D: u8 = 0x27;

/// Command header flags
const CMDH_FIS_LEN: u16 = 5;  // 20 bytes / 4 = 5 DWs
const CMDH_WRITE: u16 = 0x0040;

/// SATA commands
const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;
const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_FLUSH_CACHE_EXT: u8 = 0xEA;

/// AHCI controller structure
pub struct AhciController {
    base_addr: *mut u8,
    ports: Vec<AhciPort>,
}

/// AHCI port structure
pub struct AhciPort {
    port_num: u32,
    base: *mut u8,
    cmd_list: *mut CommandHeader,
    cmd_table: *mut CommandTable,
    fis: *mut ReceivedFIS,
    buffer: *mut u8,
    sector_count: u64,
    model: [u8; 40],
    is_atapi: bool,
}

/// Command Header (1KB aligned, 32 bytes each)
#[repr(C, align(128))]
struct CommandHeader {
    flags: u16,      // Flags (FIS length, etc.)
    prdtl: u16,      // Physical Region Descriptor Table Length
    prdbc: u32,      // Physical Region Descriptor Byte Count
    ctba: u64,       // Command Table Base Address
    reserved: [u32; 4],
}

/// Command Table
#[repr(C, align(128))]
struct CommandTable {
    cfis: [u8; 64],      // Command FIS (up to 64 bytes)
    acmd: [u8; 16],      // ATAPI command (12 or 16 bytes)
    reserved: [u8; 48],  // Reserved
    prdt: [PRDTEntry; 1], // Physical Region Descriptor Table (variable)
}

/// PRDT Entry
#[repr(C)]
struct PRDTEntry {
    dba: u64,        // Data Base Address
    reserved: u32,
    dbc: u32,        // Data Byte Count (0-based, bit 31 = interrupt)
}

/// Received FIS structure
#[repr(C, align(256))]
struct ReceivedFIS {
    dsfis: [u8; 28],   // DMA Setup FIS
    pad1: [u8; 4],
    psfis: [u8; 20],   // PIO Setup FIS
    pad2: [u8; 12],
    rfis: [u8; 20],    // D2H Register FIS
    pad3: [u8; 4],
    sdbfis: [u8; 8],   // Set Device Bits FIS
    ufis: [u8; 64],    // Unknown FIS
    reserved: [u8; 96],
}

/// H2D Register FIS
#[repr(C)]
struct FISRegH2D {
    fis_type: u8,
    flags: u8,       // Port multiplier and C bit
    command: u8,
    featurel: u8,
    lba0: u8,
    lba1: u8,
    lba2: u8,
    device: u8,
    lba3: u8,
    lba4: u8,
    lba5: u8,
    featureh: u8,
    countl: u8,
    counth: u8,
    icc: u8,
    control: u8,
    reserved: [u8; 4],
}

// SAFETY: AhciPort is only accessed from a single thread
unsafe impl Send for AhciPort {}
unsafe impl Sync for AhciPort {}

impl AhciPort {
    /// Create new AHCI port
    pub fn new(port_num: u32, base: *mut u8) -> Option<Self> {
        // Allocate memory for structures
        let cmd_list = alloc_dma_aligned(1024, 1024)? as *mut CommandHeader;
        let cmd_table = alloc_dma_aligned(1024, 128)? as *mut CommandTable;
        let fis = alloc_dma_aligned(256, 256)? as *mut ReceivedFIS;
        let buffer = alloc_dma_aligned(8192, 4096)?;

        Some(Self {
            port_num,
            base,
            cmd_list,
            cmd_table,
            fis,
            buffer,
            sector_count: 0,
            model: [0; 40],
            is_atapi: false,
        })
    }

    /// Initialize port
    pub fn init(&mut self) -> Result<(), StorageError> {
        // Stop command engine
        self.stop_command_engine()?;

        // Set up command list and FIS
        let clb = virt_to_phys_u64(self.cmd_list as u64);
        let fb = virt_to_phys_u64(self.fis as u64);

        unsafe {
            write_reg(self.base, PORT_CLB, clb as u32);
            write_reg(self.base, PORT_CLBU, (clb >> 32) as u32);
            write_reg(self.base, PORT_FB, fb as u32);
            write_reg(self.base, PORT_FBU, (fb >> 32) as u32);
        }

        // Clear interrupt status
        unsafe {
            write_reg(self.base, PORT_IS, 0xFFFFFFFF);
        }

        // Start command engine
        self.start_command_engine()?;

        // Check device signature
        let sig = unsafe { read_reg(self.base, PORT_SIG) };
        match sig {
            SIG_SATA => {
                self.is_atapi = false;
            }
            SIG_ATAPI => {
                self.is_atapi = true;
                return Err(StorageError::NotFound); // Skip ATAPI for now
            }
            _ => {
                return Err(StorageError::NotFound);
            }
        }

        // Identify device
        self.identify()?;

        Ok(())
    }

    /// Stop command engine
    fn stop_command_engine(&mut self) -> Result<(), StorageError> {
        let mut cmd = unsafe { read_reg(self.base, PORT_CMD) };
        
        // Clear ST and FRE bits
        cmd &= !(PORT_CMD_ST | PORT_CMD_FRE);
        unsafe {
            write_reg(self.base, PORT_CMD, cmd);
        }

        // Wait for CR and FR to clear
        let timeout = 1000000;
        for i in 0..timeout {
            cmd = unsafe { read_reg(self.base, PORT_CMD) };
            if cmd & (PORT_CMD_CR | PORT_CMD_FR) == 0 {
                return Ok(());
            }
            if i % 1000 == 0 {
                core::hint::spin_loop();
            }
        }

        Err(StorageError::Timeout)
    }

    /// Start command engine
    fn start_command_engine(&mut self) -> Result<(), StorageError> {
        // Set FRE first
        let mut cmd = unsafe { read_reg(self.base, PORT_CMD) };
        cmd |= PORT_CMD_FRE;
        unsafe {
            write_reg(self.base, PORT_CMD, cmd);
        }

        // Then set ST
        cmd |= PORT_CMD_ST;
        unsafe {
            write_reg(self.base, PORT_CMD, cmd);
        }

        Ok(())
    }

    /// Identify device
    fn identify(&mut self) -> Result<(), StorageError> {
        // Set up command header
        unsafe {
            (*self.cmd_list).flags = CMDH_FIS_LEN;
            (*self.cmd_list).prdtl = 1;
            (*self.cmd_list).ctba = virt_to_phys_u64(self.cmd_table as u64);

            // Set up PRDT
            (*self.cmd_table).prdt[0].dba = virt_to_phys_u64(self.buffer as u64);
            (*self.cmd_table).prdt[0].dbc = 511 | (1 << 31); // 512 bytes, interrupt on completion

            // Build FIS
            let fis = &mut (*self.cmd_table).cfis as *mut u8 as *mut FISRegH2D;
            core::ptr::write_bytes(fis, 0, 1);
            (*fis).fis_type = FIS_TYPE_REG_H2D;
            (*fis).flags = 1 << 7; // C bit set
            (*fis).command = ATA_CMD_IDENTIFY;
            (*fis).device = 0;
        }

        // Issue command
        unsafe {
            write_reg(self.base, PORT_CI, 1);
        }

        // Wait for completion
        self.wait_command()?;

        // Parse identify data
        let id_data = unsafe { core::slice::from_raw_parts(self.buffer as *mut u16, 256) };
        
        // Get model name (words 27-46, byte swapped)
        for i in 0..20 {
            let word = id_data[27 + i];
            self.model[i * 2] = (word >> 8) as u8;
            self.model[i * 2 + 1] = (word & 0xFF) as u8;
        }

        // Get sector count (LBA48 words 100-103, or LBA28 words 60-61)
        let lba48_sectors = 
            (id_data[100] as u64) |
            ((id_data[101] as u64) << 16) |
            ((id_data[102] as u64) << 32) |
            ((id_data[103] as u64) << 48);
        
        if lba48_sectors > 0 {
            self.sector_count = lba48_sectors;
        } else {
            self.sector_count = (id_data[60] as u64) | ((id_data[61] as u64) << 16);
        }

        Ok(())
    }

    /// Wait for command completion
    fn wait_command(&self) -> Result<(), StorageError> {
        let timeout = 10000000;
        
        for i in 0..timeout {
            let ci = unsafe { read_reg(self.base, PORT_CI) };
            if ci & 1 == 0 {
                // Check for errors
                let tfd = unsafe { read_reg(self.base, PORT_TFD) };
                if tfd & 0x01 != 0 {
                    return Err(StorageError::IoError);
                }
                return Ok(());
            }
            if i % 1000 == 0 {
                core::hint::spin_loop();
            }
        }

        Err(StorageError::Timeout)
    }

    /// Read sectors
    fn read_sectors(&self, lba: u64, count: u16, buf: &mut [u8]) -> Result<(), StorageError> {
        if count == 0 || count > 256 {
            return Err(StorageError::InvalidArgument);
        }

        // Set up command
        unsafe {
            (*self.cmd_list).flags = CMDH_FIS_LEN;
            (*self.cmd_list).prdtl = 1;
            (*self.cmd_list).ctba = virt_to_phys_u64(self.cmd_table as u64);

            // Set up PRDT - use internal buffer for now
            (*self.cmd_table).prdt[0].dba = virt_to_phys_u64(self.buffer as u64);
            (*self.cmd_table).prdt[0].dbc = ((count as u32) * 512 - 1) | (1 << 31);

            // Build FIS
            let fis = &mut (*self.cmd_table).cfis as *mut u8 as *mut FISRegH2D;
            core::ptr::write_bytes(fis, 0, 1);
            (*fis).fis_type = FIS_TYPE_REG_H2D;
            (*fis).flags = 1 << 7;
            (*fis).command = ATA_CMD_READ_DMA_EXT;
            (*fis).lba0 = (lba & 0xFF) as u8;
            (*fis).lba1 = ((lba >> 8) & 0xFF) as u8;
            (*fis).lba2 = ((lba >> 16) & 0xFF) as u8;
            (*fis).device = 1 << 6; // LBA mode
            (*fis).lba3 = ((lba >> 24) & 0xFF) as u8;
            (*fis).lba4 = ((lba >> 32) & 0xFF) as u8;
            (*fis).lba5 = ((lba >> 40) & 0xFF) as u8;
            (*fis).countl = (count & 0xFF) as u8;
            (*fis).counth = ((count >> 8) & 0xFF) as u8;
        }

        // Issue command
        unsafe {
            write_reg(self.base, PORT_CI, 1);
        }

        // Wait for completion
        self.wait_command()?;

        // Copy data to buffer
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.buffer,
                buf.as_mut_ptr(),
                (count as usize) * 512
            );
        }

        Ok(())
    }

    /// Write sectors
    fn write_sectors(&self, lba: u64, count: u16, buf: &[u8]) -> Result<(), StorageError> {
        if count == 0 || count > 256 {
            return Err(StorageError::InvalidArgument);
        }

        // Copy data from buffer
        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.buffer,
                (count as usize) * 512
            );
        }

        // Set up command
        unsafe {
            (*self.cmd_list).flags = CMDH_FIS_LEN | CMDH_WRITE;
            (*self.cmd_list).prdtl = 1;
            (*self.cmd_list).ctba = virt_to_phys_u64(self.cmd_table as u64);

            // Set up PRDT
            (*self.cmd_table).prdt[0].dba = virt_to_phys_u64(self.buffer as u64);
            (*self.cmd_table).prdt[0].dbc = ((count as u32) * 512 - 1) | (1 << 31);

            // Build FIS
            let fis = &mut (*self.cmd_table).cfis as *mut u8 as *mut FISRegH2D;
            core::ptr::write_bytes(fis, 0, 1);
            (*fis).fis_type = FIS_TYPE_REG_H2D;
            (*fis).flags = 1 << 7;
            (*fis).command = ATA_CMD_WRITE_DMA_EXT;
            (*fis).lba0 = (lba & 0xFF) as u8;
            (*fis).lba1 = ((lba >> 8) & 0xFF) as u8;
            (*fis).lba2 = ((lba >> 16) & 0xFF) as u8;
            (*fis).device = 1 << 6;
            (*fis).lba3 = ((lba >> 24) & 0xFF) as u8;
            (*fis).lba4 = ((lba >> 32) & 0xFF) as u8;
            (*fis).lba5 = ((lba >> 40) & 0xFF) as u8;
            (*fis).countl = (count & 0xFF) as u8;
            (*fis).counth = ((count >> 8) & 0xFF) as u8;
        }

        // Issue command
        unsafe {
            write_reg(self.base, PORT_CI, 1);
        }

        // Wait for completion
        self.wait_command()?;

        // Flush cache
        self.flush()
    }

    /// Flush cache
    fn flush(&self) -> Result<(), StorageError> {
        unsafe {
            (*self.cmd_list).flags = CMDH_FIS_LEN;
            (*self.cmd_list).prdtl = 0;

            let fis = &mut (*self.cmd_table).cfis as *mut u8 as *mut FISRegH2D;
            core::ptr::write_bytes(fis, 0, 1);
            (*fis).fis_type = FIS_TYPE_REG_H2D;
            (*fis).flags = 1 << 7;
            (*fis).command = ATA_CMD_FLUSH_CACHE_EXT;

            write_reg(self.base, PORT_CI, 1);
        }

        self.wait_command()
    }
}

impl BlockDevice for AhciPort {
    fn name(&self) -> &str {
        // Static name based on port number
        match self.port_num {
            0 => "sda",
            1 => "sdb",
            2 => "sdc",
            3 => "sdd",
            _ => "sdx",
        }
    }

    fn block_size(&self) -> usize {
        512
    }

    fn block_count(&self) -> u64 {
        self.sector_count
    }

    fn read_blocks(&self, start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError> {
        if count == 0 {
            return Ok(());
        }

        // AHCI can handle up to 65536 sectors at once
        let max_count = 256; // Be conservative for now
        
        if count > max_count {
            let mut offset = 0;
            let mut remaining = count;
            let mut current_lba = start;

            while remaining > 0 {
                let to_read = remaining.min(max_count);
                self.read_blocks(current_lba, to_read, &mut buf[offset..offset + to_read * 512])?;
                offset += to_read * 512;
                remaining -= to_read;
                current_lba += to_read as u64;
            }
            return Ok(());
        }

        self.read_sectors(start, count as u16, buf)
    }

    fn write_blocks(&self, start: u64, count: usize, buf: &[u8]) -> Result<(), StorageError> {
        if count == 0 {
            return Ok(());
        }

        let max_count = 256;
        
        if count > max_count {
            let mut offset = 0;
            let mut remaining = count;
            let mut current_lba = start;

            while remaining > 0 {
                let to_write = remaining.min(max_count);
                self.write_blocks(current_lba, to_write, &buf[offset..offset + to_write * 512])?;
                offset += to_write * 512;
                remaining -= to_write;
                current_lba += to_write as u64;
            }
            return Ok(());
        }

        self.write_sectors(start, count as u16, buf)
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.flush()
    }
}

/// Initialize AHCI controller
pub fn init() {
    println!("[ahci] Probing for AHCI controllers...");

    // Find AHCI controller on PCI
    if let Some(device) = pci::find_device(SATA_CLASS, SATA_SUBCLASS) {
        // Check programming interface for AHCI mode
        if device.prog_if != AHCI_PROGIF {
            println!("[ahci] Controller not in AHCI mode");
            return;
        }

        println!("[ahci] Found AHCI controller at {:02X}:{:02X}.{}",
            device.bus, device.device, device.function);

        // Read BAR5 (AHCI base address)
        let bar5 = device.read_config(0x24);
        let base_addr = if bar5 & 1 == 0 {
            (bar5 & 0xFFFFFFF0) as u64
        } else {
            println!("[ahci] Unexpected I/O BAR");
            return;
        };

        // Map the memory region (for now assume it's identity mapped)
        let ahci_base = (base_addr + crate::mm::PHYSICAL_MEMORY_OFFSET) as *mut u8;

        // Read capabilities
        let cap = unsafe { read_reg(ahci_base, REG_CAP) };
        let port_count = ((cap >> 0) & 0x1F) + 1; // Number of ports
        let cmd_slots = ((cap >> 8) & 0x1F) + 1;  // Number of command slots

        println!("[ahci] Ports: {}, Command slots: {}", port_count, cmd_slots);

        // Read ports implemented bitmap
        let pi = unsafe { read_reg(ahci_base, REG_PI) };

        // Enable AHCI mode
        let ghc = unsafe { read_reg(ahci_base, REG_GHC) };
        unsafe {
            write_reg(ahci_base, REG_GHC, ghc | 0x80000000); // AHCI Enable
        }

        // Probe each implemented port
        for port in 0..32 {
            if pi & (1 << port) == 0 {
                continue;
            }

            let port_base = unsafe { ahci_base.add(0x100 + port * 0x80) };

            if let Some(mut ahci_port) = AhciPort::new(port as u32, port_base) {
                if ahci_port.init().is_ok() {
                    let model = core::str::from_utf8(&ahci_port.model)
                        .unwrap_or("Unknown")
                        .trim();
                    println!("[ahci] Port {}: {} ({} sectors)",
                        port, model, ahci_port.sector_count);
                    
                    crate::storage::register_device(Box::new(ahci_port));
                } else {
                    println!("[ahci] Port {}: No device or initialization failed", port);
                }
            }
        }
    }
}

/// Read AHCI register
unsafe fn read_reg(base: *mut u8, offset: usize) -> u32 {
    core::ptr::read_volatile(base.add(offset) as *mut u32)
}

/// Write AHCI register
unsafe fn write_reg(base: *mut u8, offset: usize, value: u32) {
    core::ptr::write_volatile(base.add(offset) as *mut u32, value);
}

/// Allocate DMA-aligned memory
fn alloc_dma_aligned(size: usize, align: usize) -> Option<*mut u8> {
    use alloc::alloc::{alloc_zeroed, Layout};
    
    let layout = Layout::from_size_align(size, align).ok()?;
    let ptr = unsafe { alloc_zeroed(layout) };
    
    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}
