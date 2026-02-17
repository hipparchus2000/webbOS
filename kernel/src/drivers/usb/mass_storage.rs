//! USB Mass Storage Driver
//!
//! Implements USB Mass Storage Class (MSC) with BOT (Bulk-Only Transport).
//! Supports USB flash drives, external hard drives, etc.

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::format;
use alloc::sync::Arc;
use spin::Mutex;

use crate::println;
use crate::error::{UsbError, VFatError, IoError};
use crate::storage::{BlockDevice, StorageError};
use super::{UsbDriver, UsbDevice, UsbClass};

/// Mass Storage driver
pub struct MassStorageDriver {
    name: &'static str,
    devices: Vec<Arc<Mutex<UsbMassStorage>>>,
}

/// USB Mass Storage device
#[derive(Debug)]
pub struct UsbMassStorage {
    /// USB address
    pub address: u8,
    /// Interface number
    pub interface: u8,
    /// Bulk OUT endpoint
    pub bulk_out: u8,
    /// Bulk IN endpoint
    pub bulk_in: u8,
    /// Interrupt endpoint (for CB devices)
    pub interrupt_ep: Option<u8>,
    /// Max LUN (Logical Unit Number)
    pub max_lun: u8,
    /// Device capacity in sectors
    pub capacity_sectors: u64,
    /// Sector size (usually 512 bytes)
    pub sector_size: u32,
    /// Device is ready
    pub ready: bool,
    /// Device model/name
    pub model: String,
    /// Next command tag
    next_tag: u32,
    /// Max packet size for bulk out
    pub max_packet_out: u16,
    /// Max packet size for bulk in
    pub max_packet_in: u16,
}

/// CBW (Command Block Wrapper) - 31 bytes
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct CommandBlockWrapper {
    /// Signature 'USBC' (0x43425355)
    pub signature: u32,
    /// Tag (unique per command)
    pub tag: u32,
    /// Data transfer length
    pub data_transfer_length: u32,
    /// Flags (0x00 = OUT, 0x80 = IN)
    pub flags: u8,
    /// Logical Unit Number
    pub lun: u8,
    /// Command length (1-16)
    pub command_length: u8,
    /// SCSI command (up to 16 bytes)
    pub command: [u8; 16],
}

impl CommandBlockWrapper {
    /// Create new CBW
    pub fn new(tag: u32, data_len: u32, flags: u8, lun: u8, command: &[u8]) -> Self {
        let mut cbw = Self {
            signature: 0x43425355, // 'USBC'
            tag,
            data_transfer_length: data_len,
            flags,
            lun,
            command_length: command.len() as u8,
            command: [0; 16],
        };
        cbw.command[..command.len()].copy_from_slice(command);
        cbw
    }
}

/// CSW (Command Status Wrapper) - 13 bytes
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy)]
pub struct CommandStatusWrapper {
    /// Signature 'USBS' (0x53425355)
    pub signature: u32,
    /// Tag (matches CBW)
    pub tag: u32,
    /// Data residue
    pub data_residue: u32,
    /// Status (0 = Good, 1 = Check Condition, 2 = Phase Error)
    pub status: u8,
}

/// SCSI commands for USB mass storage
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScsiCommand {
    /// Test Unit Ready
    TestUnitReady = 0x00,
    /// Request Sense
    RequestSense = 0x03,
    /// Inquiry
    Inquiry = 0x12,
    /// Mode Sense (6)
    ModeSense6 = 0x1A,
    /// Start Stop Unit
    StartStopUnit = 0x1B,
    /// Prevent Allow Medium Removal
    PreventAllowRemoval = 0x1E,
    /// Read Format Capacities
    ReadFormatCapacities = 0x23,
    /// Read Capacity (10)
    ReadCapacity10 = 0x25,
    /// Read (10)
    Read10 = 0x28,
    /// Write (10)
    Write10 = 0x2A,
    /// Verify (10)
    Verify10 = 0x2F,
    /// Read Capacity (16) - for large drives
    ReadCapacity16 = 0x9E,
    /// Read (16)
    Read16 = 0x88,
    /// Write (16)
    Write16 = 0x8A,
}

/// SCSI sense data
#[derive(Debug, Clone)]
pub struct ScsiSenseData {
    /// Response code
    pub response_code: u8,
    /// Sense key
    pub sense_key: u8,
    /// Additional sense code
    pub asc: u8,
    /// Additional sense code qualifier
    pub ascq: u8,
}

impl UsbMassStorage {
    /// Get the next command tag
    fn next_tag(&mut self) -> u32 {
        let tag = self.next_tag;
        self.next_tag = self.next_tag.wrapping_add(1);
        tag
    }

    /// Build SCSI READ(10) CDB
    fn build_read10_cdb(lba: u32, count: u16) -> [u8; 10] {
        [
            ScsiCommand::Read10 as u8,
            0, // RDPROTECT, DPO, FUA, FUA_NV, Obsolete
            (lba >> 24) as u8,
            (lba >> 16) as u8,
            (lba >> 8) as u8,
            lba as u8,
            0, // Group number
            (count >> 8) as u8,
            count as u8,
            0, // Control
        ]
    }

    /// Build SCSI WRITE(10) CDB
    fn build_write10_cdb(lba: u32, count: u16) -> [u8; 10] {
        [
            ScsiCommand::Write10 as u8,
            0, // WRPROTECT, DPO, FUA, FUA_NV, Obsolete
            (lba >> 24) as u8,
            (lba >> 16) as u8,
            (lba >> 8) as u8,
            lba as u8,
            0, // Group number
            (count >> 8) as u8,
            count as u8,
            0, // Control
        ]
    }

    /// Build SCSI READ CAPACITY(10) CDB
    fn build_read_capacity10_cdb() -> [u8; 10] {
        [
            ScsiCommand::ReadCapacity10 as u8,
            0, // Reserved
            0, 0, 0, 0, // Logical block address (0 = last LBA for current capacity)
            0, // Reserved
            0, // Reserved
            0, // PMI (Partial Medium Indicator)
            0, // Control
        ]
    }

    /// Build SCSI TEST UNIT READY CDB
    fn build_test_unit_ready_cdb() -> [u8; 6] {
        [
            ScsiCommand::TestUnitReady as u8,
            0, // LUN and reserved
            0, 0, // Reserved
            0, // Reserved
            0, // Control
        ]
    }

    /// Build SCSI INQUIRY CDB
    fn build_inquiry_cdb(alloc_len: u8) -> [u8; 6] {
        [
            ScsiCommand::Inquiry as u8,
            0, // LUN and CMDDT/EVPD
            0, // Page code
            0, // Reserved
            alloc_len, // Allocation length
            0, // Control
        ]
    }

    /// Build SCSI REQUEST SENSE CDB
    fn build_request_sense_cdb(alloc_len: u8) -> [u8; 6] {
        [
            ScsiCommand::RequestSense as u8,
            0, // LUN and reserved
            0, // Reserved
            0, // Reserved
            alloc_len, // Allocation length
            0, // Control
        ]
    }

    /// Execute a SCSI command using BOT (Bulk-Only Transport)
    /// 
    /// # Arguments
    /// * `command` - The SCSI CDB bytes
    /// * `data_buffer` - Buffer for data phase (read or write)
    /// * `data_in` - true for IN transfer (device to host), false for OUT
    fn execute_scsi_command(
        &mut self,
        command: &[u8],
        data_buffer: &mut [u8],
        data_in: bool,
    ) -> Result<(), UsbError> {
        self.execute_scsi_command_with_data(command, data_buffer, data_in, &[])
    }

    /// Execute a SCSI command with separate write data
    /// 
    /// This variant allows passing write data separately from the response buffer
    fn execute_scsi_command_with_data(
        &mut self,
        command: &[u8],
        response_buffer: &mut [u8],
        data_in: bool,
        write_data: &[u8],
    ) -> Result<(), UsbError> {
        let tag = self.next_tag();
        let data_len = if data_in { 
            response_buffer.len() 
        } else { 
            write_data.len() 
        } as u32;
        let flags = if data_in { 0x80 } else { 0x00 };

        // Step 1: Build and send CBW
        let cbw = CommandBlockWrapper::new(tag, data_len, flags, 0, command);
        
        // Send CBW to bulk OUT endpoint
        self.send_bulk_out(&cbw)?;

        // Step 2: Data phase (if any)
        if data_len > 0 {
            if data_in {
                // Read data from bulk IN endpoint
                self.receive_bulk_in(response_buffer)?;
            } else {
                // Write data to bulk OUT endpoint
                self.send_bulk_out_bytes(write_data)?;
            }
        }

        // Step 3: Receive CSW
        let csw = self.receive_csw()?;

        // Step 4: Check status
        if csw.signature != 0x53425355 {
            println!("[usb-msc] Invalid CSW signature: 0x{:08X}", csw.signature);
            return Err(UsbError::TransferError("Invalid CSW signature".into()));
        }

        if csw.tag != tag {
            println!("[usb-msc] CSW tag mismatch: expected {}, got {}", tag, csw.tag);
            return Err(UsbError::TransferError("CSW tag mismatch".into()));
        }

        match csw.status {
            0x00 => Ok(()), // Good
            0x01 => {
                // Check Condition - request sense for more info
                println!("[usb-msc] Check Condition status, requesting sense");
                Err(UsbError::TransferError("SCSI Check Condition".into()))
            }
            0x02 => {
                println!("[usb-msc] Phase Error");
                Err(UsbError::TransferError("SCSI Phase Error".into()))
            }
            _ => {
                println!("[usb-msc] Unknown status: {}", csw.status);
                Err(UsbError::TransferError("Unknown SCSI status".into()))
            }
        }
    }

    /// Send data to bulk OUT endpoint
    fn send_bulk_out<T>(&self, data: &T) -> Result<(), UsbError> {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                data as *const T as *const u8,
                core::mem::size_of::<T>(),
            )
        };
        self.send_bulk_out_bytes(bytes)
    }

    /// Send bytes to bulk OUT endpoint
    fn send_bulk_out_bytes(&self, data: &[u8]) -> Result<(), UsbError> {
        // TODO: Integrate with actual xHCI bulk transfer
        // For now, this is a placeholder that would interface with the USB controller
        // to perform the actual bulk OUT transfer
        
        // This would typically:
        // 1. Allocate a TRB (Transfer Request Block)
        // 2. Set up the bulk OUT endpoint
        // 3. Ring the doorbell
        // 4. Wait for completion
        
        println!("[usb-msc] Bulk OUT {} bytes to ep {}", data.len(), self.bulk_out);
        
        // Placeholder - actual implementation depends on xHCI integration
        // For now, assume success in simulation
        Ok(())
    }

    /// Receive data from bulk IN endpoint
    fn receive_bulk_in(&self, buffer: &mut [u8]) -> Result<(), UsbError> {
        // TODO: Integrate with actual xHCI bulk transfer
        println!("[usb-msc] Bulk IN {} bytes from ep {}", buffer.len(), self.bulk_in);
        
        // Placeholder - actual implementation depends on xHCI integration
        // For now, fill with zeros to indicate not actually read
        buffer.fill(0);
        Ok(())
    }

    /// Receive CSW from bulk IN endpoint
    fn receive_csw(&self) -> Result<CommandStatusWrapper, UsbError> {
        let mut csw = CommandStatusWrapper {
            signature: 0,
            tag: 0,
            data_residue: 0,
            status: 0,
        };
        
        let bytes = unsafe {
            core::slice::from_raw_parts_mut(
                &mut csw as *mut CommandStatusWrapper as *mut u8,
                core::mem::size_of::<CommandStatusWrapper>(),
            )
        };
        
        self.receive_bulk_in(bytes)?;
        Ok(csw)
    }

    /// Read capacity from device using READ CAPACITY(10)
    pub fn read_capacity(&mut self) -> Result<(u64, u32), UsbError> {
        let cdb = Self::build_read_capacity10_cdb();
        let mut response = [0u8; 8]; // Read Capacity response is 8 bytes

        self.execute_scsi_command(&cdb, &mut response, true)?;

        // Parse response:
        // Bytes 0-3: Last LBA (big-endian)
        // Bytes 4-7: Block length in bytes (big-endian)
        let last_lba = u32::from_be_bytes([response[0], response[1], response[2], response[3]]) as u64;
        let block_len = u32::from_be_bytes([response[4], response[5], response[6], response[7]]);

        let total_sectors = last_lba + 1; // Convert last LBA to total count
        
        println!("[usb-msc] Capacity: {} sectors, {} bytes/sector", total_sectors, block_len);
        
        Ok((total_sectors, block_len))
    }

    /// Perform INQUIRY to get device information
    pub fn inquiry(&mut self) -> Result<String, UsbError> {
        let cdb = Self::build_inquiry_cdb(36); // Standard inquiry data length
        let mut response = [0u8; 36];

        self.execute_scsi_command(&cdb, &mut response, true)?;

        // Parse inquiry data
        // Bytes 8-15: Vendor ID (8 bytes)
        // Bytes 16-31: Product ID (16 bytes)
        // Bytes 32-35: Product Revision (4 bytes)
        let vendor = core::str::from_utf8(&response[8..16])
            .unwrap_or("Unknown")
            .trim();
        let product = core::str::from_utf8(&response[16..32])
            .unwrap_or("Unknown")
            .trim();
        let revision = core::str::from_utf8(&response[32..36])
            .unwrap_or("0000")
            .trim();

        let model = format!("{} {} {}", vendor, product, revision);
        Ok(model)
    }

    /// Test if unit is ready
    pub fn test_unit_ready(&mut self) -> Result<(), UsbError> {
        let cdb = Self::build_test_unit_ready_cdb();
        self.execute_scsi_command(&cdb, &mut [], false)
    }

    /// Read sectors from the device
    pub fn read_sectors(&mut self, lba: u64, count: u16, buffer: &mut [u8]) -> Result<(), UsbError> {
        if !self.ready {
            return Err(UsbError::DeviceNotResponding);
        }

        // Validate buffer size
        let expected_len = count as usize * self.sector_size as usize;
        if buffer.len() < expected_len {
            return Err(UsbError::InvalidParameter(
                format!("Buffer too small: {} < {}", buffer.len(), expected_len)
            ));
        }

        // Check if LBA fits in 32 bits (READ(10) limitation)
        if lba >= (1u64 << 32) {
            // Would need READ(16) for large drives
            return Err(UsbError::UnsupportedDevice);
        }

        let cdb = Self::build_read10_cdb(lba as u32, count);
        self.execute_scsi_command(&cdb, &mut buffer[..expected_len], true)
    }

    /// Write sectors to the device
    pub fn write_sectors(&mut self, lba: u64, count: u16, buffer: &[u8]) -> Result<(), UsbError> {
        if !self.ready {
            return Err(UsbError::DeviceNotResponding);
        }

        // Validate buffer size
        let expected_len = count as usize * self.sector_size as usize;
        if buffer.len() < expected_len {
            return Err(UsbError::InvalidParameter(
                format!("Buffer too small: {} < {}", buffer.len(), expected_len)
            ));
        }

        // Check if LBA fits in 32 bits (WRITE(10) limitation)
        if lba >= (1u64 << 32) {
            // Would need WRITE(16) for large drives
            return Err(UsbError::UnsupportedDevice);
        }

        let cdb = Self::build_write10_cdb(lba as u32, count);
        let data_slice = &buffer[..expected_len];
        
        // Use the write_data variant to avoid needing a mutable buffer
        self.execute_scsi_command_with_data(&cdb, &mut [], false, data_slice)
    }
}

/// Block device wrapper for USB mass storage
/// 
/// This wraps UsbMassStorage to implement the BlockDevice trait
pub struct UsbMassStorageBlockDevice {
    inner: Arc<Mutex<UsbMassStorage>>,
    device_index: usize,
}

impl UsbMassStorageBlockDevice {
    /// Create a new block device wrapper
    pub fn new(device: Arc<Mutex<UsbMassStorage>>, index: usize) -> Self {
        Self {
            inner: device,
            device_index: index,
        }
    }
}

// SAFETY: UsbMassStorageBlockDevice is thread-safe via Arc<Mutex>
unsafe impl Send for UsbMassStorageBlockDevice {}
unsafe impl Sync for UsbMassStorageBlockDevice {}

impl BlockDevice for UsbMassStorageBlockDevice {
    fn name(&self) -> &str {
        // Return a static name based on device index
        // In a real implementation, this might return the model string
        "usb-storage"
    }

    fn block_size(&self) -> usize {
        let device = self.inner.lock();
        device.sector_size as usize
    }

    fn block_count(&self) -> u64 {
        let device = self.inner.lock();
        device.capacity_sectors
    }

    fn read_blocks(&self, start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError> {
        let mut device = self.inner.lock();
        
        if count == 0 {
            return Ok(());
        }

        if count > 65535 {
            // SCSI READ(10) uses 16-bit transfer length
            // Split into multiple reads
            let mut offset = 0;
            let mut remaining = count;
            let mut current_lba = start;

            while remaining > 0 {
                let to_read = remaining.min(65535);
                let expected_len = to_read * device.sector_size as usize;
                self.read_blocks(current_lba, to_read, &mut buf[offset..offset + expected_len])?;
                offset += expected_len;
                remaining -= to_read;
                current_lba += to_read as u64;
            }
            return Ok(());
        }

        device
            .read_sectors(start, count as u16, buf)
            .map_err(|e| match e {
                UsbError::DeviceNotResponding => StorageError::NoMedia,
                UsbError::Timeout => StorageError::Timeout,
                UsbError::TransferError(_) => StorageError::IoError,
                _ => StorageError::Unknown,
            })
    }

    fn write_blocks(&self, start: u64, count: usize, buf: &[u8]) -> Result<(), StorageError> {
        let mut device = self.inner.lock();
        
        if count == 0 {
            return Ok(());
        }

        if count > 65535 {
            // Split into multiple writes
            let mut offset = 0;
            let mut remaining = count;
            let mut current_lba = start;

            while remaining > 0 {
                let to_write = remaining.min(65535);
                let expected_len = to_write * device.sector_size as usize;
                self.write_blocks(current_lba, to_write, &buf[offset..offset + expected_len])?;
                offset += expected_len;
                remaining -= to_write;
                current_lba += to_write as u64;
            }
            return Ok(());
        }

        device
            .write_sectors(start, count as u16, buf)
            .map_err(|e| match e {
                UsbError::DeviceNotResponding => StorageError::NoMedia,
                UsbError::Timeout => StorageError::Timeout,
                UsbError::TransferError(_) => StorageError::IoError,
                _ => StorageError::Unknown,
            })
    }

    fn flush(&self) -> Result<(), StorageError> {
        // USB mass storage typically doesn't have a flush command
        // The SCSI SYNCHRONIZE CACHE command could be implemented here
        Ok(())
    }
}

/// VFS BlockDevice trait implementation for USB mass storage
/// 
/// This is separate from the storage subsystem BlockDevice trait
pub struct UsbMassStorageVfsBlockDevice {
    inner: Arc<Mutex<UsbMassStorage>>,
}

impl UsbMassStorageVfsBlockDevice {
    /// Create a new VFS block device wrapper
    pub fn new(device: Arc<Mutex<UsbMassStorage>>) -> Self {
        Self { inner: device }
    }
}

// SAFETY: Thread-safe via Arc<Mutex>
unsafe impl Send for UsbMassStorageVfsBlockDevice {}
unsafe impl Sync for UsbMassStorageVfsBlockDevice {}

impl crate::fs::block::BlockDevice for UsbMassStorageVfsBlockDevice {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), VFatError> {
        let mut device = self.inner.lock();
        let sector_size = device.sector_size as usize;
        
        if buffer.len() != sector_size {
            return Err(VFatError::InvalidParameter(format!(
                "Buffer size {} != sector size {}",
                buffer.len(),
                sector_size
            )));
        }

        device
            .read_sectors(block, 1, buffer)
            .map_err(|e| VFatError::Io(IoError::Other(format!("USB read error: {:?}", e))))
    }

    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), VFatError> {
        let mut device = self.inner.lock();
        let sector_size = device.sector_size as usize;
        
        if buffer.len() != sector_size {
            return Err(VFatError::InvalidParameter(format!(
                "Buffer size {} != sector size {}",
                buffer.len(),
                sector_size
            )));
        }

        device
            .write_sectors(block, 1, buffer)
            .map_err(|e| VFatError::Io(IoError::Other(format!("USB write error: {:?}", e))))
    }

    fn read_blocks(&mut self, start_block: u64, count: usize, buffer: &mut [u8]) -> Result<(), VFatError> {
        let mut device = self.inner.lock();
        let sector_size = device.sector_size as usize;
        
        if buffer.len() != count * sector_size {
            return Err(VFatError::InvalidParameter(
                String::from("Buffer size mismatch")
            ));
        }

        // Handle large reads by chunking
        let mut remaining = count;
        let mut current_lba = start_block;
        let mut offset = 0;

        while remaining > 0 {
            let to_read = remaining.min(65535); // Max SCSI transfer
            let chunk_size = to_read * sector_size;
            
            device
                .read_sectors(current_lba, to_read as u16, &mut buffer[offset..offset + chunk_size])
                .map_err(|e| VFatError::Io(IoError::Other(format!("USB read error: {:?}", e))))?;
            
            remaining -= to_read;
            current_lba += to_read as u64;
            offset += chunk_size;
        }

        Ok(())
    }

    fn write_blocks(&mut self, start_block: u64, count: usize, buffer: &[u8]) -> Result<(), VFatError> {
        let mut device = self.inner.lock();
        let sector_size = device.sector_size as usize;
        
        if buffer.len() != count * sector_size {
            return Err(VFatError::InvalidParameter(
                String::from("Buffer size mismatch")
            ));
        }

        // Handle large writes by chunking
        let mut remaining = count;
        let mut current_lba = start_block;
        let mut offset = 0;

        while remaining > 0 {
            let to_write = remaining.min(65535); // Max SCSI transfer
            let chunk_size = to_write * sector_size;
            
            device
                .write_sectors(current_lba, to_write as u16, &buffer[offset..offset + chunk_size])
                .map_err(|e| VFatError::Io(IoError::Other(format!("USB write error: {:?}", e))))?;
            
            remaining -= to_write;
            current_lba += to_write as u64;
            offset += chunk_size;
        }

        Ok(())
    }

    fn capacity(&self) -> u64 {
        let device = self.inner.lock();
        device.capacity_sectors
    }

    fn block_size(&self) -> usize {
        let device = self.inner.lock();
        device.sector_size as usize
    }

    fn flush(&mut self) -> Result<(), VFatError> {
        Ok(())
    }
}

impl MassStorageDriver {
    /// Create new mass storage driver
    pub const fn new() -> Self {
        Self {
            name: "USB Mass Storage",
            devices: Vec::new(),
        }
    }

    /// Get connected storage devices
    pub fn devices(&self) -> &[Arc<Mutex<UsbMassStorage>>] {
        &self.devices
    }

    /// Register a USB mass storage device as a block device
    /// 
    /// This function delegates to the usb_storage module which handles:
    /// - Registration with the storage subsystem (for raw block access)
    /// - Auto-mounting via VFS (for filesystem access)
    pub fn register_block_device(&self, device: &Arc<Mutex<UsbMassStorage>>) {
        let dev = device.lock();
        
        println!("[usb-msc] Registering block device for USB storage ({} sectors, {} bytes/sector)",
            dev.capacity_sectors,
            dev.sector_size
        );
        drop(dev);

        // Delegate to usb_storage module for registration and auto-mount
        crate::storage::usb_storage::register_device(device.clone()).ok();
    }

    /// Initialize USB mass storage device
    fn initialize_device(&self, device: &mut UsbMassStorage) -> Result<(), UsbError> {
        // Step 1: Test Unit Ready (may need retries)
        for _ in 0..5 {
            match device.test_unit_ready() {
                Ok(()) => break,
                Err(_) => {
                    // Device might need time to spin up
                    // In real implementation, add small delay here
                }
            }
        }

        // Step 2: Get device information via INQUIRY
        match device.inquiry() {
            Ok(model) => {
                device.model = model;
                println!("[usb-msc] Device model: {}", device.model);
            }
            Err(e) => {
                println!("[usb-msc] INQUIRY failed: {:?}", e);
                device.model = String::from("Unknown USB Device");
            }
        }

        // Step 3: Read Capacity
        match device.read_capacity() {
            Ok((sectors, sector_size)) => {
                device.capacity_sectors = sectors;
                device.sector_size = sector_size;
                device.ready = true;
                println!("[usb-msc] Device ready: {} sectors x {} bytes", sectors, sector_size);
                Ok(())
            }
            Err(e) => {
                println!("[usb-msc] Read Capacity failed: {:?}", e);
                device.ready = false;
                Err(e)
            }
        }
    }

    /// Send SCSI command to device (legacy method)
    fn send_scsi_command(
        &self,
        device: &UsbMassStorage,
        command: &[u8],
        data: &mut [u8],
        data_in: bool,
    ) -> Result<(), UsbError> {
        println!(
            "[usb-msc] Sending SCSI command {:02X} to device {}",
            command[0], device.address
        );

        // This is a placeholder - actual implementation would use the device's
        // execute_scsi_command method through a mutable reference
        Err(UsbError::UnsupportedDevice)
    }

    /// Read sectors from device (legacy method, use device.read_sectors instead)
    pub fn read_sectors(
        &self,
        device: &UsbMassStorage,
        lba: u64,
        count: u16,
        buffer: &mut [u8],
    ) -> Result<(), UsbError> {
        if !device.ready {
            return Err(UsbError::DeviceNotResponding);
        }

        // This is a placeholder - actual implementation would require mutable access
        Err(UsbError::UnsupportedDevice)
    }

    /// Write sectors to device (legacy method, use device.write_sectors instead)
    pub fn write_sectors(
        &self,
        device: &UsbMassStorage,
        lba: u64,
        count: u16,
        buffer: &[u8],
    ) -> Result<(), UsbError> {
        if !device.ready {
            return Err(UsbError::DeviceNotResponding);
        }

        // This is a placeholder - actual implementation would require mutable access
        Err(UsbError::UnsupportedDevice)
    }
}

impl UsbDriver for MassStorageDriver {
    fn name(&self) -> &str {
        self.name
    }

    fn supports(&self, device: &UsbDevice) -> bool {
        // Mass Storage class is 0x08
        device.device_descriptor.class == UsbClass::MassStorage as u8
    }

    fn init(&mut self, device: &mut UsbDevice) -> Result<(), UsbError> {
        println!(
            "[usb-msc] Initializing mass storage at address {}",
            device.address
        );

        // Parse interface and endpoint descriptors from configuration
        // This is a simplified version - real implementation would parse descriptors
        let interface = 0;
        let bulk_out = 1;
        let bulk_in = 2;

        let mut msd = UsbMassStorage {
            address: device.address,
            interface,
            bulk_out,
            bulk_in,
            interrupt_ep: None,
            max_lun: 0,
            capacity_sectors: 0,
            sector_size: 512,
            ready: false,
            model: String::new(),
            next_tag: 1,
            max_packet_out: 512,
            max_packet_in: 512,
        };

        // Initialize the device (TEST UNIT READY, INQUIRY, READ CAPACITY)
        self.initialize_device(&mut msd)?;

        // Wrap in Arc<Mutex> for shared access
        let msd_arc = Arc::new(Mutex::new(msd));
        
        // Register with block device subsystem if ready
        if msd_arc.lock().ready {
            self.register_block_device(&msd_arc);
        }

        // Store in devices list
        self.devices.push(msd_arc);

        println!("[usb-msc] Mass storage initialized successfully");
        Ok(())
    }

    fn disconnect(&mut self, device: &UsbDevice) {
        println!(
            "[usb-msc] Mass storage disconnected from address {}",
            device.address
        );

        // Find and remove the device
        let idx = self.devices.iter().position(|d| d.lock().address == device.address);
        
        if let Some(index) = idx {
            // Unmount if mounted
            crate::storage::usb_storage::disconnect(index);
            
            self.devices.remove(index);
            println!("[usb-msc] Device {} removed from storage subsystem", device.address);
        }
    }
}

/// Mass Storage subclass codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MassStorageSubclass {
    /// RBC (Reduced Block Commands)
    Rbc = 0x01,
    /// ATAPI (CD/DVD)
    Atapi = 0x02,
    /// QIC-157 (tape)
    Qic157 = 0x03,
    /// UFI (floppy)
    Ufi = 0x04,
    /// SFF-8070i (obsolete)
    Sff8070i = 0x05,
    /// SCSI transparent command set
    Scsi = 0x06,
}

/// Mass Storage protocol codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MassStorageProtocol {
    /// CBI (Control/Bulk/Interrupt) with command completion interrupt
    CbiWithInt = 0x00,
    /// CBI without command completion interrupt
    CbiWithoutInt = 0x01,
    /// BOT (Bulk-Only Transport)
    Bot = 0x50,
    /// UAS (USB Attached SCSI)
    Uas = 0x62,
}
