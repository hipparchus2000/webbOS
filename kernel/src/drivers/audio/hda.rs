//! Intel HD Audio (HDA) Controller Driver
//!
//! Implements basic playback support for Intel High Definition Audio controllers.
//! 
//! # References
//! - Intel High Definition Audio Specification 1.0a
//! - Intel ICH6-ICH10 Datasheets
//!
//! # Architecture Overview
//! - PCI Multimedia device (Class 0x04, Subclass 0x03)
//! - Memory-mapped I/O registers
//! - DMA-based audio streaming
//! - Codec communication via CORB/RIRB

use super::{AudioDevice, AudioError, AudioFormat};
use crate::drivers::pci::{self, PciDevice};
use crate::mm::{phys_to_virt, virt_to_phys_u64};
use crate::println;
use webbos_shared::types::PhysAddr;
use alloc::alloc::{alloc_zeroed, Layout};

use core::ptr::{read_volatile, write_volatile};


// ============================================================================
// PCI Configuration
// ============================================================================

/// HDA PCI class code (Multimedia)
pub const HDA_PCI_CLASS: u8 = 0x04;
/// HDA PCI subclass (HDA Controller)
pub const HDA_PCI_SUBCLASS: u8 = 0x03;

/// Intel vendor ID
pub const VENDOR_INTEL: u16 = 0x8086;
/// AMD vendor ID  
pub const VENDOR_AMD: u16 = 0x1022;
/// NVIDIA vendor ID
pub const VENDOR_NVIDIA: u16 = 0x10DE;
/// Realtek vendor ID
pub const VENDOR_REALTEK: u16 = 0x10EC;

// ============================================================================
// HDA Register Offsets
// ============================================================================

/// Global Capabilities
const REG_GCAP: usize = 0x00;
/// Minor Version
const REG_VMIN: usize = 0x02;
/// Major Version
const REG_VMAJ: usize = 0x03;
/// Global Control
const REG_GCTL: usize = 0x08;
/// Wake Enable
const REG_WAKEEN: usize = 0x0C;
/// State Change Status
const REG_STATESTS: usize = 0x0E;
/// Global Status
const REG_GSTS: usize = 0x10;
/// CORB Lower Base Address
const REG_CORBLBASE: usize = 0x40;
/// CORB Upper Base Address
const REG_CORBUBASE: usize = 0x44;
/// CORB Write Pointer
const REG_CORBWP: usize = 0x48;
/// CORB Read Pointer
const REG_CORBRP: usize = 0x4A;
/// CORB Control
const REG_CORBCTL: usize = 0x4C;
/// CORB Status
const REG_CORBSTS: usize = 0x4D;
/// CORB Size
const REG_CORBSIZE: usize = 0x4E;
/// RIRB Lower Base Address
const REG_RIRBLBASE: usize = 0x50;
/// RIRB Upper Base Address
const REG_RIRBUBASE: usize = 0x54;
/// RIRB Write Pointer
const REG_RIRBWP: usize = 0x58;
/// Response Interrupt Count
const REG_RINTCNT: usize = 0x5A;
/// RIRB Control
const REG_RIRBCTL: usize = 0x5C;
/// RIRB Status
const REG_RIRBSTS: usize = 0x5D;
/// RIRB Size
const REG_RIRBSIZE: usize = 0x5E;
/// Immediate Command Output Interface
const REG_ICOI: usize = 0x60;
/// Immediate Command Input Interface  
const REG_ICII: usize = 0x64;
/// Immediate Command Status
const REG_ICIS: usize = 0x68;
/// DMA Position Lower Base Address
const REG_DPLBASE: usize = 0x70;
/// DMA Position Upper Base Address
const REG_DPUBASE: usize = 0x74;

// Stream Descriptor 0 (Playback) Registers
const STREAM_BASE: usize = 0x80;
const STREAM_SIZE: usize = 0x20;

/// Stream Control
const REG_SD_CTL: usize = 0x00;
/// Stream Status
const REG_SD_STS: usize = 0x03;
/// Stream Link Position in Current Buffer
const REG_SD_LPIB: usize = 0x04;
/// Stream Cyclic Buffer Length
const REG_SD_CBL: usize = 0x08;
/// Stream Last Valid Index
const REG_SD_LVI: usize = 0x0C;
/// Stream FIFO Size
const REG_SD_FIFOSIZE: usize = 0x10;
/// Stream Format
const REG_SD_FMT: usize = 0x12;
/// Stream Buffer Descriptor List Lower Base Address
const REG_SD_BDLPL: usize = 0x18;
/// Stream Buffer Descriptor List Upper Base Address
const REG_SD_BDLPU: usize = 0x1C;

// ============================================================================
// Register Bit Definitions
// ============================================================================

// GCAP bits
const GCAP_64OK: u16 = 1 << 0;
const GCAP_NSDO_MASK: u16 = 0x0003;
const GCAP_BSS_MASK: u16 = 0x00F0;
const GCAP_ISS_MASK: u16 = 0x0F00;
const GCAP_OSS_MASK: u16 = 0xF000;

// GCTL bits
const GCTL_CRST: u32 = 1 << 0;
const GCTL_FCNTRL: u32 = 1 << 1;
const GCTL_UNSOL: u32 = 1 << 8;

// CORB/RIRB control bits
const CORBCTL_RUN: u8 = 1 << 1;
const CORBSTS_CMEI: u8 = 1 << 0;
const RIRBCTL_RUN: u8 = 1 << 1;
const RIRBCTL_RINTCTL: u8 = 1 << 0;
const RIRBSTS_RINTFL: u8 = 1 << 0;
const RIRBSTS_RIRBOIS: u8 = 1 << 2;

// Stream control bits
const SD_CTL_SRST: u32 = 1 << 0;
const SD_CTL_RUN: u32 = 1 << 1;
const SD_CTL_IOCE: u32 = 1 << 2;
const SD_CTL_FEIE: u32 = 1 << 3;
const SD_CTL_DEIE: u32 = 1 << 4;

// State status bits
const STATESTS_SDIWake: u16 = 0x7FFF;

// ICIS bits
const ICIS_ICB: u16 = 1 << 0;
const ICIS_IRV: u16 = 1 << 1;

// ============================================================================
// Buffer Sizes
// ============================================================================

/// CORB buffer size (256 entries * 4 bytes)
const CORB_SIZE: usize = 256;
/// RIRB buffer size (256 entries * 8 bytes)
const RIRB_SIZE: usize = 256;
/// BDL size (32 entries * 16 bytes)
const BDL_SIZE: usize = 32;
/// DMA position buffer size (8 bytes per stream)
const DMA_POS_SIZE: usize = 256;
/// Default DMA buffer size (1 second of 48kHz stereo 16-bit)
const DEFAULT_BUFFER_SIZE: usize = 48000 * 4;

// ============================================================================
// HDA Structures
// ============================================================================

/// Buffer Descriptor List Entry
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct BdlEntry {
    /// Lower 32 bits of buffer address
    addr_low: u32,
    /// Upper 32 bits of buffer address
    addr_high: u32,
    /// Buffer length in bytes
    length: u32,
    /// Flags (IOC for interrupt on completion)
    flags: u32,
}

impl BdlEntry {
    const fn new() -> Self {
        Self {
            addr_low: 0,
            addr_high: 0,
            length: 0,
            flags: 0,
        }
    }
    
    fn set_address(&mut self, addr: u64) {
        self.addr_low = (addr & 0xFFFFFFFF) as u32;
        self.addr_high = ((addr >> 32) & 0xFFFFFFFF) as u32;
    }
    
    fn set_buffer(&mut self, addr: u64, len: u32, ioc: bool) {
        self.set_address(addr);
        self.length = len;
        self.flags = if ioc { 1 } else { 0 };
    }
}

/// Stream format bits
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamFormat {
    /// 48kHz, 16-bit, stereo
    S16LE48K = 0x0011,
    /// 44.1kHz, 16-bit, stereo
    S16LE44K = 0x4011,
    /// 96kHz, 16-bit, stereo  
    S16LE96K = 0x0051,
    /// 48kHz, 24-bit, stereo
    S24LE48K = 0x0013,
}

impl StreamFormat {
    fn from_audio_format(format: &AudioFormat) -> Option<u16> {
        if format.channels != 2 {
            return None; // Only stereo supported for now
        }
        
        match (format.sample_rate, format.bits_per_sample) {
            (44100, 16) => Some(0x4011),
            (48000, 16) => Some(0x0011),
            (96000, 16) => Some(0x0051),
            (48000, 24) => Some(0x0013),
            _ => None,
        }
    }
}

/// Codec verb/command
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum CodecVerb {
    /// Get parameter
    GetParam = 0xF0000,
    /// Set converter format
    SetStreamFormat = 0x20000,
    /// Set amplifier gain/mute
    SetAmpGainMute = 0x30000,
    /// Set pin widget control
    SetPinCtrl = 0x70700,
    /// Set stream channel
    SetStreamChan = 0x70600,
}

/// Widget parameters
const PARAM_VENDOR_ID: u8 = 0x00;
const PARAM_SUBORDINATE_NODE_COUNT: u8 = 0x04;
const PARAM_FUNCTION_GROUP_TYPE: u8 = 0x05;
const PARAM_AUDIO_WIDGET_CAP: u8 = 0x09;
const PARAM_PCM_SIZE_RATE: u8 = 0x0A;
const PARAM_PIN_CAP: u8 = 0x0C;
const PARAM_AMP_OUT_CAP: u8 = 0x12;

/// Pin widget control bits
const PIN_CTRL_OUT_ENABLE: u8 = 0x40;
const PIN_CTRL_HP_ENABLE: u8 = 0x80;

/// Function group types
const FN_GROUP_TYPE_AUDIO: u8 = 0x01;

// ============================================================================
// HDA Controller
// ============================================================================

/// Intel HD Audio Controller
pub struct HdaController {
    /// PCI device information
    pci_dev: PciDevice,
    /// MMIO base address
    mmio_base: usize,
    /// Virtual address for MMIO access
    mmio_virt: usize,
    /// CORB buffer
    corb: *mut u32,
    /// RIRB buffer
    rirb: *mut u64,
    /// BDL for stream 0
    bdl: *mut BdlEntry,
    /// DMA buffer for audio data
    dma_buffer: *mut u8,
    /// DMA buffer size
    dma_buffer_size: usize,
    /// DMA position buffer
    dma_pos: *mut u64,
    /// Current volume (0-100)
    volume: u8,
    /// Is currently playing
    playing: bool,
    /// Current format
    current_format: Option<AudioFormat>,
    /// Output node ID
    output_node: u8,
    /// Codec address
    codec_addr: u8,
}

// SAFETY: HdaController is Send+Sync because all pointers are to DMA buffers
// that are exclusively owned by this controller
unsafe impl Send for HdaController {}
unsafe impl Sync for HdaController {}

impl HdaController {
    /// Initialize the HDA controller
    pub fn init() -> Result<Self, AudioError> {
        println!("[hda] Looking for HD Audio controller...");
        
        // Find HDA controller via PCI
        let pci_dev = Self::find_controller()?;
        
        // Get MMIO base address from BAR0
        let mmio_base = Self::get_mmio_base(&pci_dev)?;
        let mmio_virt = phys_to_virt(PhysAddr::new(mmio_base)).as_u64() as usize;
        
        println!("[hda] Found controller at {:04X}:{:04X}", 
            pci_dev.vendor_id, pci_dev.device_id);
        println!("[hda] MMIO base: {:#010X} -> {:#010X}", mmio_base, mmio_virt);
        
        // Allocate DMA buffers
        let corb = Self::alloc_dma_buffer::<u32>(CORB_SIZE)?;
        let rirb = Self::alloc_dma_buffer::<u64>(RIRB_SIZE)?;
        let bdl = Self::alloc_dma_buffer::<BdlEntry>(BDL_SIZE)?;
        let dma_buffer = Self::alloc_dma_buffer::<u8>(DEFAULT_BUFFER_SIZE)?;
        let dma_pos = Self::alloc_dma_buffer::<u64>(DMA_POS_SIZE / 8)?;
        
        let mut controller = Self {
            pci_dev,
            mmio_base: mmio_base as usize,
            mmio_virt,
            corb,
            rirb,
            bdl,
            dma_buffer: dma_buffer as *mut u8,
            dma_buffer_size: DEFAULT_BUFFER_SIZE,
            dma_pos,
            volume: 75,
            playing: false,
            current_format: None,
            output_node: 0,
            codec_addr: 0,
        };
        
        // Initialize the controller
        controller.reset()?;
        controller.setup_corb_rirb()?;
        controller.setup_dma_position()?;
        controller.enumerate_codecs()?;
        controller.setup_output()?;
        
        println!("[hda] Controller initialized successfully");
        Ok(controller)
    }
    
    /// Find HDA controller via PCI
    fn find_controller() -> Result<PciDevice, AudioError> {
        // First try to find by class/subclass
        if let Some(dev) = pci::find_device(HDA_PCI_CLASS, HDA_PCI_SUBCLASS) {
            return Ok(dev);
        }
        
        // Also check known HDA device IDs
        let known_hda_devices = [
            (VENDOR_INTEL, 0x2668), // ICH6
            (VENDOR_INTEL, 0x27D8), // ICH7
            (VENDOR_INTEL, 0x284B), // ICH8
            (VENDOR_INTEL, 0x293E), // ICH9
            (VENDOR_INTEL, 0x3A6E), // ICH10
            (VENDOR_INTEL, 0x8C20), // Lynx Point (Haswell)
            (VENDOR_INTEL, 0x9C20), // Lynx Point-LP
            (VENDOR_INTEL, 0xA170), // Sunrise Point (Skylake)
            (VENDOR_INTEL, 0xA2F0), // Kaby Lake
            (VENDOR_INTEL, 0x9D70), // Broxton
            (VENDOR_INTEL, 0xA348), // Cannon Lake
            (VENDOR_INTEL, 0x34C8), // Ice Lake
            (VENDOR_INTEL, 0xA0C8), // Tiger Lake
            (VENDOR_INTEL, 0x51C8), // Alder Lake
            (VENDOR_INTEL, 0x7A50), // Raptor Lake
            (VENDOR_AMD, 0x1457),   // AMD HD Audio
            (VENDOR_AMD, 0x15E2),   // AMD Raven Ridge
            (VENDOR_AMD, 0x1637),   // AMD Renoir
            (VENDOR_NVIDIA, 0x0BE3), // NVIDIA HDA
            (VENDOR_NVIDIA, 0x0E0F),
            (VENDOR_REALTEK, 0x0887), // Realtek ALC887
            (VENDOR_REALTEK, 0x0888), // Realtek ALC888
            (VENDOR_REALTEK, 0x0892), // Realtek ALC892
            (VENDOR_REALTEK, 0x0900), // Realtek ALC900
        ];
        
        for (vendor, device) in &known_hda_devices {
            if let Some(dev) = pci::find_device_by_id(*vendor, *device) {
                return Ok(dev);
            }
        }
        
        Err(AudioError::DeviceNotFound)
    }
    
    /// Get MMIO base address from PCI BAR0
    fn get_mmio_base(dev: &PciDevice) -> Result<u64, AudioError> {
        let bar0 = dev.bars[0];
        
        if bar0 == 0 {
            return Err(AudioError::HardwareError);
        }
        
        // Check if memory mapped (bit 0 should be 0)
        if bar0 & 1 != 0 {
            // I/O mapped, not supported
            return Err(AudioError::Unsupported);
        }
        
        // 32-bit BAR
        let base = (bar0 & 0xFFFFFFF0) as u64;
        
        // Check if 64-bit BAR
        if bar0 & 0x06 == 0x04 {
            // 64-bit BAR, use BAR1 for upper bits
            let bar1 = dev.bars[1];
            let high = (bar1 as u64) << 32;
            Ok(base | high)
        } else {
            Ok(base)
        }
    }
    
    /// Allocate DMA buffer with proper alignment
    fn alloc_dma_buffer<T>(count: usize) -> Result<*mut T, AudioError> {
        let size = core::mem::size_of::<T>() * count;
        let align = 128; // HDA requires 128-byte alignment
        
        let layout = Layout::from_size_align(size, align)
            .map_err(|_| AudioError::DmaAllocationFailed)?;
        
        let ptr = unsafe { alloc_zeroed(layout) };
        
        if ptr.is_null() {
            return Err(AudioError::DmaAllocationFailed);
        }
        
        Ok(ptr as *mut T)
    }
    
    /// Read 8-bit register
    unsafe fn read_reg8(&self, offset: usize) -> u8 {
        read_volatile((self.mmio_virt + offset) as *const u8)
    }
    
    /// Read 16-bit register
    unsafe fn read_reg16(&self, offset: usize) -> u16 {
        read_volatile((self.mmio_virt + offset) as *const u16)
    }
    
    /// Read 32-bit register
    unsafe fn read_reg32(&self, offset: usize) -> u32 {
        read_volatile((self.mmio_virt + offset) as *const u32)
    }
    
    /// Write 8-bit register
    unsafe fn write_reg8(&self, offset: usize, value: u8) {
        write_volatile((self.mmio_virt + offset) as *mut u8, value);
    }
    
    /// Write 16-bit register
    unsafe fn write_reg16(&self, offset: usize, value: u16) {
        write_volatile((self.mmio_virt + offset) as *mut u16, value);
    }
    
    /// Write 32-bit register
    unsafe fn write_reg32(&self, offset: usize, value: u32) {
        write_volatile((self.mmio_virt + offset) as *mut u32, value);
    }
    
    /// Reset the controller
    fn reset(&mut self) -> Result<(), AudioError> {
        println!("[hda] Resetting controller...");
        
        unsafe {
            // Read global capabilities
            let gcap = self.read_reg16(REG_GCAP);
            let oss = (gcap & GCAP_OSS_MASK) >> 12;
            let iss = (gcap & GCAP_ISS_MASK) >> 8;
            let bss = (gcap & GCAP_BSS_MASK) >> 4;
            let ok64 = gcap & GCAP_64OK != 0;
            
            println!("[hda] GCAP: OSS={}, ISS={}, BSS={}, 64OK={}", 
                oss, iss, bss, ok64);
            
            // Read version
            let vmin = self.read_reg8(REG_VMIN);
            let vmaj = self.read_reg8(REG_VMAJ);
            println!("[hda] Version: {}.{}", vmaj, vmin);
            
            // Reset controller
            let gctl = self.read_reg32(REG_GCTL);
            self.write_reg32(REG_GCTL, gctl & !GCTL_CRST);
            
            // Wait for reset
            let mut timeout = 1000;
            while timeout > 0 {
                let gctl = self.read_reg32(REG_GCTL);
                if gctl & GCTL_CRST == 0 {
                    break;
                }
                timeout -= 1;
                core::hint::spin_loop();
            }
            
            if timeout == 0 {
                return Err(AudioError::HardwareError);
            }
            
            // Bring out of reset
            self.write_reg32(REG_GCTL, GCTL_CRST);
            
            // Wait for ready
            timeout = 1000;
            while timeout > 0 {
                let gctl = self.read_reg32(REG_GCTL);
                if gctl & GCTL_CRST != 0 {
                    break;
                }
                timeout -= 1;
                core::hint::spin_loop();
            }
            
            if timeout == 0 {
                return Err(AudioError::HardwareError);
            }
            
            // Enable unsolicited responses
            let gctl = self.read_reg32(REG_GCTL);
            self.write_reg32(REG_GCTL, gctl | GCTL_UNSOL);
            
            // Wait a bit for codecs to wake up
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        println!("[hda] Controller reset complete");
        Ok(())
    }
    
    /// Setup CORB (Command Output Ring Buffer) and RIRB (Response Input Ring Buffer)
    fn setup_corb_rirb(&mut self) -> Result<(), AudioError> {
        println!("[hda] Setting up CORB/RIRB...");
        
        unsafe {
            // Stop CORB and RIRB
            self.write_reg8(REG_CORBCTL, 0);
            self.write_reg8(REG_RIRBCTL, 0);
            
            // Wait for stop
            while self.read_reg8(REG_CORBCTL) & CORBCTL_RUN != 0 {}
            while self.read_reg8(REG_RIRBCTL) & RIRBCTL_RUN != 0 {}
            
            // Set buffer sizes (256 entries)
            let corbsize = self.read_reg8(REG_CORBSIZE);
            let rirbsize = self.read_reg8(REG_RIRBSIZE);
            
            // Find 256-entry size capability
            let mut corb_size_bits = 0;
            for i in 0..4 {
                if corbsize & (1 << (i + 4)) != 0 {
                    corb_size_bits = i;
                    break;
                }
            }
            
            let mut rirb_size_bits = 0;
            for i in 0..4 {
                if rirbsize & (1 << (i + 4)) != 0 {
                    rirb_size_bits = i;
                    break;
                }
            }
            
            self.write_reg8(REG_CORBSIZE, corb_size_bits << 4);
            self.write_reg8(REG_RIRBSIZE, rirb_size_bits << 4);
            
            // Set base addresses
            let corb_phys = virt_to_phys_u64(self.corb as u64);
            let rirb_phys = virt_to_phys_u64(self.rirb as u64);
            
            self.write_reg32(REG_CORBLBASE, (corb_phys & 0xFFFFFFFF) as u32);
            self.write_reg32(REG_CORBUBASE, ((corb_phys >> 32) & 0xFFFFFFFF) as u32);
            
            self.write_reg32(REG_RIRBLBASE, (rirb_phys & 0xFFFFFFFF) as u32);
            self.write_reg32(REG_RIRBUBASE, ((rirb_phys >> 32) & 0xFFFFFFFF) as u32);
            
            // Reset read/write pointers
            self.write_reg16(REG_CORBRP, 1 << 15); // Reset bit
            while self.read_reg16(REG_CORBRP) & (1 << 15) == 0 {}
            self.write_reg16(REG_CORBRP, 0);
            
            self.write_reg16(REG_CORBWP, 0);
            self.write_reg16(REG_RIRBWP, 1 << 15); // Reset bit
            
            // Start CORB and RIRB
            self.write_reg8(REG_CORBCTL, CORBCTL_RUN);
            self.write_reg8(REG_RIRBCTL, RIRBCTL_RUN | RIRBCTL_RINTCTL);
        }
        
        println!("[hda] CORB/RIRB setup complete");
        Ok(())
    }
    
    /// Setup DMA position buffer
    fn setup_dma_position(&mut self) -> Result<(), AudioError> {
        unsafe {
            let dma_pos_phys = virt_to_phys_u64(self.dma_pos as u64);
            self.write_reg32(REG_DPLBASE, ((dma_pos_phys & 0xFFFFFFFF) | 1) as u32); // Bit 0 = enable
            self.write_reg32(REG_DPUBASE, ((dma_pos_phys >> 32) & 0xFFFFFFFF) as u32);
        }
        Ok(())
    }
    
    /// Send command to codec via CORB
    unsafe fn send_command(&mut self, codec: u8, node: u8, verb: u32, data: u8) -> Result<u64, AudioError> {
        let cmd = ((codec as u32) << 28) | ((node as u32) << 20) | verb | (data as u32);
        
        // Wait for space in CORB
        let mut timeout = 1000;
        loop {
            let wp = self.read_reg16(REG_CORBWP) & 0xFF;
            let rp = self.read_reg16(REG_CORBRP) & 0xFF;
            
            if ((wp + 1) & 0xFF) != rp {
                break;
            }
            
            timeout -= 1;
            if timeout == 0 {
                return Err(AudioError::HardwareError);
            }
            core::hint::spin_loop();
        }
        
        // Write command to CORB
        let wp = self.read_reg16(REG_CORBWP) & 0xFF;
        let new_wp = (wp + 1) & 0xFF;
        
        core::ptr::write_volatile(self.corb.add(new_wp as usize), cmd);
        
        // Update write pointer
        self.write_reg16(REG_CORBWP, new_wp);
        
        // Wait for response in RIRB
        let rirb_wp_before = self.read_reg16(REG_RIRBWP) & 0xFF;
        
        timeout = 1000;
        loop {
            let rirb_wp = self.read_reg16(REG_RIRBWP) & 0xFF;
            if rirb_wp != rirb_wp_before {
                // Read response
                let resp = core::ptr::read_volatile(self.rirb.add(rirb_wp as usize));
                return Ok(resp);
            }
            
            timeout -= 1;
            if timeout == 0 {
                return Err(AudioError::Timeout);
            }
            core::hint::spin_loop();
        }
    }
    
    /// Enumerate codecs connected to the controller
    fn enumerate_codecs(&mut self) -> Result<(), AudioError> {
        println!("[hda] Enumerating codecs...");
        
        unsafe {
            // Check state status for codec presence
            let statests = self.read_reg16(REG_STATESTS);
            self.write_reg16(REG_STATESTS, statests); // Clear status
            
            for i in 0..15 {
                if statests & (1 << i) != 0 {
                    println!("[hda] Found codec at address {}", i);
                    self.codec_addr = i as u8;
                    
                    // Get vendor/device ID
                    let vid_did = self.send_command(self.codec_addr, 0, 
                        CodecVerb::GetParam as u32, PARAM_VENDOR_ID)?;
                    let vendor = (vid_did >> 16) as u16;
                    let device = vid_did as u16;
                    
                    println!("[hda]   Vendor: {:#06X}, Device: {:#06X}", vendor, device);
                    
                    // Get function group count
                    let node_count = self.send_command(self.codec_addr, 0,
                        CodecVerb::GetParam as u32, PARAM_SUBORDINATE_NODE_COUNT)?;
                    let start_node = ((node_count >> 16) & 0xFF) as u8;
                    let num_nodes = (node_count & 0xFF) as u8;
                    
                    println!("[hda]   Nodes: {} to {}", start_node, start_node + num_nodes - 1);
                    
                    // Find audio function group and output widget
                    for node in start_node..(start_node + num_nodes) {
                        if let Ok(func_type) = self.send_command(self.codec_addr, node,
                            CodecVerb::GetParam as u32, PARAM_FUNCTION_GROUP_TYPE) {
                            
                            if (func_type & 0xFF) == FN_GROUP_TYPE_AUDIO as u64 {
                                println!("[hda]   Found audio function group at node {}", node);
                                self.find_output_widget(node)?;
                            }
                        }
                    }
                    
                    break; // Only handle first codec for now
                }
            }
        }
        
        if self.output_node == 0 {
            println!("[hda] Warning: No output widget found");
        }
        
        Ok(())
    }
    
    /// Find output widget (DAC + pin complex)
    fn find_output_widget(&mut self, fg_node: u8) -> Result<(), AudioError> {
        unsafe {
            // Get widget count in this function group
            let node_count = self.send_command(self.codec_addr, fg_node,
                CodecVerb::GetParam as u32, PARAM_SUBORDINATE_NODE_COUNT)?;
            let start_node = ((node_count >> 16) & 0xFF) as u8;
            let num_nodes = (node_count & 0xFF) as u8;
            
            // Look for pin complex with output support
            for node in start_node..(start_node + num_nodes) {
                let widget_cap = self.send_command(self.codec_addr, node,
                    CodecVerb::GetParam as u32, PARAM_AUDIO_WIDGET_CAP)?;
                
                let widget_type = (widget_cap >> 20) & 0xF;
                
                // 0 = Audio Output (DAC)
                // 4 = Pin Complex
                if widget_type == 4 {
                    // Check pin capabilities
                    let pin_cap = self.send_command(self.codec_addr, node,
                        CodecVerb::GetParam as u32, PARAM_PIN_CAP)?;
                    
                    // Check if it supports output
                    if pin_cap & 0x00010000 != 0 {
                        println!("[hda]   Found output pin at node {}", node);
                        self.output_node = node;
                        return Ok(());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Setup output path
    fn setup_output(&mut self) -> Result<(), AudioError> {
        if self.output_node == 0 {
            return Ok(());
        }
        
        println!("[hda] Setting up output path...");
        
        unsafe {
            // Enable pin output
            let pin_ctrl = PIN_CTRL_OUT_ENABLE | PIN_CTRL_HP_ENABLE;
            self.send_command(self.codec_addr, self.output_node,
                CodecVerb::SetPinCtrl as u32, pin_ctrl)?;
            
            // Set amplifier to unmute with 0dB gain
            // Bits: 15=Set Output Amp, 14=Set Input Amp, 13=Set Left, 12=Set Right
            // 7=Mute, 6:0=Gain
            let amp_gain = 0xB000 | 0x7F; // Set output, both channels, max gain
            self.send_command(self.codec_addr, self.output_node,
                CodecVerb::SetAmpGainMute as u32, (amp_gain >> 8) as u8)?;
            self.send_command(self.codec_addr, self.output_node,
                CodecVerb::SetAmpGainMute as u32, (amp_gain & 0xFF) as u8)?;
        }
        
        println!("[hda] Output path configured");
        Ok(())
    }
    
    /// Setup stream for playback
    fn setup_stream(&mut self, format: &AudioFormat) -> Result<(), AudioError> {
        let stream_base = STREAM_BASE + 0 * STREAM_SIZE; // Stream 0
        
        unsafe {
            // Stop the stream
            let ctl = self.read_reg32(stream_base + REG_SD_CTL);
            self.write_reg32(stream_base + REG_SD_CTL, ctl & !SD_CTL_RUN);
            
            // Wait for stop
            while self.read_reg32(stream_base + REG_SD_CTL) & SD_CTL_RUN != 0 {}
            
            // Reset the stream
            self.write_reg32(stream_base + REG_SD_CTL, SD_CTL_SRST);
            while self.read_reg32(stream_base + REG_SD_CTL) & SD_CTL_SRST == 0 {}
            
            // Wait a bit
            for _ in 0..100 {
                core::hint::spin_loop();
            }
            
            // Clear reset
            self.write_reg32(stream_base + REG_SD_CTL, 0);
            while self.read_reg32(stream_base + REG_SD_CTL) & SD_CTL_SRST != 0 {}
            
            // Setup BDL
            let bdl_phys = virt_to_phys_u64(self.bdl as u64);
            self.write_reg32(stream_base + REG_SD_BDLPL, (bdl_phys & 0xFFFFFFFF) as u32);
            self.write_reg32(stream_base + REG_SD_BDLPU, ((bdl_phys >> 32) & 0xFFFFFFFF) as u32);
            
            // Setup format
            let hda_format = StreamFormat::from_audio_format(format)
                .ok_or(AudioError::InvalidFormat)?;
            self.write_reg16(stream_base + REG_SD_FMT, hda_format);
            
            // Set buffer length (1 second worth of samples)
            let buffer_bytes = format.bytes_per_second();
            self.write_reg32(stream_base + REG_SD_CBL, buffer_bytes as u32);
            
            // Set last valid index (1 BDL entry)
            self.write_reg16(stream_base + REG_SD_LVI, 0);
            
            // Clear interrupts
            self.write_reg8(stream_base + REG_SD_STS, 0xFF);
            
            // Enable interrupts
            self.write_reg32(stream_base + REG_SD_CTL, SD_CTL_IOCE | SD_CTL_FEIE);
        }
        
        Ok(())
    }
    
    /// Start stream playback
    fn start_stream(&mut self) -> Result<(), AudioError> {
        let stream_base = STREAM_BASE + 0 * STREAM_SIZE;
        
        unsafe {
            let ctl = self.read_reg32(stream_base + REG_SD_CTL);
            self.write_reg32(stream_base + REG_SD_CTL, ctl | SD_CTL_RUN);
            
            // Wait for running
            let mut timeout = 1000;
            while timeout > 0 {
                if self.read_reg32(stream_base + REG_SD_CTL) & SD_CTL_RUN != 0 {
                    break;
                }
                timeout -= 1;
                core::hint::spin_loop();
            }
            
            if timeout == 0 {
                return Err(AudioError::HardwareError);
            }
        }
        
        Ok(())
    }
    
    /// Stop stream playback
    fn stop_stream(&mut self) {
        let stream_base = STREAM_BASE + 0 * STREAM_SIZE;
        
        unsafe {
            let ctl = self.read_reg32(stream_base + REG_SD_CTL);
            self.write_reg32(stream_base + REG_SD_CTL, ctl & !SD_CTL_RUN);
            
            // Wait for stop
            let mut timeout = 1000;
            while timeout > 0 && self.read_reg32(stream_base + REG_SD_CTL) & SD_CTL_RUN != 0 {
                timeout -= 1;
                core::hint::spin_loop();
            }
        }
    }
    
    /// Apply volume setting to hardware
    fn apply_volume(&mut self) {
        if self.output_node == 0 {
            return;
        }
        
        unsafe {
            // Convert 0-100 volume to 0-127 gain
            // Actually for simplicity, we just mute if 0
            if self.volume == 0 {
                // Mute
                let amp_gain = 0xB080; // Set output, both channels, mute
                let _ = self.send_command(self.codec_addr, self.output_node,
                    CodecVerb::SetAmpGainMute as u32, (amp_gain >> 8) as u8);
                let _ = self.send_command(self.codec_addr, self.output_node,
                    CodecVerb::SetAmpGainMute as u32, (amp_gain & 0xFF) as u8);
            } else {
                // Unmute with gain
                let gain = (self.volume as u16 * 0x7F / 100) & 0x7F;
                let amp_gain = 0xB000 | gain;
                let _ = self.send_command(self.codec_addr, self.output_node,
                    CodecVerb::SetAmpGainMute as u32, (amp_gain >> 8) as u8);
                let _ = self.send_command(self.codec_addr, self.output_node,
                    CodecVerb::SetAmpGainMute as u32, (amp_gain & 0xFF) as u8);
            }
        }
    }
}

impl AudioDevice for HdaController {
    fn name(&self) -> &str {
        "Intel HD Audio"
    }
    
    fn play(&mut self, buffer: &[u8], format: &AudioFormat) -> Result<(), AudioError> {
        if !format.is_valid() {
            return Err(AudioError::InvalidFormat);
        }
        
        if buffer.len() > self.dma_buffer_size {
            return Err(AudioError::BufferTooLarge);
        }
        
        // Stop any current playback
        self.stop();
        
        // Setup stream
        self.setup_stream(format)?;
        
        // Copy data to DMA buffer
        unsafe {
            core::ptr::copy_nonoverlapping(
                buffer.as_ptr(),
                self.dma_buffer,
                buffer.len()
            );
            
            // Setup BDL entry
            let dma_phys = virt_to_phys_u64(self.dma_buffer as u64);
            let entry = &mut *self.bdl.add(0);
            entry.set_buffer(dma_phys, buffer.len() as u32, true);
        }
        
        // Start playback
        self.start_stream()?;
        
        self.playing = true;
        self.current_format = Some(*format);
        
        Ok(())
    }
    
    fn stop(&mut self) {
        if self.playing {
            self.stop_stream();
            self.playing = false;
        }
    }
    
    fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(100);
        self.apply_volume();
    }
    
    fn is_playing(&self) -> bool {
        if !self.playing {
            return false;
        }
        
        // Check stream status
        let stream_base = STREAM_BASE + 0 * STREAM_SIZE;
        unsafe {
            let ctl = self.read_reg32(stream_base + REG_SD_CTL);
            (ctl & SD_CTL_RUN) != 0
        }
    }
    
    fn volume(&self) -> u8 {
        self.volume
    }
}

impl Drop for HdaController {
    fn drop(&mut self) {
        self.stop();
        
        // Stop CORB and RIRB
        unsafe {
            self.write_reg8(REG_CORBCTL, 0);
            self.write_reg8(REG_RIRBCTL, 0);
        }
        
        // Free DMA buffers would go here
        // Note: alloc API doesn't have a standard free function
        // In a real implementation, we'd track layouts and use dealloc
    }
}

/// Quick test to verify HDA controller detection
pub fn test_detection() {
    println!("[hda] Testing HDA controller detection...");
    
    match HdaController::find_controller() {
        Ok(dev) => {
            println!("[hda] Found controller: {:04X}:{:04X}", 
                dev.vendor_id, dev.device_id);
        }
        Err(_) => {
            println!("[hda] No HDA controller found");
        }
    }
}
