# Phase 2 Resource Plan: ARM64 Kernel Porting for Raspberry Pi 5

**Date:** February 15, 2026  
**Project:** webbOS Raspberry Pi 5 Porting  
**Phase:** 2 (Core Porting - Weeks 2-3)  
**Prepared by:** Claude Le Comptable, Finance & Resource Planning Specialist

## Executive Summary

Based on analysis of the existing webbOS codebase and the 7-week porting plan, Phase 2 requires focused resource allocation for ARM64 kernel porting. This phase involves significant architectural changes from x86_64 to ARM64, requiring specialized tools, hardware, and development time.

## 1. Current Architecture Analysis

### Existing Codebase (x86_64)
- **Architecture:** x86_64 with UEFI boot
- **Assembly:** x86_64 inline assembly throughout kernel
- **I/O:** Port I/O (inb/outb) for device communication
- **Interrupts:** PIC (8259A) with IDT-based handling
- **Memory:** x86_64 paging with 4-level page tables
- **Boot:** UEFI bootloader with OVMF firmware

### Target Architecture (ARM64/Raspberry Pi 5)
- **Architecture:** ARM64 (Cortex-A76)
- **Assembly:** AArch64 assembly required
- **I/O:** Memory-mapped I/O (MMIO) instead of port I/O
- **Interrupts:** ARM GIC (Generic Interrupt Controller)
- **Memory:** ARM64 paging with different MMU configuration
- **Boot:** Raspberry Pi firmware or custom bootloader

## 2. Resource Requirements for Phase 2

### 2.1 Hardware Requirements

#### Essential Hardware:
1. **Raspberry Pi 5 (8GB RAM)** - Primary development target
   - **Quantity:** 2 (primary + backup)
   - **Cost:** $80 × 2 = $160
   - **Purpose:** Native testing, hardware validation

2. **High-speed SD Cards (64GB, Class 10/A2)**
   - **Quantity:** 4
   - **Cost:** $15 × 4 = $60
   - **Purpose:** Boot media, test images, backups

3. **USB Serial Adapter (FTDI-based)**
   - **Quantity:** 2
   - **Cost:** $12 × 2 = $24
   - **Purpose:** Serial console debugging (GPIO 14/15)

4. **Power Supplies (USB-C, 5V/3A)**
   - **Quantity:** 2
   - **Cost:** $15 × 2 = $30
   - **Purpose:** Reliable power for development boards

5. **Network Connectivity**
   - **Ethernet cables:** 2 × $5 = $10
   - **USB WiFi adapter (optional):** $15

#### Total Hardware Cost: $299

### 2.2 Software & Toolchain Requirements

#### Cross-compilation Toolchain:
1. **ARM64 GCC Toolchain**
   - **Cost:** Free (open source)
   - **Source:** `aarch64-linux-gnu-gcc` package
   - **Purpose:** Cross-compilation from x86_64 to ARM64

2. **Rust ARM64 Targets**
   - **Cost:** Free
   - **Targets:** `aarch64-unknown-none`, `aarch64-unknown-none-softfloat`
   - **Setup:** `rustup target add aarch64-unknown-none`

3. **QEMU ARM64 Emulation**
   - **Cost:** Free
   - **Version:** QEMU 8.0+ with ARM64 support
   - **Purpose:** Rapid testing without hardware

#### Development Tools:
1. **Debugging Tools**
   - OpenOCD for ARM debugging: Free
   - GDB with ARM64 support: Free
   - Serial terminal software: Free (screen, minicom)

2. **Build Automation**
   - Custom build scripts: Development time
   - CI/CD pipeline setup: Development time

### 2.3 Development Time Allocation

#### Phase 2 Timeline (Weeks 2-3):
**Total: 80 developer hours (2 weeks × 40 hours)**

| Task | Hours | Priority | Description |
|------|-------|----------|-------------|
| **Week 2: Foundation Setup** | **40** | | |
| ARM64 target specification | 8 | High | Create `aarch64-unknown-none.json` |
| Cross-compilation setup | 6 | High | Toolchain configuration |
| QEMU ARM64 testing env | 4 | Medium | Emulation environment |
| Architecture analysis | 6 | High | Detailed x86_64→ARM64 mapping |
| Build system adaptation | 8 | High | Makefile updates |
| Initial codebase audit | 8 | Medium | Identify all arch-specific code |
| **Week 3: Core Porting** | **40** | | |
| ARM64 assembly porting | 12 | High | Convert inline assembly |
| Memory management port | 10 | High | ARM64 paging system |
| Interrupt handling (GIC) | 10 | High | ARM GIC vs x86 PIC |
| Basic driver adaptation | 8 | Medium | Console, timer drivers |
| **Buffer & Contingency** | **20** | | |
| Testing & debugging | 10 | High | QEMU and hardware testing |
| Documentation | 5 | Medium | Porting guide updates |
| Contingency | 5 | - | Unforeseen issues |

**Total Phase 2 Hours: 80**

## 3. Technical Challenges & Mitigation

### High-Risk Areas:

1. **Inline Assembly Conversion**
   - **Risk:** x86_64 assembly throughout codebase
   - **Mitigation:** Create ARM64 equivalents, use Rust abstractions where possible
   - **Fallback:** Implement critical paths in pure Rust

2. **Memory Management Differences**
   - **Risk:** x86_64 vs ARM64 paging differences
   - **Mitigation:** Study ARM64 memory model, use existing Rust ARM64 OS references
   - **Fallback:** Start with identity mapping, refine later

3. **Interrupt Controller**
   - **Risk:** PIC vs GIC completely different
   - **Mitigation:** Use `arm-gic` crate, study Raspberry Pi 5 GIC documentation
   - **Fallback:** Basic polling for initial testing

4. **Device I/O**
   - **Risk:** Port I/O (inb/outb) vs Memory-Mapped I/O
   - **Mitigation:** Create MMIO abstraction layer
   - **Fallback:** Stub implementations for initial boot

### Cost Optimization Strategies:

1. **Use QEMU for 80% of Development**
   - Reduces hardware wear
   - Faster iteration cycles
   - Only use real hardware for final validation

2. **Leverage Open Source Tools**
   - All toolchains are free
   - Use existing ARM64 Rust OS projects as reference
   - Contribute improvements back to community

3. **Phased Hardware Acquisition**
   - Start with 1 Raspberry Pi 5
   - Add second unit only if needed
   - Use existing peripherals where possible

## 4. Budget Allocation for Phase 2

### Phase 2 Budget Breakdown:

| Category | Item | Quantity | Unit Cost | Total | Notes |
|----------|------|----------|-----------|-------|-------|
| **Hardware** | | | | **$299** | |
| | Raspberry Pi 5 (8GB) | 2 | $80 | $160 | Primary + backup |
| | SD Cards (64GB A2) | 4 | $15 | $60 | Boot media + backups |
| | USB Serial Adapters | 2 | $12 | $24 | Debug console |
| | Power Supplies | 2 | $15 | $30 | Reliable power |
| | Ethernet Cables | 2 | $5 | $10 | Network connectivity |
| | USB WiFi Adapter | 1 | $15 | $15 | Optional wireless |
| **Software** | | | | **$0** | |
| | Toolchains | - | Free | $0 | Open source |
| | QEMU | - | Free | $0 | Emulation |
| | Development Tools | - | Free | $0 | Open source |
| **Development** | | | | **$4,000** | |
| | Developer Time | 80 hours | $50/hour | $4,000 | Based on market rates |
| **Contingency** | | | | **$430** | |
| | 10% Buffer | - | - | $430 | For unforeseen costs |
| **TOTAL** | | | | **$4,729** | |

### Full Project Budget Update (7 Weeks):

| Phase | Weeks | Hardware | Development | Total |
|-------|-------|----------|-------------|-------|
| Phase 1 | Week 1 | $0 | $2,000 | $2,000 |
| **Phase 2** | **Weeks 2-3** | **$299** | **$4,000** | **$4,729** |
| Phase 3 | Weeks 4-5 | $100 | $4,000 | $4,100 |
| Phase 4 | Week 6 | $50 | $2,000 | $2,050 |
| Phase 5 | Week 7 | $50 | $2,000 | $2,050 |
| **Project Total** | **7 Weeks** | **$499** | **$14,000** | **$14,929** |

*Note: Development costs based on 280 total hours at $50/hour market rate.*

## 5. Toolchain Recommendations

### Primary Development Environment:

1. **Cross-compilation from x86_64:**
   ```
   # Required packages
   sudo apt-get install gcc-aarch64-linux-gnu qemu-system-arm
   
   # Rust targets
   rustup target add aarch64-unknown-none
   rustup target add aarch64-unknown-none-softfloat
   ```

2. **Build Configuration:**
   - Create `aarch64-unknown-none.json` target specification
   - Update Makefile with ARM64 targets
   - Separate build profiles for QEMU vs Raspberry Pi

3. **Testing Strategy:**
   - **QEMU ARM64:** `qemu-system-aarch64 -M virt` for rapid testing
   - **Raspberry Pi 5:** Actual hardware for validation
   - **Serial Debug:** UART over GPIO 14/15 at 115200 baud

### Recommended Development Workflow:

1. **Local Development (x86_64):**
   ```
   # Build for ARM64
   cargo build --target aarch64-unknown-none
   
   # Test in QEMU
   qemu-system-aarch64 -M virt -cpu cortex-a72 \
     -kernel target/aarch64-unknown-none/debug/kernel \
     -serial stdio
   ```

2. **Hardware Testing:**
   ```
   # Copy to SD card
   cp kernel8.img /media/sd-card/
   
   # Monitor serial output
   screen /dev/ttyUSB0 115200
   ```

## 6. Hardware Setup Timeline

### Week 2 (Immediate):
- **Day 1-2:** Order Raspberry Pi 5 hardware
- **Day 3-4:** Set up cross-compilation toolchain
- **Day 5-7:** Configure QEMU ARM64 environment

### Week 3 (Development):
- **Day 1-3:** Initial ARM64 kernel boot in QEMU
- **Day 4-5:** First hardware test (serial console working)
- **Day 6-7:** Basic drivers functional

### Critical Path Items:
1. **Serial Console Setup:** Must be working by Day 5 of Week 3
2. **SD Card Preparation:** Ready before first hardware test
3. **Backup Hardware:** Available by Week 4 for parallel testing

## 7. Risk Management

### Technical Risks:
1. **ARM64 Assembly Complexity**
   - **Probability:** Medium
   - **Impact:** High
   - **Mitigation:** Allocate extra time, use reference implementations

2. **Hardware Compatibility Issues**
   - **Probability:** Low
   - **Impact:** Medium
   - **Mitigation:** Test early, keep hardware simple

3. **Toolchain Bugs**
   - **Probability:** Low
   - **Impact:** Medium
   - **Mitigation:** Use stable toolchain versions

### Project Risks:
1. **Timeline Slippage**
   - **Probability:** Medium
   - **Impact:** Medium
   - **Mitigation:** 20-hour contingency buffer in schedule

2. **Hardware Failure**
   - **Probability:** Low
   - **Impact:** High
   - **Mitigation:** Backup hardware available

## 8. Success Metrics for Phase 2

### Minimum Viable Outcomes:
1. ✅ ARM64 kernel boots in QEMU
2. ✅ Serial console output working
3. ✅ Basic memory management functional
4. ✅ Cross-compilation pipeline established

### Target Outcomes:
1. ✅ Kernel boots on Raspberry Pi 5 hardware
2. ✅ Basic drivers (UART, timer) working
3. ✅ Build system fully adapted for ARM64
4. ✅ Documentation for ARM64 porting complete

### Stretch Goals:
1. ⭐ VFAT32 read operations working on ARM64
2. ⭐ Basic shell functionality
3. ⭐ Performance benchmarks established

## 9. Deliverables Checklist

### By End of Week 2:
- [ ] ARM64 target specification created
- [ ] Cross-compilation toolchain working
- [ ] QEMU ARM64 testing environment
- [ ] Detailed architecture analysis document
- [ ] Build system adapted for ARM64

### By End of Week 3:
- [ ] ARM64 kernel boots in QEMU
- [ ] Serial console output working
- [ ] Basic ARM64 drivers ported
- [ ] First successful boot on Raspberry Pi 5
- [ ] Phase 2 completion report

## 10. Next Steps

### Immediate Actions (Next 48 hours):
1. **Order Hardware:** Raspberry Pi 5, SD cards, serial adapters
2. **Setup Development Environment:** Install ARM64 toolchain
3. **Create Target Specification:** `aarch64-unknown-none.json`
4. **Begin Code Analysis:** Identify all architecture-specific code

### Week 2 Focus:
1. **Build System:** Adapt Makefile for ARM64
2. **QEMU Testing:** Establish emulation workflow
3. **Initial Porting:** Start with simplest components

### Week 3 Focus:
1. **Core Porting:** Memory management, interrupts
2. **Hardware Testing:** First Raspberry Pi 5 boot
3. **Documentation:** Update porting guide

---

**Prepared by:** Claude Le Comptable  
**Date:** February 15, 2026  
**Status:** READY FOR IMPLEMENTATION

*This resource plan will be updated as Phase 2 progresses.*