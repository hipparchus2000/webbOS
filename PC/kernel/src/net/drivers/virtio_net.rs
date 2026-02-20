//! VirtIO Network Driver
//!
//! Implementation of VirtIO 1.0 network device driver.
//! Supports QEMU/KVM virtio-net-pci device.

use core::mem::size_of;
use core::sync::atomic::{fence, Ordering};
use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Mutex;

use crate::net::{MacAddress, NetworkInterface, NetError};
use crate::net;
use crate::drivers::pci::{read_config8, read_config16, read_config32};
use crate::mm::{phys_to_virt, virt_to_phys_u64};
use crate::println;

/// VirtIO PCI device IDs
const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_NET_DEVICE_ID: u16 = 0x1041;

/// VirtIO PCI configuration offsets
const VIRTIO_PCI_DEVICE_FEATURES: usize = 0x00;
const VIRTIO_PCI_GUEST_FEATURES: usize = 0x04;
const VIRTIO_PCI_QUEUE_PFN: usize = 0x08;
const VIRTIO_PCI_QUEUE_NUM: usize = 0x0C;
const VIRTIO_PCI_QUEUE_SEL: usize = 0x0E;
const VIRTIO_PCI_QUEUE_NOTIFY: usize = 0x10;
const VIRTIO_PCI_STATUS: usize = 0x12;
const VIRTIO_PCI_ISR: usize = 0x13;

/// VirtIO device status flags
const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
const VIRTIO_STATUS_FAILED: u8 = 128;

/// VirtIO network device feature bits
const VIRTIO_NET_F_MAC: u32 = 1 << 5;
const VIRTIO_NET_F_STATUS: u32 = 1 << 16;
const VIRTIO_NET_F_MRG_RXBUF: u32 = 1 << 15;

/// VirtQueue descriptor
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

/// VirtQueue available ring
#[repr(C)]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; 0], // Variable size
}

/// VirtQueue used element
#[repr(C)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

/// VirtQueue used ring
#[repr(C)]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; 0], // Variable size
}

/// VirtQueue
struct VirtQueue {
    queue_size: u16,
    descriptors: *mut VirtqDesc,
    available: *mut VirtqAvail,
    used: *mut VirtqUsed,
    /// Index in available ring for next descriptor
    avail_idx: u16,
    /// Index in used ring for next processed element
    used_idx: u16,
    /// Physical addresses for the queue
    desc_phys: u64,
    avail_phys: u64,
    used_phys: u64,
}

impl VirtQueue {
    /// Create new VirtQueue
    fn new(size: u16) -> Option<Self> {
        if size == 0 || (size & (size - 1)) != 0 {
            return None; // Must be power of 2
        }

        // Allocate descriptor table (16 bytes each)
        let desc_size = (size as usize) * size_of::<VirtqDesc>();
        let desc_ptr = alloc_dma(desc_size)?;
        
        // Allocate available ring (6 bytes + 2*size)
        let avail_size = 6 + (size as usize) * 2;
        let avail_ptr = alloc_dma(avail_size)?;
        
        // Allocate used ring (4 bytes + 8*size)
        let used_size = 4 + (size as usize) * size_of::<VirtqUsedElem>();
        let used_ptr = alloc_dma(used_size)?;

        // Clear descriptors
        unsafe {
            core::ptr::write_bytes(desc_ptr, 0, desc_size);
            core::ptr::write_bytes(avail_ptr, 0, avail_size);
            core::ptr::write_bytes(used_ptr, 0, used_size);
        }

        let desc_phys = virt_to_phys_u64(desc_ptr as u64);
        let avail_phys = virt_to_phys_u64(avail_ptr as u64);
        let used_phys = virt_to_phys_u64(used_ptr as u64);

        Some(Self {
            queue_size: size,
            descriptors: desc_ptr as *mut VirtqDesc,
            available: avail_ptr as *mut VirtqAvail,
            used: used_ptr as *mut VirtqUsed,
            avail_idx: 0,
            used_idx: 0,
            desc_phys,
            avail_phys,
            used_phys,
        })
    }

    /// Add buffer to queue
    fn add_buffer(&mut self, buffers: &[(u64, usize, bool)]) -> Option<u16> {
        let num_bufs = buffers.len();
        if num_bufs == 0 || num_bufs > self.queue_size as usize {
            return None;
        }

        // Find free descriptors (simple: use ring buffer approach)
        let start_idx = self.avail_idx % self.queue_size;
        
        unsafe {
            for (i, (addr, len, write)) in buffers.iter().enumerate() {
                let desc = &mut *self.descriptors.add(((start_idx as usize + i) % self.queue_size as usize));
                desc.addr = *addr;
                desc.len = *len as u32;
                desc.flags = if *write { 2 } else { 0 } | if i < num_bufs - 1 { 1 } else { 0 };
                desc.next = if i < num_bufs - 1 {
                    ((start_idx + i as u16 + 1) % self.queue_size)
                } else {
                    0
                };
            }

            // Add to available ring
            let avail = &mut *self.available;
            let ring_ptr = (avail as *mut VirtqAvail as *mut u8).add(4) as *mut u16;
            *ring_ptr.add((avail.idx % self.queue_size) as usize) = start_idx;
            
            fence(Ordering::SeqCst);
            avail.idx = avail.idx.wrapping_add(1);
        }

        self.avail_idx = self.avail_idx.wrapping_add(num_bufs as u16);
        Some(start_idx)
    }

    /// Check if there are used buffers
    fn has_used(&self) -> bool {
        unsafe {
            let used = &*self.used;
            used.idx != self.used_idx
        }
    }

    /// Get next used buffer
    fn get_used(&mut self) -> Option<(u16, u32)> {
        if !self.has_used() {
            return None;
        }

        unsafe {
            let used = &*self.used;
            let elem = &*(&used.ring as *const [VirtqUsedElem; 0] as *const VirtqUsedElem)
                .add((self.used_idx % self.queue_size) as usize);
            
            self.used_idx = self.used_idx.wrapping_add(1);
            Some((elem.id as u16, elem.len))
        }
    }

    /// Get physical addresses for queue setup
    fn get_phys(&self) -> (u64, u64, u64) {
        (self.desc_phys, self.avail_phys, self.used_phys)
    }
}

/// VirtIO Network Device
struct VirtioNetDevice {
    base_addr: u32,
    mac: MacAddress,
    mtu: usize,
    receive_queue: Mutex<VirtQueue>,
    transmit_queue: Mutex<VirtQueue>,
    link_up: Mutex<bool>,
    /// Receive buffers
    rx_buffers: Mutex<Vec<(u64, *mut u8)>>,
    /// Transmit buffer (single for simplicity)
    tx_buffer: Mutex<(u64, *mut u8)>,
}

/// Allocate DMA-capable memory
fn alloc_dma(size: usize) -> Option<*mut u8> {
    use alloc::alloc::{alloc_zeroed, Layout};
    
    // Round up to page size
    let size = ((size + 4095) / 4096) * 4096;
    
    let layout = Layout::from_size_align(size, 4096).ok()?;
    let ptr = unsafe { alloc_zeroed(layout) };
    
    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}

/// Read from PCI BAR
unsafe fn pci_read8(base: u32, offset: usize) -> u8 {
    core::ptr::read_volatile((base as usize + offset) as *const u8)
}

unsafe fn pci_read16(base: u32, offset: usize) -> u16 {
    core::ptr::read_volatile((base as usize + offset) as *const u16)
}

unsafe fn pci_read32(base: u32, offset: usize) -> u32 {
    core::ptr::read_volatile((base as usize + offset) as *const u32)
}

/// Write to PCI BAR
unsafe fn pci_write8(base: u32, offset: usize, val: u8) {
    core::ptr::write_volatile((base as usize + offset) as *mut u8, val);
}

unsafe fn pci_write16(base: u32, offset: usize, val: u16) {
    core::ptr::write_volatile((base as usize + offset) as *mut u16, val);
}

unsafe fn pci_write32(base: u32, offset: usize, val: u32) {
    core::ptr::write_volatile((base as usize + offset) as *mut u32, val);
}

impl VirtioNetDevice {
    /// Initialize VirtIO network device
    fn new(base_addr: u32) -> Option<Self> {
        // Reset device
        unsafe {
            pci_write8(base_addr, VIRTIO_PCI_STATUS, 0);
        }

        // Acknowledge device
        unsafe {
            let status = pci_read8(base_addr, VIRTIO_PCI_STATUS);
            pci_write8(base_addr, VIRTIO_PCI_STATUS, status | VIRTIO_STATUS_ACKNOWLEDGE);
        }

        // We know how to drive this device
        unsafe {
            let status = pci_read8(base_addr, VIRTIO_PCI_STATUS);
            pci_write8(base_addr, VIRTIO_PCI_STATUS, status | VIRTIO_STATUS_DRIVER);
        }

        // Read device features
        let device_features = unsafe { pci_read32(base_addr, VIRTIO_PCI_DEVICE_FEATURES) };
        
        // Negotiate features (we want MAC support)
        let wanted_features = VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS;
        let guest_features = device_features & wanted_features;
        
        unsafe {
            pci_write32(base_addr, VIRTIO_PCI_GUEST_FEATURES, guest_features);
        }

        // Features OK
        unsafe {
            let status = pci_read8(base_addr, VIRTIO_PCI_STATUS);
            pci_write8(base_addr, VIRTIO_PCI_STATUS, status | VIRTIO_STATUS_FEATURES_OK);
        }

        // Read MAC address if supported
        let has_mac = (guest_features & VIRTIO_NET_F_MAC) != 0;
        let mac = if has_mac {
            // MAC is at offset 0x14 in device config
            let mac_bytes: [u8; 6] = unsafe {
                [
                    pci_read8(base_addr, 0x14),
                    pci_read8(base_addr, 0x15),
                    pci_read8(base_addr, 0x16),
                    pci_read8(base_addr, 0x17),
                    pci_read8(base_addr, 0x18),
                    pci_read8(base_addr, 0x19),
                ]
            };
            MacAddress::new(mac_bytes)
        } else {
            MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56])
        };

        // Create receive queue (queue 0)
        let rx_queue = VirtQueue::new(256)?;
        unsafe {
            pci_write16(base_addr, VIRTIO_PCI_QUEUE_SEL, 0);
            pci_write16(base_addr, VIRTIO_PCI_QUEUE_NUM, 256);
            // For legacy virtio, we write the PFN
            // For modern virtio, we'd use the capability structure
            let (desc, avail, used) = rx_queue.get_phys();
            pci_write32(base_addr, VIRTIO_PCI_QUEUE_PFN, (desc >> 12) as u32);
        }

        // Create transmit queue (queue 1)
        let tx_queue = VirtQueue::new(256)?;
        unsafe {
            pci_write16(base_addr, VIRTIO_PCI_QUEUE_SEL, 1);
            pci_write16(base_addr, VIRTIO_PCI_QUEUE_NUM, 256);
            let (desc, avail, used) = tx_queue.get_phys();
            pci_write32(base_addr, VIRTIO_PCI_QUEUE_PFN, (desc >> 12) as u32);
        }

        // DRIVER_OK
        unsafe {
            let status = pci_read8(base_addr, VIRTIO_PCI_STATUS);
            pci_write8(base_addr, VIRTIO_PCI_STATUS, status | VIRTIO_STATUS_DRIVER_OK);
        }

        // Allocate and populate receive buffers
        let mut rx_buffers = Vec::new();
        for _ in 0..128 {
            let buf = alloc_dma(2048)?; // 2KB buffers
            let phys = virt_to_phys_u64(buf as u64);
            rx_buffers.push((phys, buf));
        }

        // Allocate transmit buffer
        let tx_buf = alloc_dma(2048)?;
        let tx_phys = virt_to_phys_u64(tx_buf as u64);

        let mut device = Self {
            base_addr,
            mac,
            mtu: 1500,
            receive_queue: Mutex::new(rx_queue),
            transmit_queue: Mutex::new(tx_queue),
            link_up: Mutex::new(false),
            rx_buffers: Mutex::new(rx_buffers),
            tx_buffer: Mutex::new((tx_phys, tx_buf)),
        };

        // Fill receive queue with buffers
        device.fill_rx_queue();

        Some(device)
    }

    /// Fill receive queue with buffers
    fn fill_rx_queue(&self) {
        let mut queue = self.receive_queue.lock();
        let buffers = self.rx_buffers.lock();

        for (phys, _virt) in buffers.iter().take(64) {
            queue.add_buffer(&[(*phys + 12, 2036, true)]); // Offset for virtio_net_hdr
        }

        // Notify device
        unsafe {
            pci_write16(self.base_addr, VIRTIO_PCI_QUEUE_NOTIFY, 0);
        }
    }
}

// SAFETY: VirtioNetDevice is only accessed from a single thread
unsafe impl Send for VirtioNetDevice {}
unsafe impl Sync for VirtioNetDevice {}

impl NetworkInterface for VirtioNetDevice {
    fn name(&self) -> &str {
        "virtio-net"
    }

    fn mac_address(&self) -> MacAddress {
        self.mac
    }

    fn mtu(&self) -> usize {
        self.mtu
    }

    fn send(&self, data: &[u8]) -> Result<usize, NetError> {
        if data.len() > self.mtu {
            return Err(NetError::PacketTooLarge);
        }

        let mut queue = self.transmit_queue.lock();
        let tx_buf = self.tx_buffer.lock();

        unsafe {
            // Copy data to transmit buffer (after virtio_net_hdr)
            const HDR_SIZE: usize = 12; // sizeof(struct virtio_net_hdr)
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                (tx_buf.1).add(HDR_SIZE),
                data.len()
            );

            // Clear virtio header
            core::ptr::write_bytes(tx_buf.1, 0, HDR_SIZE);
        }

        // Add to transmit queue
        if queue.add_buffer(&[(tx_buf.0, 12 + data.len(), false)]).is_none() {
            return Err(NetError::NoBuffer);
        }

        // Notify device
        unsafe {
            pci_write16(self.base_addr, VIRTIO_PCI_QUEUE_NOTIFY, 1);
        }

        Ok(data.len())
    }

    fn receive(&self, buf: &mut [u8]) -> Result<usize, NetError> {
        let mut queue = self.receive_queue.lock();

        if !queue.has_used() {
            return Err(NetError::NoBuffer);
        }

        if let Some((id, len)) = queue.get_used() {
            let rx_buffers = self.rx_buffers.lock();
            
            // Find the buffer
            if let Some((phys, virt)) = rx_buffers.get(id as usize) {
                let hdr_size = 12; // virtio_net_hdr
                let data_len = (len as usize).saturating_sub(hdr_size);
                let copy_len = data_len.min(buf.len());

                unsafe {
                    core::ptr::copy_nonoverlapping(
                        virt.add(hdr_size),
                        buf.as_mut_ptr(),
                        copy_len
                    );
                }

                // Re-add buffer to queue
                queue.add_buffer(&[(*phys + hdr_size as u64, 2048 - hdr_size, true)]);
                
                // Notify device
                unsafe {
                    pci_write16(self.base_addr, VIRTIO_PCI_QUEUE_NOTIFY, 0);
                }

                return Ok(copy_len);
            }
        }

        Err(NetError::NoBuffer)
    }

    fn is_link_up(&self) -> bool {
        *self.link_up.lock()
    }
}

/// Initialize VirtIO network driver
pub fn init() {
    // Scan PCI for VirtIO network device
    if let Some(device) = find_virtio_net_device() {
        println!("[virtio-net] Found device at {:08X}", device.base_addr);
        
        if let Some(net_dev) = VirtioNetDevice::new(device.base_addr) {
            let mac = net_dev.mac_address();
            let mac_str = mac.format();
            let mac_str = core::str::from_utf8(&mac_str).unwrap_or("?");
            
            println!("[virtio-net] MAC: {}", mac_str);
            
            // Register with network stack
            net::register_interface(Box::new(net_dev));
        } else {
            println!("[virtio-net] Failed to initialize device");
        }
    }
}

/// PCI device info
struct PciDevice {
    bus: u8,
    slot: u8,
    func: u8,
    vendor: u16,
    device: u16,
    base_addr: u32,
}

/// Find VirtIO network device on PCI bus
fn find_virtio_net_device() -> Option<PciDevice> {
    // Scan all PCI buses (simplified)
    for bus in 0..256u16 {
        for slot in 0..32u8 {
            let vendor = read_config32(bus as u8, slot, 0, 0) as u16;
            
            if vendor == 0xFFFF || vendor != VIRTIO_VENDOR_ID {
                continue;
            }

            let device = (read_config32(bus as u8, slot, 0, 0) >> 16) as u16;
            
            if device == VIRTIO_NET_DEVICE_ID {
                // Read BAR0 for base address
                let bar0 = read_config32(bus as u8, slot, 0, 0x10);
                let base_addr = if bar0 & 1 == 0 {
                    // Memory mapped
                    bar0 & 0xFFFFFFF0
                } else {
                    // I/O mapped
                    (bar0 & 0xFFFFFFFC) | 0x80000000 // Mark as I/O
                };

                return Some(PciDevice {
                    bus: bus as u8,
                    slot,
                    func: 0,
                    vendor,
                    device,
                    base_addr,
                });
            }
        }
    }

    None
}
