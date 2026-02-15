# Phase 2 Startup Checklist

**Date:** February 15, 2026  
**Project:** webbOS Raspberry Pi 5 Porting  
**Phase:** 2 (Core Porting - Weeks 2-3)

## Immediate Actions (Today)

### 1. Budget Approval
- [ ] Review Phase 2 budget: $4,866
- [ ] Approve minimal hardware purchase: $200
- [ ] Authorize Week 2 development work

### 2. Hardware Ordering
- [ ] Order 1× Raspberry Pi 5 (8GB): $135
- [ ] Order 2× SD cards (64GB A2): $30
- [ ] Order 1× USB-C power supply: $15
- [ ] Order 1× FTDI serial adapter: $15
- [ ] Order jumper wires: $5
- **Total: $200**

### 3. Development Environment Setup
- [ ] Install ARM64 cross-compilation toolchain:
  ```bash
  sudo apt-get install gcc-aarch64-linux-gnu qemu-system-arm
  ```
- [ ] Add Rust ARM64 targets:
  ```bash
  rustup target add aarch64-unknown-none
  rustup target add aarch64-unknown-none-softfloat
  ```
- [ ] Verify QEMU ARM64 installation:
  ```bash
  qemu-system-aarch64 --version
  ```

## Week 2 Goals (February 16-22)

### Toolchain & Build System
- [ ] Create `aarch64-unknown-none.json` target specification
- [ ] Update Makefile for ARM64 builds
- [ ] Test cross-compilation with simple test program
- [ ] Configure QEMU ARM64 testing environment

### Code Analysis
- [ ] Identify all inline assembly (`asm!`) in codebase
- [ ] Document port I/O usage (`inb`/`outb`)
- [ ] Map x86_64 interrupts to ARM64 equivalents
- [ ] Analyze memory management differences

### Documentation
- [ ] Create architecture porting guide
- [ ] Document build process for ARM64
- [ ] Update project status with Phase 2 start
- [ ] Create testing procedures

## Week 3 Preparation

### Hardware Setup (When hardware arrives)
- [ ] Test Raspberry Pi 5 with Raspberry Pi OS
- [ ] Configure serial console (115200 baud)
- [ ] Prepare bootable SD card with test kernel
- [ ] Verify serial communication works

### Development Readiness
- [ ] Have basic ARM64 kernel ready for testing
- [ ] Prepare serial output driver
- [ ] Create recovery SD card images
- [ ] Set up debugging environment (GDB, OpenOCD)

## Success Criteria

### By End of Week 2:
- [ ] Cross-compilation working
- [ ] QEMU ARM64 booting test kernel
- [ ] Build system adapted for ARM64
- [ ] Architecture analysis complete

### By End of Week 3:
- [ ] ARM64 kernel boots in QEMU
- [ ] Serial console functional on hardware
- [ ] Basic drivers ported (UART, timer)
- [ ] Stable boot demonstrated

## Risk Mitigation

### If Hardware Delayed:
- [ ] Focus on QEMU testing
- [ ] Complete toolchain optimization
- [ ] Work on architecture abstraction
- [ ] Document porting procedures

### If Technical Challenges:
- [ ] Use existing ARM64 Rust OS references
- [ ] Simplify initial goals (boot → console → basic drivers)
- [ ] Leverage community resources
- [ ] Adjust timeline if necessary

## Communication Plan

### Daily Updates:
- Progress against checklist
- Any blockers or issues
- Hardware arrival status

### Weekly Review:
- Budget vs actual spend
- Timeline vs plan
- Risk assessment update
- Next week planning

## Resources

### Documentation:
- `PHASE2_RESOURCE_PLAN.md` - Detailed resource allocation
- `TOOLCHAIN_RECOMMENDATIONS.md` - Setup guide
- `HARDWARE_SETUP_TIMELINE.md` - Hardware integration plan

### Reference Materials:
- Raspberry Pi 5 datasheet
- ARM64 architecture reference
- Existing ARM64 Rust OS projects

### Team Contacts:
- **Technical Lead:** Ingrid L'Ingénieure
- **Hardware:** Pierre Le Propriétaire  
- **Finance:** Claude Le Comptable
- **Project Management:** Björn Le Bâtisseur

---

**Status:** READY TO START  
**Next Review:** February 22, 2026 (End of Week 2)

*Checklist will be updated weekly based on progress.*