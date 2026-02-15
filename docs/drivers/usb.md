# USB Driver Documentation (Research Phase)

## Overview

The USB driver is currently in the research and skeleton implementation phase. This document outlines the architecture, implementation plan, and research notes for USB 3.0 (XHCI) and USB 2.0 (DWC2) support on Raspberry Pi 5.

## Architecture

### Raspberry Pi 5 USB

The Pi 5 introduces USB 3.0 support via PCIe:

- **XHCI Controller**: VIA VL805 or similar PCIe USB 3.0 controller
- **DWC2 Controller**: USB 2.0 via internal bus
- **Ports**: 1x USB 3.0 + 3x USB 2.0 (shared via hub)

### Raspberry Pi 4 USB

- **PCIe XHCI**: VIA VL805 for USB 3.0
- **DWC2**: USB 2.0 OTG controller

## Implementation Phases

### Phase 1: USB 2.0 DWC2 (Complete)

1. [ ] Implement basic DWC2 initialization
2. [ ] USB device enumeration
3. [ ] Control transfers
4. [ ] Bulk transfers for mass storage

### Phase 2: USB 3.0 XHCI (Research)

1. [ ] PCIe enumeration to find XHCI
2. [ ] XHCI initialization
3. [ ] USB 3.0 device support
4. [ ] SuperSpeed transfers

## XHCI Controller

### Specification

- **XHCI Version**: 1.2
- **USB Version**: 3.2 Gen 1 (5 Gbps)
- **Max Ports**: 4
- **PCI Class**: 0x0C0330

### Register Overview

```rust
pub mod xhci_regs {
    // Capability Registers (read-only)
    pub const CAP_LENGTH: usize = 0x00;     // Capability length/version
    pub const HCI_VERSION: usize = 0x02;    // Interface version
    pub const HCS_PARAMS1: usize = 0x04;    // Structural params 1
    pub const HCS_PARAMS2: usize = 0x08;    // Structural params 2
    pub const HCS_PARAMS3: usize = 0x0C;    // Structural params 3
    pub const HCC_PARAMS1: usize = 0x10;    // Capability params 1
    pub const DBOFF: usize = 0x14;          // Doorbell offset
    pub const RTSOFF: usize = 0x18;         // Runtime regs offset
    
    // Operational Registers
    pub const USB_CMD: usize = 0x00;        // USB Command
    pub const USB_STS: usize = 0x04;        // USB Status
    pub const PAGE_SIZE: usize = 0x08;      // Page size
    pub const DNCTRL: usize = 0x14;         // Device notification
    pub const CRCR: usize = 0x18;           // Command ring control
    pub const DCBAAP: usize = 0x30;         // Device context array
    pub const CONFIG: usize = 0x38;         // Configuration
    
    // Port Status Registers (base + 0x400 + port*0x10)
    pub const PORT_STATUS_BASE: usize = 0x400;
    pub const PORTSC: usize = 0x00;         // Port status/control
    pub const PORTPMSC: usize = 0x04;       // Port power mgmt
    pub const PORTLI: usize = 0x08;         // Port link info
}
```

### Initialization Sequence

```rust
fn init_xhci() -> Result<(), UsbError> {
    // 1. Reset controller
    // 2. Wait for CNR (Controller Not Ready) clear
    // 3. Allocate memory structures
    // 4. Program DCBAAP
    // 5. Set up command ring
    // 6. Set up event ring
    // 7. Start controller (RS = 1)
    // 8. Wait for HCH (HCHalted) clear
    Ok(())
}
```

### Memory Structures

#### Device Context Base Address Array (DCBAA)

```
DCBAA[0] -> Scratchpad buffer array (if used)
DCBAA[1] -> Device Context for slot 1
DCBAA[2] -> Device Context for slot 2
...
DCBAA[n] -> Device Context for slot n
```

- Size: Up to 255 entries
- Alignment: 64 bytes
- Each entry: 64-bit pointer

#### Command Ring

```
┌─────────────────┐
│     TRB 0       │ ──┐
├─────────────────┤   │
│     TRB 1       │   │
├─────────────────┤   │ Circular buffer
│     TRB 2       │   │
├─────────────────┤   │
│      ...        │   │
├─────────────────┤   │
│     TRB n       │ ──┘
└─────────────────┘
```

- Each TRB: 16 bytes
- Alignment: 64 bytes
- Linked by Link TRB

#### Transfer Ring

Per-endpoint queue for data transfers:
- Normal TRBs for data
- Setup TRBs for control
- Status TRBs for completion

### USB Device Enumeration

```
1. Detect device connection (CSC bit in PORTSC)
2. Reset port (PR = 1)
3. Wait for reset complete (PRC = 1)
4. Determine speed from PORTSC
5. Enable slot command
6. Address device command
7. Get device descriptor
8. Get configuration descriptor
9. Set configuration
10. Load driver
```

## DWC2 Controller

### Overview

The Synopsys DesignWare Core 2 USB controller:

- **Type**: USB 2.0 OTG
- **Speeds**: Low, Full, High
- **Channels**: Multiple DMA channels

### Key Registers

```rust
pub mod dwc2_regs {
    pub const GOTGCTL: usize = 0x00;    // OTG Control
    pub const GAHBCFG: usize = 0x08;    // AHB Configuration
    pub const GUSBCFG: usize = 0x0C;    // USB Configuration
    pub const GRSTCTL: usize = 0x10;    // Reset Control
    pub const GINTSTS: usize = 0x14;    // Interrupt Status
    pub const GINTMSK: usize = 0x18;    // Interrupt Mask
    pub const GRXSTSR: usize = 0x1C;    // Receive Status
    pub const GRXFSIZ: usize = 0x24;    // Rx FIFO Size
    pub const GNPTXFSIZ: usize = 0x28;  // Non-periodic Tx FIFO
}
```

## API Reference (Planned)

```rust
// Initialize USB subsystem
usb::init();

// Get controller info
let info = usb::get_controller_info(index);

// Poll for device events
usb::poll_events();

// Send control transfer
usb::control_transfer(device, request, data);

// Send bulk transfer
usb::bulk_transfer(device, endpoint, data);
```

## Current Status

| Feature | Status | Notes |
|---------|--------|-------|
| XHCI Detection | Skeleton | Needs PCIe enumeration |
| XHCI Init | Skeleton | Register definitions |
| XHCI Memory | Not Started | DCBAAP, rings needed |
| DWC2 Support | Research | Register map defined |
| Device Enum | Not Started | After XHCI init |
| Mass Storage | Not Started | After enumeration |

## Research Notes

### PCIe Enumeration

To find XHCI controller:

1. Scan PCIe bus for Class Code 0x0C0330
2. Read BAR0 for MMIO base address
3. Map BAR into kernel address space
4. Initialize XHCI

### DMA Considerations

- USB uses DMA extensively
- Need coherent memory allocation
- Cache management required
- DMA descriptor alignment (64 bytes)

### Interrupt Handling

XHCI can use:
- MSI/MSI-X (preferred)
- Legacy PCI interrupts
- Polling mode (for debugging)

## Resources

- [XHCI Specification 1.2](https://www.intel.com/content/www/us/en/io/universal-serial-bus/extensible-host-controler-interface-usb-xhci.html)
- [USB 3.2 Specification](https://www.usb.org/usb32)
- [Synopsys DWC2 Databook](https://www.synopsys.com/dw/ipdir.php?ds=dwc_usb2_hs_otg)
- [Linux XHCI Driver](https://github.com/torvalds/linux/tree/master/drivers/usb/host)

## See Also

- [Ethernet Documentation](ethernet.md) - Network stack integration
- [HAL Documentation](hal.md) - Platform detection
