# Hardware Setup Timeline for Raspberry Pi 5 Porting

**Date:** February 15, 2026  
**Project:** webbOS Raspberry Pi 5 Porting  
**Phase:** 2 (Core Porting - Weeks 2-3)

## Executive Summary

This document outlines a phased hardware acquisition and setup timeline for the Raspberry Pi 5 porting project. The approach minimizes upfront costs while ensuring hardware is available when needed for critical path development.

## 1. Hardware Requirements Matrix

### Essential Hardware (Must Have):

| Item | Quantity | Purpose | Critical Path | Cost |
|------|----------|---------|---------------|------|
| Raspberry Pi 5 (8GB) | 1 | Primary development board | Week 3, Day 1 | $80 |
| SD Card (64GB A2) | 2 | Boot media + backup | Week 2, Day 3 | $30 |
| USB Serial Adapter | 1 | Serial console debugging | Week 3, Day 1 | $12 |
| Power Supply | 1 | Reliable power | Week 3, Day 1 | $15 |
| Ethernet Cable | 1 | Network connectivity | Week 2, Day 5 | $5 |
| **Subtotal** | | | | **$142** |

### Recommended Backup Hardware:

| Item | Quantity | Purpose | When Needed | Cost |
|------|----------|---------|-------------|------|
| Raspberry Pi 5 (8GB) | 1 | Backup/testing board | Week 4 | $80 |
| SD Card (64GB A2) | 2 | Additional test media | Week 4 | $30 |
| USB Serial Adapter | 1 | Backup debug console | Week 4 | $12 |
| **Subtotal** | | | | **$122** |

### Optional/Enhancement Hardware:

| Item | Purpose | Benefit | Cost |
|------|---------|---------|------|
| USB WiFi Adapter | Wireless testing | Flexibility | $15 |
| HDMI Cable | Display output | Visual debugging | $10 |
| USB Keyboard/Mouse | Direct input | Hardware testing | $40 |
| Logic Analyzer | Signal debugging | Advanced debugging | $100 |
| **Subtotal** | | | **$165** |

**Total Hardware Budget Range:** $142 - $429

## 2. Acquisition Timeline

### Phase 1: Immediate Acquisition (Week 2, Days 1-3)

**Budget:** $142  
**Priority:** CRITICAL PATH

| Day | Item | Action | Expected Delivery |
|-----|------|--------|-------------------|
| **Day 1** | Raspberry Pi 5 | Order from authorized retailer | 2-3 business days |
| **Day 1** | SD Cards | Order from Amazon/Newegg | 1-2 business days |
| **Day 2** | USB Serial Adapter | Order from electronics supplier | 2-3 business days |
| **Day 2** | Power Supply | Order with Raspberry Pi | 2-3 business days |
| **Day 3** | Ethernet Cable | Local purchase or order | 1-2 business days |

**Week 2 Goal:** All essential hardware ordered by Day 3.

### Phase 2: Backup Hardware (Week 3, Days 5-7)

**Budget:** $122  
**Priority:** RISK MITIGATION

| Timing | Item | Trigger Condition |
|--------|------|-------------------|
| Week 3, Day 5 | Backup Raspberry Pi 5 | If primary board has issues |
| Week 3, Day 6 | Additional SD cards | If frequent reflashing needed |
| Week 3, Day 7 | Backup serial adapter | If debugging becomes bottleneck |

**Decision Point:** Assess hardware reliability by Week 3, Day 4.

### Phase 3: Enhancement Hardware (Week 4+)

**Budget:** $165  
**Priority:** OPTIONAL

| Timing | Item | Justification |
|--------|------|---------------|
| Week 4 | USB WiFi Adapter | If wireless testing required |
| Week 5 | HDMI cable | If display debugging needed |
| Week 6 | Logic analyzer | If signal-level debugging required |

## 3. Setup and Configuration Timeline

### Week 2: Preparation Phase

**Goal:** Software environment ready before hardware arrives.

| Day | Task | Duration | Dependencies |
|-----|------|----------|--------------|
| **Day 1-2** | Cross-compilation setup | 4 hours | None |
| **Day 2-3** | QEMU ARM64 configuration | 3 hours | Toolchain installed |
| **Day 3-4** | Build system adaptation | 4 hours | Target specification |
| **Day 4-5** | Initial code analysis | 6 hours | Codebase access |
| **Day 5-7** | ARM64 target specification | 8 hours | Architecture analysis |

**Week 2 Deliverables:**
- ✅ Cross-compilation working
- ✅ QEMU ARM64 booting test kernel
- ✅ Build system adapted for ARM64
- ✅ Detailed architecture analysis

### Week 3: Hardware Integration

**Goal:** First successful boot on Raspberry Pi 5.

| Day | Task | Hardware Required | Success Criteria |
|-----|------|-------------------|------------------|
| **Day 1** | Hardware unboxing & inspection | All essential hardware | All components functional |
| **Day 1** | SD card preparation | SD cards, card reader | Bootable test image |
| **Day 2** | Serial console setup | USB serial adapter | Serial output visible |
| **Day 2** | First boot attempt | Raspberry Pi 5, power | Any output on serial |
| **Day 3** | UART driver implementation | - | Kernel serial output |
| **Day 4** | Basic memory initialization | - | Memory maps working |
| **Day 5** | Interrupt controller setup | - | Timer interrupts working |
| **Day 6** | Basic driver porting | - | Console I/O functional |
| **Day 7** | Validation testing | - | Stable boot 10+ times |

**Week 3 Deliverables:**
- ✅ Raspberry Pi 5 boots webbOS kernel
- ✅ Serial console fully functional
- ✅ Basic drivers working
- ✅ Stable operation demonstrated

## 4. Risk Management Plan

### Hardware Risks:

| Risk | Probability | Impact | Mitigation | Trigger for Action |
|------|------------|--------|------------|-------------------|
| **Raspberry Pi DOA** | 5% | High | Order from reputable supplier | No power/indicator on arrival |
| **SD card corruption** | 15% | Medium | Multiple cards, regular backups | Boot failures, file corruption |
| **Serial adapter issues** | 10% | Medium | Backup adapter, test early | No serial output |
| **Power supply failure** | 8% | Medium | Quality PSU, voltage monitor | Random resets, instability |
| **ESD damage** | 3% | High | Anti-static precautions | Intermittent failures |

### Mitigation Strategies:

1. **Supplier Selection:**
   - Use authorized Raspberry Pi resellers
   - Check warranty terms (minimum 1 year)
   - Read reviews for reliability

2. **Testing Protocol:**
   - Test all hardware within 24 hours of arrival
   - Keep original packaging for returns
   - Document serial numbers

3. **Backup Strategy:**
   - Maintain disk images of working configurations
   - Use version control for SD card contents
   - Regular backups to cloud/local storage

## 5. Cost Optimization Strategies

### Tiered Acquisition Approach:

**Level 1: Minimum Viable Setup ($142)**
- 1× Raspberry Pi 5
- 2× SD cards
- Basic peripherals
- *Covers 90% of development needs*

**Level 2: Robust Development Setup ($264)**
- Level 1 + backup Raspberry Pi
- Additional SD cards
- Backup serial adapter
- *Reduces downtime risk*

**Level 3: Full Featured Lab ($429)**
- Level 2 + enhancement hardware
- Display capabilities
- Advanced debugging tools
- *Maximum productivity*

### Recommended Path:
1. Start with Level 1 ($142)
2. Monitor hardware reliability for 1 week
3. Upgrade to Level 2 if issues arise
4. Consider Level 3 only for specific needs

### Alternative Cost Savings:

1. **Use Existing Peripherals:**
   - Keyboard/mouse from existing setup
   - Monitor with HDMI input
   - Network via existing infrastructure

2. **Cloud Alternatives:**
   - AWS/GCP ARM64 instances for testing ($0.10-0.20/hour)
   - GitHub Actions for CI/CD (free for open source)

3. **Community Resources:**
   - Borrow hardware from local makerspace
   - Use university/company lab equipment
   - Participate in hardware loan programs

## 6. Setup Checklist

### Pre-arrival Preparation (Week 2):

- [ ] Cross-compilation toolchain installed
- [ ] QEMU ARM64 configured
- [ ] Build system adapted
- [ ] Serial terminal software installed
- [ ] Workspace prepared (static-safe area)
- [ ] Network configuration planned

### Day 1 Setup:

- [ ] Unbox and inspect all components
- [ ] Test power supply with multimeter (optional)
- [ ] Prepare first SD card with test image
- [ ] Connect serial adapter to development machine
- [ ] Verify serial communication (loopback test)

### Day 2 Integration:

- [ ] Assemble Raspberry Pi (minimal configuration)
- [ ] Insert SD card with test kernel
- [ ] Connect serial adapter to GPIO pins
- [ ] Apply power, monitor serial output
- [ ] Document any issues or anomalies

### Day 3-7 Development:

- [ ] Iterate on kernel modifications
- [ ] Test each change in QEMU first
- [ ] Deploy to hardware for validation
- [ ] Document successful configurations
- [ ] Create recovery images

## 7. Success Metrics

### Hardware Setup Success:

1. **Time to First Boot:** < 48 hours after hardware arrival
2. **Serial Console Reliability:** 100% connection success rate
3. **Boot Stability:** 10+ consecutive successful boots
4. **Development Velocity:** < 5 minute deploy-test cycle

### Cost Efficiency Metrics:

1. **Hardware Utilization:** > 80% of available development time
2. **Downtime:** < 5% due to hardware issues
3. **Return on Investment:** Value created > 3× hardware cost
4. **Budget Adherence:** Actual spend within 10% of plan

## 8. Vendor Recommendations

### Primary Suppliers:

1. **Raspberry Pi 5:**
   - [Adafruit](https://www.adafruit.com) - Reliable, good support
   - [Pimoroni](https://shop.pimoroni.com) - Raspberry Pi specialists
   - [SparkFun](https://www.sparkfun.com) - Quality components

2. **SD Cards:**
   - SanDisk Extreme or Samsung EVO Select
   - Purchase from Amazon (sold by Amazon)
   - Avoid no-name brands

3. **Serial Adapters:**
   - FTDI-based adapters (FT232RL chipset)
   - CP2102 or CH340 as alternatives
   - Avoid counterfeit FTDI chips

4. **Power Supplies:**
   - Official Raspberry Pi 5 PSU
   - Anker or RAVPower USB-C PD supplies
   - Ensure 5V/3A minimum rating

### Price Monitoring:

- Set up price alerts on camelcamelcamel.com
- Check rpilocator.com for stock availability
- Consider used market for backup hardware

## 9. Long-term Maintenance Plan

### Ongoing Costs:

| Item | Frequency | Cost | Purpose |
|------|-----------|------|---------|
| SD card replacement | Every 6 months | $15 | Wear leveling |
| Backup power supply | As needed | $15 | Replacement |
| Cleaning supplies | Annual | $10 | Dust prevention |
| **Annual Maintenance:** | | **$40** | |

### Hardware Refresh Cycle:

- **Primary board:** Replace every 2-3 years
- **SD cards:** Replace annually or as needed
- **Peripherals:** Replace on failure
- **Test equipment:** Upgrade based on needs

### Depreciation Schedule:

- Year 1: 30% depreciation
- Year 2: 50% depreciation  
- Year 3: 70% depreciation
- Year 4+: Consider for replacement

## 10. Emergency Response Plan

### Hardware Failure Response:

1. **Immediate Actions:**
   - Switch to backup hardware if available
   - Use QEMU for continued development
   - Document failure symptoms

2. **Diagnosis:**
   - Isolate failed component
   - Test with known-good replacements
   - Check for warranty coverage

3. **Recovery:**
   - Order replacement if under warranty
   - Use alternative development methods
   - Adjust timeline if necessary

### Communication Plan:

- Daily hardware status updates
- Immediate notification of critical failures
- Weekly budget vs actual spend report
- Monthly hardware utilization review

---

**Prepared by:** Claude Le Comptable  
**Date:** February 15, 2026  
**Status:** READY FOR IMPLEMENTATION

*This timeline will be updated based on actual hardware arrival and setup experience.*