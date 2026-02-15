# Hardware Acquisition Plan for webbOS Raspberry Pi 5 Porting

**Date:** February 15, 2026  
**Prepared by:** Pierre Le Propriétaire  
**Project:** webbOS Raspberry Pi 5 Porting (Phase 2)

## Executive Summary

This document outlines the hardware requirements, acquisition strategy, and budget for porting webbOS to Raspberry Pi 5. The plan covers essential hardware for development, testing, and debugging, with consideration for backup options and cost optimization.

## 1. Core Hardware Requirements

### Primary Development Setup (Minimum Viable Configuration)

| Item | Quantity | Purpose | Estimated Cost | Priority |
|------|----------|---------|----------------|----------|
| **Raspberry Pi 5 (8GB)** | 2 | Primary development and testing boards | $135 × 2 = $270 | Critical |
| **High-quality SD Cards (64GB, Class 10/A2)** | 4 | OS images, testing different configurations | $15 × 4 = $60 | Critical |
| **USB-C Power Supplies (5V/3A)** | 2 | Reliable power for Raspberry Pi 5 | $12 × 2 = $24 | Critical |
| **USB to UART Serial Adapter (FT232RL)** | 2 | Serial console debugging via GPIO 14/15 | $15 × 2 = $30 | Critical |
| **Jumper Wires (Female-Female)** | 20 | Connecting serial adapter to GPIO pins | $5 | Critical |
| **Ethernet Cables** | 2 | Network connectivity for development | $8 × 2 = $16 | High |
| **USB 3.0 Flash Drive (128GB)** | 1 | Additional storage for testing | $25 | Medium |

**Subtotal (Primary Setup):** $430

### Optional/Enhanced Configuration

| Item | Quantity | Purpose | Estimated Cost | Priority |
|------|----------|---------|----------------|----------|
| **Raspberry Pi 5 (4GB)** | 1 | Testing lower memory configurations | $80 | Medium |
| **Raspberry Pi 5 Active Cooler** | 2 | Thermal management for extended testing | $8 × 2 = $16 | Medium |
| **Official Raspberry Pi 5 Case** | 2 | Protection and organization | $10 × 2 = $20 | Medium |
| **HDMI Cables** | 2 | Display output testing | $10 × 2 = $20 | Low |
| **USB Keyboard/Mouse Combo** | 1 | Input device testing | $25 | Low |
| **MicroSD Card Reader** | 1 | Faster image writing | $15 | Medium |
| **Heat Sinks** | 2 | Additional cooling | $5 × 2 = $10 | Low |

**Subtotal (Enhanced):** $186

### Backup/Redundancy Hardware

| Item | Quantity | Purpose | Estimated Cost | Priority |
|------|----------|---------|----------------|----------|
| **Extra Raspberry Pi 5 (8GB)** | 1 | Backup in case of hardware failure | $135 | Medium |
| **Extra SD Cards** | 2 | Backup storage media | $15 × 2 = $30 | Medium |
| **Extra Power Supply** | 1 | Backup power source | $12 | Medium |
| **Extra Serial Adapter** | 1 | Backup debugging tool | $15 | Medium |

**Subtotal (Backup):** $192

## 2. Total Budget Requirements

| Configuration | Estimated Cost |
|---------------|----------------|
| Primary Setup | $430 |
| Enhanced Setup | $186 |
| Backup Hardware | $192 |
| **Total Maximum** | **$808** |
| **Total Minimum (Primary only)** | **$430** |

## 3. Hardware Specifications Analysis

### Raspberry Pi 5 Key Specifications for webbOS

#### CPU & Memory
- **Processor:** Broadcom BCM2712 quad-core Arm Cortex-A76 @ 2.4GHz
- **L2 Cache:** 512KB per core
- **L3 Cache:** 2MB shared
- **Memory Options:** 4GB or 8GB LPDDR4X-4267 SDRAM
- **Recommended:** 8GB for webbOS development and testing

#### I/O & Connectivity
- **USB:** 2× USB 3.0 (5 Gbps), 2× USB 2.0
- **PCIe:** PCIe 2.0 x1 interface (via M.2 HAT)
- **Ethernet:** Gigabit Ethernet (PoE+ via HAT)
- **Wireless:** Dual-band 802.11ac Wi-Fi, Bluetooth 5.0
- **GPIO:** 40-pin header with UART, SPI, I2C, PWM

#### Storage & Boot
- **Primary Storage:** MicroSD card (SDR104 mode)
- **Alternative Boot:** USB mass storage, network boot
- **Expansion:** PCIe for NVMe SSD via M.2 HAT

#### Display & Video
- **GPU:** VideoCore VII
- **Display Output:** Dual 4Kp60 HDMI
- **Camera/Display:** Dual 4-lane MIPI interfaces

### Hardware Limitations & Considerations

1. **Memory Constraints:**
   - 4GB vs 8GB configurations affect webbOS capabilities
   - webbOS currently uses ~20MB kernel + applications
   - 4GB sufficient for basic testing, 8GB recommended for full features

2. **Storage Performance:**
   - MicroSD cards slower than NVMe via PCIe
   - Consider M.2 HAT for performance testing
   - VFAT32 write performance may be SD card limited

3. **Power Requirements:**
   - Raspberry Pi 5 requires 5V/3A minimum
   - Active cooling recommended for sustained loads
   - Power spikes during boot/initialization

4. **Debugging Limitations:**
   - No JTAG debugging without additional hardware
   - Serial console via UART is primary debugging method
   - Limited hardware breakpoints

## 4. Acquisition Timeline

### Week 1: Immediate Purchase (Days 1-3)
1. **Order Priority 1 Items:**
   - 2× Raspberry Pi 5 (8GB)
   - 4× High-quality SD cards (64GB, Class 10/A2)
   - 2× USB-C power supplies
   - 2× USB to UART serial adapters
   - Jumper wires

2. **Expected Delivery:** 3-5 business days
3. **Budget Allocation:** $430

### Week 1: Secondary Purchase (Days 4-7)
1. **Order Priority 2 Items (if budget allows):**
   - Active coolers and cases
   - Ethernet cables
   - USB flash drive
   - MicroSD card reader

2. **Expected Delivery:** 5-7 business days
3. **Budget Allocation:** $81

### Week 2: Backup Hardware (If Needed)
1. **Order based on initial testing results**
2. **Consider additional Raspberry Pi 5 if hardware issues arise**
3. **Budget Allocation:** $192 (contingency)

## 5. Vendor Recommendations

### Primary Vendors (US-based)
1. **Raspberry Pi Official Resellers:**
   - PiShop.us
   - Canakit.com
   - Adafruit.com
   - Sparkfun.com

2. **General Electronics:**
   - Amazon.com (check for official resellers)
   - Microcenter.com (in-store pickup available)
   - Digikey.com
   - Mouser.com

3. **Specialized Components:**
   - Waveshare.com (for specialized debug adapters)
   - Cytron.io (Raspberry Pi accessories)

### International Considerations
- Check local Raspberry Pi distributors
- Consider shipping times and import duties
- Verify warranty and support availability

## 6. Cost Optimization Strategies

### Tiered Acquisition Approach
1. **Start with Minimum Viable Configuration ($430)**
   - Test basic functionality
   - Validate porting approach
   - Identify additional needs

2. **Expand Based on Testing Results**
   - Purchase enhanced components as needed
   - Add backup hardware if issues arise
   - Scale based on project success

### Alternative Cost-Saving Options
1. **Use Existing Hardware:**
   - Raspberry Pi 4 (8GB) for initial testing
   - Reuse SD cards from other projects
   - Share serial adapters between team members

2. **Lower-cost Alternatives:**
   - Raspberry Pi 5 (4GB) instead of 8GB for some units
   - Generic USB-C power supplies
   - Basic cases instead of official ones

3. **Rental/Lease Options:**
   - Consider short-term hardware rental
   - University/educational discounts
   - Open source hardware lending programs

## 7. Risk Mitigation

### Hardware Failure Risks
1. **SD Card Corruption:**
   - Mitigation: Multiple SD cards, regular backups
   - Recovery: Image cloning tools

2. **Power Supply Issues:**
   - Mitigation: High-quality power supplies, voltage monitoring
   - Recovery: Backup power supplies

3. **Raspberry Pi Hardware Defects:**
   - Mitigation: Multiple units, purchase from reputable vendors
   - Recovery: Warranty claims, spare units

### Supply Chain Risks
1. **Stock Availability:**
   - Mitigation: Order early, multiple vendors
   - Alternative: Raspberry Pi 4 as fallback

2. **Shipping Delays:**
   - Mitigation: Express shipping option, local vendors
   - Alternative: Virtual testing while waiting

3. **Price Fluctuations:**
   - Mitigation: Fixed budget with contingency
   - Monitoring: Track price trends

## 8. Success Metrics

### Hardware Acquisition Success Criteria
1. **Timeliness:** All hardware received within 7 business days
2. **Functionality:** 100% of components functional on arrival
3. **Budget Adherence:** Actual cost within 10% of estimate
4. **Completeness:** All critical components available for Week 2 development

### Quality Assurance Checklist
- [ ] Raspberry Pi 5 boots to Raspberry Pi OS
- [ ] Serial console accessible via UART adapter
- [ ] SD cards writable and bootable
- [ ] Network connectivity functional
- [ ] Power supplies provide stable 5V/3A

## 9. Next Steps

### Immediate Actions (Next 24 hours)
1. [ ] Finalize budget approval
2. [ ] Place orders for Priority 1 items
3. [ ] Set up tracking for shipments
4. [ ] Prepare workspace for hardware arrival

### Preparation While Waiting
1. [ ] Set up cross-compilation environment
2. [ ] Prepare SD card imaging scripts
3. [ ] Create serial debugging documentation
4. [ ] Test QEMU ARM64 emulation

### Hardware Arrival Checklist
1. [ ] Inspect all components for damage
2. [ ] Test each Raspberry Pi 5 individually
3. [ ] Verify serial console functionality
4. [ ] Create baseline SD card images
5. [ ] Document hardware configurations

## 10. Appendix

### Recommended SD Cards
- SanDisk Extreme Pro (A2, V30)
- Samsung EVO Select (A2, U3)
- Kingston Canvas React Plus (A2, V90)

### Recommended Serial Adapters
- FT232RL-based adapters (FTDI chipset)
- CP2102-based adapters (lower cost alternative)
- Raspberry Pi Debug Probe (official solution)

### Useful Accessories
- GPIO breakout boards for easier connection
- Bench power supply for voltage testing
- Logic analyzer for signal debugging
- Thermal camera for heat distribution analysis

### Contact Information
- **Hardware Lead:** Pierre Le Propriétaire
- **Budget Approval:** Claude Le Comptable
- **Technical Coordination:** Ingrid L'Ingénieure
- **Project Management:** Björn Le Bâtisseur

---

*This plan will be updated as hardware is acquired and tested. Last updated: February 15, 2026*