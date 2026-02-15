# Ethernet Driver Documentation (Research Phase)

## Overview

The Ethernet driver is currently in the research and skeleton implementation phase. This document outlines the architecture, implementation plan, and research notes for network interface support on Raspberry Pi 5.

## Architecture

### Raspberry Pi 5 Ethernet

The Pi 5 uses a PCIe-attached Ethernet controller:

- **Likely Controller**: Realtek RTL8111/8168
- **Interface**: PCIe x1
- **Speed**: 10/100/1000 Mbps
- **Features**: Auto-negotiation, checksum offload, VLAN

### Raspberry Pi 4 Ethernet

- **Controller**: Broadcom BCM54213PE
- **Interface**: Internal via GMII/RGMII
- **Speed**: 10/100/1000 Mbps
- **PHY**: Integrated

## Implementation Phases

### Phase 1: RTL8168 Driver (Research)

1. [ ] PCIe enumeration
2. [ ] MAC initialization
3. [ ] PHY initialization
4. [ ] DMA ring setup
5. [ ] Link detection

### Phase 2: Network Stack Integration

1. [ ] Packet transmission
2. [ ] Packet reception
3. [ ] Interrupt handling
4. [ ] Integration with webbOS net stack

## RTL8168 Controller

### PCI Identification

```
Vendor ID: 0x10EC (Realtek)
Device ID: 0x8168 (RTL8168)
Class: 0x020000 (Network / Ethernet)
```

### Register Map

```rust
pub mod rtl8168_regs {
    // MAC Address
    pub const MAC_ADDR: usize = 0x00;    // 6 bytes
    
    // Multicast
    pub const MAR: usize = 0x08;         // 8 bytes
    
    // Transmit
    pub const TNPDS: usize = 0x20;       // Tx normal priority
    pub const THPDS: usize = 0x28;       // Tx high priority
    pub const TCR: usize = 0x40;         // Tx configuration
    
    // Receive
    pub const RDSAR: usize = 0xE4;       // Rx descriptor ring
    pub const RCR: usize = 0x44;         // Rx configuration
    pub const RMS: usize = 0xDA;         // Rx max size
    
    // Command
    pub const CR: usize = 0x37;          // Command register
    pub const CPLUSCR: usize = 0xE0;     // C+ command
    
    // Interrupt
    pub const ISR: usize = 0x3E;         // Interrupt status
    pub const IMR: usize = 0x3C;         // Interrupt mask
    
    // PHY
    pub const PHYAR: usize = 0x60;       // PHY access
    pub const PHYSTATUS: usize = 0x6C;   // PHY status
    pub const TPS: usize = 0x64;         // Twister pair status
    
    // Configuration
    pub const CONFIG0: usize = 0x51;
    pub const CONFIG1: usize = 0x52;
    pub const CONFIG2: usize = 0x53;
    pub const CONFIG3: usize = 0x54;
    pub const CONFIG4: usize = 0x55;
    pub const CONFIG5: usize = 0x56;
}
```

### Command Register Bits

```rust
pub mod rtl8168_cr {
    pub const RE: u8 = 0x08;     // Receiver Enable
    pub const TE: u8 = 0x04;     // Transmitter Enable
    pub const RST: u8 = 0x10;    // Software Reset
}
```

### Interrupt Status Bits

```rust
pub mod rtl8168_isr {
    pub const ROK: u16 = 0x0001;      // Receive OK
    pub const RER: u16 = 0x0002;      // Receive Error
    pub const TOK: u16 = 0x0004;      // Transmit OK
    pub const TER: u16 = 0x0008;      // Transmit Error
    pub const LINKCHG: u16 = 0x0020;  // Link Change
    pub const RDU: u16 = 0x0040;      // Rx Descriptor Unavailable
    pub const TDU: u16 = 0x0080;      // Tx Descriptor Unavailable
}
```

### Receive Configuration

```rust
pub mod rtl8168_rcr {
    pub const AAP: u32 = 1 << 0;    // Accept All Packets
    pub const APM: u32 = 1 << 1;    // Accept Physical Match
    pub const AM: u32 = 1 << 2;     // Accept Multicast
    pub const AB: u32 = 1 << 3;     // Accept Broadcast
    pub const AR: u32 = 1 << 4;     // Append RCR (FCS)
    pub const MXDMA_MASK: u32 = 0x7 << 8;
    pub const MXDMA_UNLIMITED: u32 = 0x7 << 8;
    pub const RBLEN_MASK: u32 = 0x3 << 11;
    pub const RBLEN_8K: u32 = 0x0 << 11;
    pub const RBLEN_16K: u32 = 0x1 << 11;
    pub const RBLEN_32K: u32 = 0x2 << 11;
    pub const RBLEN_64K: u32 = 0x3 << 11;
}
```

### Initialization Sequence

```rust
fn init_rtl8168() -> Result<(), EtherError> {
    // 1. Reset controller
    mmio::write8(CR, RST);
    wait_for_reset_complete();
    
    // 2. Read MAC address
    let mac = read_mac_address();
    
    // 3. Configure Rx ring
    setup_rx_ring()?;
    
    // 4. Configure Tx ring
    setup_tx_ring()?;
    
    // 5. Configure MAC
    mmio::write32(RCR, RCR_AAP | RCR_APM | RCR_AM | RCR_AB |
                     MXDMA_UNLIMITED | RBLEN_64K);
    mmio::write32(TCR, MXDMA_UNLIMITED);
    
    // 6. Enable Rx/Tx
    mmio::write8(CR, RE | TE);
    
    // 7. Enable interrupts
    mmio::write16(IMR, ROK | TOK | LINKCHG);
    
    Ok(())
}
```

## DMA Descriptor Rings

### Receive Descriptor

```rust
#[repr(C, packed)]
pub struct RxDescriptor {
    pub status: u32,      // Frame status
    pub vlan: u16,        // VLAN tag
    pub reserved: u16,    // Reserved
    pub buffer: u32,      // Buffer address (low)
    pub buffer_hi: u32,   // Buffer address (high)
}
```

Status bits:
- Bit 0: OWN (0 = driver, 1 = hardware)
- Bit 1: EOR (End of Ring)
- Bit 2: FS (First Segment)
- Bit 3: LS (Last Segment)

### Transmit Descriptor

```rust
#[repr(C, packed)]
pub struct TxDescriptor {
    pub status: u32,      // Frame status
    pub vlan: u16,        // VLAN tag
    pub len: u16,         // Frame length
    pub buffer: u32,      // Buffer address (low)
    pub buffer_hi: u32,   // Buffer address (high)
}
```

## PHY Interface

### MII Registers

```rust
pub mod mii_regs {
    pub const CONTROL: u8 = 0;      // Basic Control
    pub const STATUS: u8 = 1;       // Basic Status
    pub const PHY_ID1: u8 = 2;      // PHY ID 1
    pub const PHY_ID2: u8 = 3;      // PHY ID 2
    pub const AN_ADVERT: u8 = 4;    // Auto-Negotiation Advertisement
    pub const AN_LPA: u8 = 5;       // Auto-Negotiation Link Partner
    pub const AN_EXP: u8 = 6;       // Auto-Negotiation Expansion
}
```

### Link Status Detection

```rust
fn check_link_status() -> bool {
    let phystatus = mmio::read8(PHYSTATUS);
    (phystatus & 0x02) != 0  // Link status bit
}

fn get_link_speed() -> LinkSpeed {
    let phystatus = mmio::read8(PHYSTATUS);
    let speed_bits = (phystatus >> 2) & 0x3;
    
    match speed_bits {
        0 => LinkSpeed::Speed10M,
        1 => LinkSpeed::Speed100M,
        2 => LinkSpeed::Speed1G,
        _ => LinkSpeed::Disconnected,
    }
}
```

## API Reference (Planned)

```rust
// Initialize Ethernet
ethernet::init();

// Get MAC address
let mac = ethernet::get_mac_address();

// Check link status
let link_up = ethernet::is_link_up();

// Get link speed
let speed = ethernet::get_link_speed();

// Send packet
ethernet::send_packet(&data);

// Receive packet
let len = ethernet::receive_packet(&mut buffer)?;

// Get statistics
let (tx, rx, tx_err, rx_err) = ethernet::get_stats();
```

## Current Status

| Feature | Status | Notes |
|---------|--------|-------|
| PCIe Detection | Skeleton | Needs PCIe enumeration |
| RTL8168 Init | Skeleton | Register definitions |
| PHY Access | Skeleton | MII read/write |
| DMA Rings | Not Started | Descriptor allocation |
| Packet TX/RX | Not Started | After DMA setup |
| Interrupts | Not Started | ISR/IMR setup |

## Research Notes

### PCIe Bus Scanning

To find Ethernet controller:

```rust
fn find_ethernet_controller() -> Option<PciDevice> {
    for bus in 0..256 {
        for device in 0..32 {
            let pci_dev = PciDevice::new(bus, device, 0);
            if pci_dev.class() == 0x020000 {
                return Some(pci_dev);
            }
        }
    }
    None
}
```

### DMA Memory Requirements

- Descriptors must be contiguous
- Alignment: 256 bytes for descriptor rings
- Buffer alignment: 8 bytes
- Cache coherency required

### Packet Reception Flow

```
1. NIC receives packet
2. DMA writes to Rx buffer
3. NIC updates Rx descriptor OWN bit
4. NIC generates interrupt (ROK)
5. Driver processes packet
6. Driver returns descriptor to NIC
```

### Packet Transmission Flow

```
1. Driver prepares packet in Tx buffer
2. Driver sets up Tx descriptor
3. Driver sets OWN bit
4. Driver notifies NIC (TD poll)
5. NIC DMAs packet
6. NIC updates descriptor, generates TOK interrupt
```

## Performance Targets

| Metric | Target |
|--------|--------|
| Throughput | 900+ Mbps (Gigabit) |
| Latency | < 100 µs |
| CPU Usage | < 5% at full rate |
| Packet Rate | 100K+ pps |

## Resources

- [RTL8168 Datasheet](https://www.realtek.com/en/component/zoo/category/network-interface-controllers-10-100-1000m-gigabit-ethernet-pci-express-software)
- [IEEE 802.3](https://standards.ieee.org/standard/802.3-2018.html)
- [Linux r8169 Driver](https://github.com/torvalds/linux/tree/master/drivers/net/ethernet/realtek)

## See Also

- [USB Documentation](usb.md) - Shared DMA concepts
- [HAL Documentation](hal.md) - Platform detection
- [Network Stack](../network.md) - Integration details
