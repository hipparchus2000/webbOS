//! NVMe (Non-Volatile Memory Express) Driver
//!
//! High-performance SSD driver for NVMe controllers.

use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::storage::{BlockDevice, StorageError};
use crate::drivers::pci::{self, PciDevice};
use crate::mm::virt_to_phys_u64;
use crate::println;

/// NVMe PCI class/subclass
const NVME_CLASS: u8 = 0x01;
const NVME_SUBCLASS: u8 = 0x08;

/// NVMe registers offsets (in BAR0)
const REG_CAP: usize = 0x0000;    // Controller Capabilities
const REG_VS: usize = 0x0008;     // Version
const REG_INTMS: usize = 0x000C;  // Interrupt Mask Set
const REG_INTMC: usize = 0x000E;  // Interrupt Mask Clear
const REG_CC: usize = 0x0014;     // Controller Configuration
const REG_CSTS: usize = 0x001C;   // Controller Status
const REG_NSSR: usize = 0x0020;   // NVM Subsystem Reset
const REG_AQA: usize = 0x0024;    // Admin Queue Attributes
const REG_ASQ: usize = 0x0028;    // Admin Submission Queue Base
const REG_ACQ: usize = 0x0030;    // Admin Completion Queue Base

/// Doorbell offsets
const DOORBELL_STRIDE: usize = 4; // Each doorbell is 4 bytes

/// Controller Configuration bits
const CC_EN: u32 = 0x01;
const CC_IOSQES: u32 = 6 << 16;  // IO SQ Entry Size = 64 bytes
const CC_IOCQES: u32 = 4 << 20;  // IO CQ Entry Size = 16 bytes
const CC_SHN_NONE: u32 = 0 << 14;
const CC_AMS_RR: u32 = 0 << 11;  // Round-robin arbitration
const CC_CSS_NVM: u32 = 0 << 4;  // NVM command set

/// Controller Status bits
const CSTS_RDY: u32 = 0x01;
const CSTS_CFS: u32 = 0x02;

/// Admin opcodes
const CMD_DELETE_SQ: u8 = 0x00;
const CMD_CREATE_SQ: u8 = 0x01;
const CMD_DELETE_CQ: u8 = 0x04;
const CMD_CREATE_CQ: u8 = 0x05;
const CMD_IDENTIFY: u8 = 0x06;

/// NVM opcodes
const CMD_READ: u8 = 0x02;
const CMD_WRITE: u8 = 0x01;
const CMD_FLUSH: u8 = 0x00;

/// Identify CNS values
const CNS_NAMESPACE: u32 = 0x00;
const CNS_CONTROLLER: u32 = 0x01;
const CNS_NS_LIST: u32 = 0x02;

/// Submission queue entry (64 bytes)
#[repr(C)]
struct SQEntry {
    opcode: u8,
    flags: u8,
    cid: u16,
    nsid: u32,
    reserved1: u64,
    mptr: u64,
    dptr: [u64; 2],
    cdw10: u32,
    cdw11: u32,
    cdw12: u32,
    cdw13: u32,
    cdw14: u32,
    cdw15: u32,
}

/// Completion queue entry (16 bytes minimum)
#[repr(C)]
struct CQEntry {
    result: u32,
    reserved: u32,
    sqhd: u16,
    sqid: u16,
    cid: u16,
    status: u16,
}

/// NVMe controller structure
pub struct NvmeController {
    base_addr: *mut u8,
    admin_sq: *mut SQEntry,
    admin_cq: *mut CQEntry,
    io_sq: *mut SQEntry,
    io_cq: *mut CQEntry,
    admin_sq_tail: u16,
    admin_cq_head: u16,
    io_sq_tail: u16,
    io_cq_head: u16,
    admin_doorbell: *mut u32,
    io_sq_doorbell: *mut u32,
    io_cq_doorbell: *mut u32,
    sq_entry_size: usize,
    cq_entry_size: usize,
    namespace_id: u32,
    sector_count: u64,
    sector_size: u64,
    model: [u8; 40],
    serial: [u8; 20],
}

// SAFETY: NVMe types are only accessed from a single thread
unsafe impl Send for NvmeController {}
unsafe impl Sync for NvmeController {}
unsafe impl Send for NvmeNamespace {}
unsafe impl Sync for NvmeNamespace {}

/// NVMe namespace (represents a single logical drive)
pub struct NvmeNamespace {
    controller: *mut NvmeController,
    nsid: u32,
    sector_count: u64,
    sector_size: u64,
    model: [u8; 40],
}

impl NvmeController {
    /// Create and initialize NVMe controller
    pub fn new(base_addr: *mut u8) -> Option<Self> {
        let admin_sq = alloc_dma(4096, 4096)? as *mut SQEntry;
        let admin_cq = alloc_dma(4096, 4096)? as *mut CQEntry;
        let io_sq = alloc_dma(4096, 4096)? as *mut SQEntry;
        let io_cq = alloc_dma(4096, 4096)? as *mut CQEntry;

        Some(Self {
            base_addr,
            admin_sq,
            admin_cq,
            io_sq,
            io_cq,
            admin_sq_tail: 0,
            admin_cq_head: 0,
            io_sq_tail: 0,
            io_cq_head: 0,
            admin_doorbell: unsafe { base_addr.add(0x1000) as *mut u32 },
            io_sq_doorbell: unsafe { base_addr.add(0x1000 + 1 * (4 << 0)) as *mut u32 },
            io_cq_doorbell: unsafe { base_addr.add(0x1000 + 2 * (4 << 0)) as *mut u32 },
            sq_entry_size: 64,
            cq_entry_size: 16,
            namespace_id: 0,
            sector_count: 0,
            sector_size: 512,
            model: [0; 40],
            serial: [0; 20],
        })
    }

    /// Initialize controller
    pub fn init(&mut self) -> Result<(), StorageError> {
        // Check capabilities
        let cap = self.read_cap();
        let doorbell_stride = 4 << ((cap >> 32) & 0xF); // DSTRD field

        println!("[nvme] CAP: {:016X}", cap);

        // Disable controller
        self.write_reg(REG_CC, 0);
        
        // Wait for controller to disable
        let timeout = 1000000;
        for i in 0..timeout {
            let csts = self.read_reg(REG_CSTS);
            if csts & CSTS_RDY == 0 {
                break;
            }
            if i % 1000 == 0 {
                core::hint::spin_loop();
            }
        }

        // Configure admin queues
        let aqa = ((4096 / 16 - 1) << 16) | (4096 / 64 - 1); // 64 entries each
        self.write_reg(REG_AQA, aqa);
        
        let asq_phys = virt_to_phys_u64(self.admin_sq as u64);
        let acq_phys = virt_to_phys_u64(self.admin_cq as u64);
        
        self.write_reg64(REG_ASQ, asq_phys);
        self.write_reg64(REG_ACQ, acq_phys);

        // Configure controller
        let cc = CC_EN | CC_IOSQES | CC_IOCQES | CC_AMS_RR | CC_CSS_NVM;
        self.write_reg(REG_CC, cc);

        // Wait for controller ready
        for i in 0..timeout {
            let csts = self.read_reg(REG_CSTS);
            if csts & CSTS_RDY != 0 {
                break;
            }
            if csts & CSTS_CFS != 0 {
                return Err(StorageError::IoError);
            }
            if i % 1000 == 0 {
                core::hint::spin_loop();
            }
        }

        // Identify controller
        self.identify_controller()?;

        // Create I/O completion queue
        self.create_io_cq()?;

        // Create I/O submission queue
        self.create_io_sq()?;

        // Identify namespace 1
        self.namespace_id = 1;
        self.identify_namespace()?;

        Ok(())
    }

    /// Read capabilities register (64-bit)
    fn read_cap(&self) -> u64 {
        let low = self.read_reg(REG_CAP);
        let high = self.read_reg(REG_CAP + 4);
        ((high as u64) << 32) | (low as u64)
    }

    /// Read register
    fn read_reg(&self, offset: usize) -> u32 {
        unsafe { core::ptr::read_volatile(self.base_addr.add(offset) as *mut u32) }
    }

    /// Write register
    fn write_reg(&self, offset: usize, value: u32) {
        unsafe { core::ptr::write_volatile(self.base_addr.add(offset) as *mut u32, value); }
    }

    /// Write 64-bit register
    fn write_reg64(&self, offset: usize, value: u64) {
        unsafe {
            core::ptr::write_volatile(self.base_addr.add(offset) as *mut u64, value);
        }
    }

    /// Submit admin command
    fn submit_admin_cmd(&mut self, opcode: u8, nsid: u32, dptr: [u64; 2], cdw10: u32, cdw11: u32) -> Result<CQEntry, StorageError> {
        let tail = self.admin_sq_tail as usize;
        
        unsafe {
            let entry = &mut *self.admin_sq.add(tail);
            core::ptr::write_bytes(entry, 0, 1);
            (*entry).opcode = opcode;
            (*entry).cid = tail as u16;
            (*entry).nsid = nsid;
            (*entry).dptr = dptr;
            (*entry).cdw10 = cdw10;
            (*entry).cdw11 = cdw11;
        }

        // Update tail doorbell
        self.admin_sq_tail = (self.admin_sq_tail + 1) % 64;
        unsafe {
            core::ptr::write_volatile(self.admin_doorbell, self.admin_sq_tail as u32);
        }

        // Wait for completion
        self.wait_completion(true)
    }

    /// Wait for command completion
    fn wait_completion(&mut self, admin: bool) -> Result<CQEntry, StorageError> {
        let cq = if admin { self.admin_cq } else { self.io_cq };
        let head = if admin { &mut self.admin_cq_head } else { &mut self.io_cq_head };
        let doorbell = if admin { unsafe { self.admin_doorbell.add(1) } } else { self.io_cq_doorbell };

        let timeout = 10000000;
        for i in 0..timeout {
            unsafe {
                let entry = &*cq.add(*head as usize);
                let status = entry.status;
                
                if status & 0x01 != 0 {
                    // Phase tag matches - new entry
                    let result = CQEntry {
                        result: entry.result,
                        reserved: entry.reserved,
                        sqhd: entry.sqhd,
                        sqid: entry.sqid,
                        cid: entry.cid,
                        status: entry.status,
                    };

                    // Update head and doorbell
                    *head = (*head + 1) % 64;
                    core::ptr::write_volatile(doorbell as *mut u32, *head as u32);

                    // Check status
                    let sc = (status >> 1) & 0xFF;
                    let sct = (status >> 9) & 0x07;
                    
                    if sc != 0 || sct != 0 {
                        return Err(StorageError::IoError);
                    }

                    return Ok(result);
                }
            }
            
            if i % 1000 == 0 {
                core::hint::spin_loop();
            }
        }

        Err(StorageError::Timeout)
    }

    /// Identify controller
    fn identify_controller(&mut self) -> Result<(), StorageError> {
        let buffer = alloc_dma(4096, 4096).ok_or(StorageError::Unknown)?;
        
        self.submit_admin_cmd(
            CMD_IDENTIFY,
            0,
            [virt_to_phys_u64(buffer as u64), 0],
            CNS_CONTROLLER,
            0
        )?;

        // Parse identify data
        unsafe {
            let data = core::slice::from_raw_parts(buffer, 4096);
            
            // Model number (bytes 24-63)
            for i in 0..40 {
                self.model[i] = data[63 - i * 2 + (i % 2) * 1];
            }
            
            // Serial number (bytes 4-23)
            for i in 0..20 {
                self.serial[i] = data[23 - i * 2 + (i % 2) * 1];
            }
        }

        Ok(())
    }

    /// Identify namespace
    fn identify_namespace(&mut self) -> Result<(), StorageError> {
        let buffer = alloc_dma(4096, 4096).ok_or(StorageError::Unknown)?;
        
        self.submit_admin_cmd(
            CMD_IDENTIFY,
            self.namespace_id,
            [virt_to_phys_u64(buffer as u64), 0],
            CNS_NAMESPACE,
            0
        )?;

        unsafe {
            let data = core::slice::from_raw_parts(buffer as *mut u64, 512);
            
            // NSZE (namespace size) at offset 0
            self.sector_count = data[0];
            
            // LBA format (at offset 128)
            let flbas = *((buffer.add(26)) as *mut u8);
            let lba_format_index = flbas & 0x0F;
            
            // Get LBA format
            let lbafs = buffer.add(128) as *mut u32;
            let lbaf = *lbafs.add(lba_format_index as usize);
            let lbads = (lbaf >> 16) & 0xFF; // LBA data size
            self.sector_size = 1u64 << lbads;
        }

        Ok(())
    }

    /// Create I/O completion queue
    fn create_io_cq(&mut self) -> Result<(), StorageError> {
        let cq_phys = virt_to_phys_u64(self.io_cq as u64);
        
        self.submit_admin_cmd(
            CMD_CREATE_CQ,
            0,
            [cq_phys, 0],
            ((4096 / 16 - 1) << 16) | 1, // Size | Queue ID
            0x0001_0001 // Interrupts enabled, physically contiguous
        )?;

        Ok(())
    }

    /// Create I/O submission queue
    fn create_io_sq(&mut self) -> Result<(), StorageError> {
        let sq_phys = virt_to_phys_u64(self.io_sq as u64);
        
        self.submit_admin_cmd(
            CMD_CREATE_SQ,
            0,
            [sq_phys, 0],
            ((4096 / 64 - 1) << 16) | 1, // Size | Queue ID
            (1 << 16) | 1 // CQ ID | Physically contiguous
        )?;

        Ok(())
    }

    /// Read sectors
    fn read_sectors(&mut self, lba: u64, count: u16, buf: *mut u8) -> Result<(), StorageError> {
        if count == 0 || count > 256 {
            return Err(StorageError::InvalidArgument);
        }

        let tail = self.io_sq_tail as usize;
        
        unsafe {
            let entry = &mut *self.io_sq.add(tail);
            core::ptr::write_bytes(entry, 0, 1);
            
            (*entry).opcode = CMD_READ;
            (*entry).cid = tail as u16;
            (*entry).nsid = self.namespace_id;
            (*entry).dptr = [virt_to_phys_u64(buf as u64), 0];
            (*entry).cdw10 = (lba & 0xFFFFFFFF) as u32;
            (*entry).cdw11 = ((lba >> 32) & 0xFFFFFFFF) as u32;
            (*entry).cdw12 = (count as u32) - 1; // 0-based count
        }

        // Update tail doorbell
        self.io_sq_tail = (self.io_sq_tail + 1) % 64;
        unsafe {
            core::ptr::write_volatile(self.io_sq_doorbell, self.io_sq_tail as u32);
        }

        // Wait for completion
        self.wait_completion(false)?;

        Ok(())
    }

    /// Write sectors
    fn write_sectors(&mut self, lba: u64, count: u16, buf: *const u8) -> Result<(), StorageError> {
        if count == 0 || count > 256 {
            return Err(StorageError::InvalidArgument);
        }

        let tail = self.io_sq_tail as usize;
        
        unsafe {
            let entry = &mut *self.io_sq.add(tail);
            core::ptr::write_bytes(entry, 0, 1);
            
            (*entry).opcode = CMD_WRITE;
            (*entry).cid = tail as u16;
            (*entry).nsid = self.namespace_id;
            (*entry).dptr = [virt_to_phys_u64(buf as u64), 0];
            (*entry).cdw10 = (lba & 0xFFFFFFFF) as u32;
            (*entry).cdw11 = ((lba >> 32) & 0xFFFFFFFF) as u32;
            (*entry).cdw12 = (count as u32) - 1;
        }

        // Update tail doorbell
        self.io_sq_tail = (self.io_sq_tail + 1) % 64;
        unsafe {
            core::ptr::write_volatile(self.io_sq_doorbell, self.io_sq_tail as u32);
        }

        // Wait for completion
        self.wait_completion(false)?;

        Ok(())
    }

    /// Flush
    fn flush(&mut self) -> Result<(), StorageError> {
        let tail = self.io_sq_tail as usize;
        
        unsafe {
            let entry = &mut *self.io_sq.add(tail);
            core::ptr::write_bytes(entry, 0, 1);
            
            (*entry).opcode = CMD_FLUSH;
            (*entry).cid = tail as u16;
            (*entry).nsid = self.namespace_id;
        }

        self.io_sq_tail = (self.io_sq_tail + 1) % 64;
        unsafe {
            core::ptr::write_volatile(self.io_sq_doorbell, self.io_sq_tail as u32);
        }

        self.wait_completion(false)?;
        Ok(())
    }
}

impl NvmeNamespace {
    /// Create namespace from controller
    pub fn from_controller(controller: *mut NvmeController, nsid: u32) -> Self {
        unsafe {
            Self {
                controller,
                nsid,
                sector_count: (*controller).sector_count,
                sector_size: (*controller).sector_size,
                model: (*controller).model,
            }
        }
    }
}

impl BlockDevice for NvmeNamespace {
    fn name(&self) -> &str {
        "nvme0n1"
    }

    fn block_size(&self) -> usize {
        self.sector_size as usize
    }

    fn block_count(&self) -> u64 {
        self.sector_count
    }

    fn read_blocks(&self, start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError> {
        if count == 0 {
            return Ok(());
        }

        // NVMe can handle up to 65535 LBAs in a single command
        let max_count = 256; // Be conservative
        
        if count > max_count {
            let mut offset = 0;
            let mut remaining = count;
            let mut current_lba = start;

            while remaining > 0 {
                let to_read = remaining.min(max_count);
                self.read_blocks(current_lba, to_read, &mut buf[offset..offset + to_read * self.sector_size as usize])?;
                offset += to_read * self.sector_size as usize;
                remaining -= to_read;
                current_lba += to_read as u64;
            }
            return Ok(());
        }

        unsafe {
            (*self.controller).read_sectors(start, count as u16, buf.as_mut_ptr())
        }
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
                self.write_blocks(current_lba, to_write, &buf[offset..offset + to_write * self.sector_size as usize])?;
                offset += to_write * self.sector_size as usize;
                remaining -= to_write;
                current_lba += to_write as u64;
            }
            return Ok(());
        }

        unsafe {
            (*self.controller).write_sectors(start, count as u16, buf.as_ptr())
        }
    }

    fn flush(&self) -> Result<(), StorageError> {
        unsafe {
            (*self.controller).flush()
        }
    }
}

/// Initialize NVMe controller
pub fn init() {
    println!("[nvme] Probing for NVMe controllers...");

    if let Some(device) = pci::find_device(NVME_CLASS, NVME_SUBCLASS) {
        println!("[nvme] Found NVMe controller at {:02X}:{:02X}.{}",
            device.bus, device.device, device.function);

        // Read BAR0
        let bar0 = device.read_config(0x10);
        let base_addr = if bar0 & 1 == 0 {
            (bar0 & 0xFFFFFFF0) as u64
        } else {
            println!("[nvme] Unexpected I/O BAR");
            return;
        };

        // Map memory
        let nvme_base = (base_addr + crate::mm::PHYSICAL_MEMORY_OFFSET) as *mut u8;

        if let Some(mut controller) = NvmeController::new(nvme_base) {
            if controller.init().is_ok() {
                let model = core::str::from_utf8(&controller.model)
                    .unwrap_or("Unknown")
                    .trim();
                let serial = core::str::from_utf8(&controller.serial)
                    .unwrap_or("Unknown")
                    .trim();
                
                println!("[nvme] {} ({})", model, serial);
                println!("[nvme] Namespace 1: {} sectors ({} MB)",
                    controller.sector_count,
                    (controller.sector_count * controller.sector_size) / (1024 * 1024));

                // Create namespace device
                let ns = NvmeNamespace::from_controller(&mut controller, 1);
                crate::storage::register_device(Box::new(ns));
            } else {
                println!("[nvme] Failed to initialize controller");
            }
        }
    }
}

/// Allocate DMA memory
fn alloc_dma(size: usize, align: usize) -> Option<*mut u8> {
    use alloc::alloc::{alloc_zeroed, Layout};
    
    let layout = Layout::from_size_align(size, align).ok()?;
    let ptr = unsafe { alloc_zeroed(layout) };
    
    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}
