//! USB Mass Storage Driver
//!
//! Implements USB Mass Storage Class (MSC) with BOT (Bulk-Only Transport).
//! Supports USB flash drives, external hard drives, etc.

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::format;

use crate::println;
use crate::error::UsbError;
use super::{UsbDriver, UsbDevice, UsbClass};

/// Mass Storage driver
pub struct MassStorageDriver {
    name: &'static str,
    devices: Vec<UsbMassStorage>,
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

impl MassStorageDriver {
    /// Create new mass storage driver
    pub const fn new() -> Self {
        Self {
            name: "USB Mass Storage",
            devices: Vec::new(),
        }
    }

    /// Get connected storage devices
    pub fn devices(&self) -> &[UsbMassStorage] {
        &self.devices
    }

    /// Send SCSI command to device
    fn send_scsi_command(&self, device: &UsbMassStorage, command: &[u8], data: &mut [u8], data_in: bool) -> Result<(), UsbError> {
        // TODO: Implement BOT protocol
        // 1. Send CBW
        // 2. Send/receive data (if any)
        // 3. Receive CSW
        // 4. Check status
        
        println!("[usb-msc] Sending SCSI command {:02X} to device {}", 
            command[0], device.address);
        
        Err(UsbError::UnsupportedDevice)
    }

    /// Read sectors from device
    pub fn read_sectors(&self, device: &UsbMassStorage, lba: u64, count: u16, buffer: &mut [u8]) -> Result<(), UsbError> {
        if !device.ready {
            return Err(UsbError::DeviceNotResponding);
        }

        // Build READ(10) or READ(16) command
        let command = if lba < (1 << 32) {
            // Use READ(10) for 32-bit LBA
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
        } else {
            // Use READ(16) for 64-bit LBA
            return Err(UsbError::UnsupportedDevice); // TODO: Implement READ(16)
        };

        self.send_scsi_command(device, &command[..10], buffer, true)
    }

    /// Write sectors to device
    pub fn write_sectors(&self, device: &UsbMassStorage, lba: u64, count: u16, buffer: &[u8]) -> Result<(), UsbError> {
        if !device.ready {
            return Err(UsbError::DeviceNotResponding);
        }

        // Build WRITE(10) command
        let command = [
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
        ];

        self.send_scsi_command(device, &command, &mut [], false)
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
        println!("[usb-msc] Initializing mass storage at address {}", device.address);
        
        // TODO:
        // 1. Get Max LUN
        // 2. Inquiry to get device info
        // 3. Test Unit Ready
        // 4. Read Capacity
        // 5. Register with block device subsystem
        
        let msd = UsbMassStorage {
            address: device.address,
            interface: 0, // TODO: Parse from descriptors
            bulk_out: 1,
            bulk_in: 2,
            interrupt_ep: None,
            max_lun: 0,
            capacity_sectors: 0,
            sector_size: 512,
            ready: false,
        };
        
        self.devices.push(msd);
        
        println!("[usb-msc] Mass storage initialized");
        Ok(())
    }

    fn disconnect(&mut self, device: &UsbDevice) {
        println!("[usb-msc] Mass storage disconnected from address {}", device.address);
        self.devices.retain(|d| d.address != device.address);
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
