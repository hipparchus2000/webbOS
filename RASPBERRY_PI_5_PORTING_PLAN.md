# Raspberry Pi 5 Porting Plan for VFAT32 Write Code

**Date:** February 15, 2026  
**Current Branch:** `feature-vfat32-driver`  
**Target Platform:** Raspberry Pi 5 (ARM64/aarch64)  
**Current Platform:** x86_64 (QEMU/UEFI)  
**Status:** PLANNING PHASE

## Executive Summary

This document outlines a comprehensive plan to port the existing VFAT32 write functionality from the x86_64 architecture to Raspberry Pi 5 (ARM64). The port involves architecture analysis, build system adaptation, dependency mapping, and multi-agent orchestration.

## 1. Architecture Analysis: x86_64 vs ARM64 (Raspberry Pi 5)

### Current Platform (x86_64)
- **Architecture:** x86_64 (64-bit Intel/AMD)
- **Boot Method:** UEFI with OVMF firmware
- **Memory Model:** Little-endian
- **Assembly:** x86_64 assembly instructions
- **Target Triple:** `x86_64-unknown-none`
- **Bootloader Target:** `x86_64-unknown-uefi`
- **Testing Environment:** QEMU with x86_64 emulation

### Target Platform (Raspberry Pi 5)
- **Architecture:** ARM64 (Cortex-A76 cores)
- **Boot Method:** U-Boot or Raspberry Pi firmware
- **Memory Model:** Little-endian (compatible)
- **Assembly:** AArch64 assembly instructions
- **Target Triple:** `aarch64-unknown-none` or `aarch64-unknown-none-softfloat`
- **Bootloader:** Custom or U-Boot based
- **Hardware Features:**
  - 4× Cortex-A76 cores @ 2.4GHz
  - 8GB LPDDR4X RAM
  - PCIe 2.0 interface
  - USB 3.0 ports
  - SD card interface
  - VideoCore VII GPU

### Key Architectural Differences
1. **Instruction Set:** x86_64 vs AArch64 (completely different assembly)
2. **Boot Process:** UEFI vs Raspberry Pi firmware chain
3. **Memory Management:** Different MMU configurations
4. **Interrupt Handling:** APIC vs ARM GIC (Generic Interrupt Controller)
5. **PCI Enumeration:** Different methods for device discovery
6. **Timer System:** HPET vs ARM system timers

## 2. Dependencies Mapping

### Current Dependencies (x86_64)
```
- spin = "0.9" (no architecture-specific code)
- bitflags = "2.4" (architecture-agnostic)
- bit_field = "0.10" (architecture-agnostic)
- volatile = "0.6" (architecture-agnostic)
- linked_list_allocator = "0.10" (architecture-agnostic)
- lazy_static = "1.4" (architecture-agnostic)
```

### Dependencies Requiring Adaptation
1. **Inline Assembly:** Any `asm!()` blocks need ARM64 equivalents
2. **Memory Barriers:** `mfence`, `sfence`, `lfence` → ARM64 `dmb`, `dsb`, `isb`
3. **Port I/O:** x86 `inb`/`outb` → ARM memory-mapped I/O
4. **Interrupts:** CLI/STI → ARM CPSR manipulation
5. **CPU Features:** CPUID → ARM MIDR_EL1 and feature registers

### New Dependencies for ARM64
1. **aarch64-cpu:** For ARM64 CPU features and registers
2. **tock-registers:** For memory-mapped register access
3. **arm-gic:** For Generic Interrupt Controller support
4. **pl011:** For UART serial output (Raspberry Pi console)

## 3. Build System Changes

### Current Build System (x86_64)
```makefile
# x86_64 targets
CARGO_TARGET_KERNEL = x86_64-unknown-none
CARGO_TARGET_BOOTLOADER = x86_64-unknown-uefi
QEMU = qemu-system-x86_64
```

### Required Changes for ARM64

#### Option A: Cross-compilation from x86_64
```makefile
# ARM64 targets
CARGO_TARGET_KERNEL = aarch64-unknown-none
CARGO_TARGET_BOOTLOADER = aarch64-unknown-uefi  # or custom bootloader
QEMU = qemu-system-aarch64

# Cross-compilation toolchain
RUST_TARGET = aarch64-unknown-none
CROSS_COMPILE = aarch64-linux-gnu-
```

#### Option B: Native Build on Raspberry Pi 5
- Install Rust toolchain directly on Raspberry Pi OS
- Build natively with `cargo build --target aarch64-unknown-none`
- Requires sufficient RAM (8GB available)

#### Recommended Approach: Hybrid
1. **Development:** Cross-compile from x86_64 for rapid iteration
2. **Testing:** Native build on Raspberry Pi 5 for final validation
3. **CI/CD:** Both cross-compilation and native build verification

### Build Configuration Changes
1. **New target specification:** `aarch64-unknown-none.json`
2. **Linker script:** ARM64-specific memory layout
3. **Bootloader:** Raspberry Pi compatible boot method
4. **QEMU configuration:** `-M raspi3b` or `-M virt` for ARM64

## 4. Testing Strategy

### Testing Phases

#### Phase 1: QEMU ARM64 Emulation
- **Tool:** `qemu-system-aarch64`
- **Machine:** `-M virt` (generic ARM virtual machine)
- **Advantages:** Fast iteration, no hardware required
- **Limitations:** May not match Raspberry Pi 5 hardware exactly

#### Phase 2: Raspberry Pi 4/5 Hardware Testing
- **Hardware:** Actual Raspberry Pi 5 board
- **Method:** SD card boot with test kernel
- **Debugging:** Serial console over UART (GPIO 14/15)
- **Validation:** Real hardware behavior

#### Phase 3: VFAT32 Write Specific Tests
1. **Basic Write Operations:**
   - File creation on FAT32 partition
   - File modification and truncation
   - Directory creation and deletion

2. **Cluster Management:**
   - Cluster allocation/deallocation
   - FAT table updates
   - Bad cluster handling

3. **Error Recovery:**
   - Power loss simulation
   - Corrupted FAT recovery
   - Disk full scenarios

4. **Performance Testing:**
   - Write throughput measurement
   - Latency under different cluster sizes
   - Concurrent file operations

### Test Automation
```bash
# Example test script
#!/bin/bash
# 1. Build for ARM64
cargo build --target aarch64-unknown-none

# 2. Create test disk image
dd if=/dev/zero of=test.img bs=1M count=64
mkfs.fat -F32 test.img

# 3. Run in QEMU
qemu-system-aarch64 -M virt -kernel target/aarch64-unknown-none/debug/kernel \
  -drive file=test.img,format=raw -serial stdio

# 4. Run on actual Raspberry Pi (manual)
# scp kernel.img to SD card and boot
```

## 5. Agent Orchestration Plan

### Agent Roles and Responsibilities

#### **Björn Le Bâtisseur** (Project Structure/Branch Management)
- **Tasks:**
  1. Create new feature branch `pi` from `feature-vfat32-driver`
  2. Set up branch protection rules
  3. Establish CI/CD pipeline for ARM64 builds
  4. Coordinate between technical agents
  5. Track progress against timeline

- **Deliverables:**
  - `pi` branch created and configured
  - Build pipeline for ARM64
  - Project tracking dashboard

#### **Sofia La Savante** (Research Pi 5 Specs/Limitations)
- **Tasks:**
  1. Research Raspberry Pi 5 hardware specifications
  2. Document boot process and firmware requirements
  3. Identify ARM64-specific constraints
  4. Research existing Rust OS projects on Raspberry Pi
  5. Document memory map and peripheral addresses

- **Deliverables:**
  - Raspberry Pi 5 hardware reference document
  - Boot process flowchart
  - Memory map documentation
  - List of known issues with Rust on ARM64

#### **Ingrid L'Ingénieure** (Technical Porting Work)
- **Tasks:**
  1. Create ARM64 target specification
  2. Port inline assembly to ARM64
  3. Adapt interrupt handling for ARM GIC
  4. Implement ARM64 memory management
  5. Port PCI enumeration for Raspberry Pi
  6. Test VFAT32 write on ARM64

- **Deliverables:**
  - Working ARM64 kernel
  - Ported VFAT32 write functionality
  - ARM64 device drivers
  - Technical porting guide

#### **Pierre Le Propriétaire** (Hardware Setup/Location)
- **Tasks:**
  1. Acquire Raspberry Pi 5 hardware
  2. Set up development environment (SD cards, power, peripherals)
  3. Configure serial console for debugging
  4. Establish secure hardware access
  5. Manage hardware inventory

- **Deliverables:**
  - Ready-to-use Raspberry Pi 5 setup
  - Serial debugging configuration
  - Hardware access procedures
  - Backup hardware if needed

#### **Claude Le Comptable** (Budget/Cost Analysis)
- **Tasks:**
  1. Calculate Raspberry Pi 5 hardware costs
  2. Estimate additional peripherals (SD cards, cables, etc.)
  3. Budget for potential hardware failures
  4. Calculate time investment vs. value
  5. Provide cost-benefit analysis

- **Deliverables:**
  - Hardware budget spreadsheet
  - Cost-benefit analysis report
  - Purchase recommendations
  - ROI estimation

### Agent Coordination Workflow
```
Week 1: Research & Planning
  Sofia → Research complete
  Björn → Branch created
  Claude → Budget approved
  Pierre → Hardware ordered

Week 2-3: Core Porting
  Ingrid → ARM64 kernel booting
  Sofia → Technical support
  Björn → CI/CD setup

Week 4-5: VFAT32 Porting
  Ingrid → VFAT32 write ported
  Pierre → Hardware testing setup
  Björn → Integration testing

Week 6: Validation
  All agents → Comprehensive testing
  Ingrid → Bug fixes
  Björn → Documentation
```

## 6. Timeline

### Phase 1: Preparation (Week 1)
- **Days 1-2:** Research and planning (Sofia, Björn)
- **Days 3-4:** Hardware acquisition (Pierre, Claude)
- **Days 5-7:** Development environment setup

### Phase 2: Core Porting (Weeks 2-3)
- **Week 2:** ARM64 kernel booting (Ingrid)
- **Week 3:** Basic drivers and memory management

### Phase 3: VFAT32 Porting (Weeks 4-5)
- **Week 4:** VFAT32 write adaptation
- **Week 5:** Integration and basic testing

### Phase 4: Testing & Validation (Week 6)
- **Days 1-3:** QEMU testing
- **Days 4-5:** Raspberry Pi hardware testing
- **Days 6-7:** Bug fixes and optimization

### Phase 5: Documentation & Handoff (Week 7)
- **Final week:** Documentation, performance tuning, project handoff

**Total Estimated Duration:** 7 weeks

## 7. Risk Assessment

### Technical Risks
1. **ARM64 Assembly Complexity:** Medium risk
   - **Mitigation:** Use existing Rust ARM64 OS projects as reference
   - **Fallback:** Implement critical parts in Rust with minimal assembly

2. **Raspberry Pi Boot Process:** Medium risk
   - **Mitigation:** Study existing bootloaders (Raspberry Pi firmware, U-Boot)
   - **Fallback:** Use chainloading from existing firmware

3. **Hardware Compatibility:** Low risk
   - **Mitigation:** Test on multiple Raspberry Pi models (4 and 5)
   - **Fallback:** Focus on QEMU if hardware issues persist

4. **VFAT32 Write Stability:** High risk
   - **Mitigation:** Extensive testing with different cluster sizes
   - **Fallback:** Implement robust error recovery

### Project Risks
1. **Agent Coordination:** Medium risk
   - **Mitigation:** Daily standups, clear communication channels
   - **Fallback:** Björn as central coordinator with escalation path

2. **Hardware Availability:** Low risk
   - **Mitigation:** Order hardware early, have backup suppliers
   - **Fallback:** Use Raspberry Pi 4 for initial development

3. **Timeline Slippage:** Medium risk
   - **Mitigation:** Buffer time in schedule, prioritize MVP features
   - **Fallback:** Extend timeline if necessary

## 8. Success Criteria

### Minimum Viable Product (MVP)
1. ✅ Kernel boots on Raspberry Pi 5 (serial output visible)
2. ✅ Basic memory management working
3. ✅ VFAT32 read operations functional
4. ✅ VFAT32 write operations functional (create, modify, delete files)
5. ✅ Stable operation for 24+ hours

### Full Success Criteria
1. **Performance:** Write speeds within 80% of theoretical maximum
2. **Reliability:** No data corruption in 1000+ write cycles
3. **Compatibility:** Works with standard FAT32-formatted SD cards
4. **Recovery:** Handles power loss gracefully
5. **Documentation:** Complete porting guide for future ARM64 projects

### Validation Metrics
1. **Boot Time:** < 5 seconds from power on to shell
2. **Write Throughput:** > 10 MB/s sustained
3. **Error Rate:** < 0.1% of operations
4. **Memory Usage:** < 256MB for kernel and filesystem
5. **CPU Utilization:** < 50% during heavy write operations

## 9. Deliverables

### Technical Deliverables
1. `pi` feature branch with ARM64 support
2. ARM64 target specification (`aarch64-unknown-none.json`)
3. Ported VFAT32 write implementation
4. Raspberry Pi 5 device drivers
5. Build scripts for cross-compilation and native build
6. Test suite for ARM64 VFAT32 operations

### Documentation Deliverables
1. Porting guide: x86_64 to ARM64
2. Raspberry Pi 5 setup guide
3. VFAT32 write testing procedures
4. Performance benchmarking results
5. Known issues and workarounds

### Project Management Deliverables
1. Project timeline with milestones
2. Risk assessment and mitigation plan
3. Agent coordination procedures
4. Budget and resource allocation
5. Success criteria validation report

## 10. Next Steps

### Immediate Actions (Next 24 hours)
1. [ ] Björn: Create `pi` branch from `feature-vfat32-driver`
2. [ ] Sofia: Begin Raspberry Pi 5 hardware research
3. [ ] Claude: Prepare hardware budget proposal
4. [ ] Pierre: Research Raspberry Pi 5 availability
5. [ ] Ingrid: Set up ARM64 cross-compilation environment

### Week 1 Milestones
1. [ ] Hardware ordered and arriving
2. [ ] ARM64 cross-compilation working
3. [ ] Basic kernel booting in QEMU ARM64
4. [ ] All agents briefed on their responsibilities
5. [ ] Communication channels established

---

## Appendices

### Appendix A: Required Hardware
- Raspberry Pi 5 (8GB recommended)
- High-quality SD cards (32GB+, Class 10)
- USB serial adapter for debugging
- Power supply (USB-C, 5V/3A minimum)
- Ethernet cable or WiFi adapter
- Optional: HDMI cable for display output

### Appendix B: Reference Projects
1. **Rust Raspberry Pi OS Projects:**
   - `rust-raspberrypi-OS-tutorials`
   - `rpi-os`
   - `aarch64-raspi`

2. **ARM64 Rust Targets:**
   - `aarch64-unknown-none`
   - `aarch64-unknown-none-softfloat`

3. **QEMU ARM64 Documentation:**
   - `qemu-system-aarch64 -M help`
   - ARM Virtual Platform documentation

### Appendix C: Useful Commands
```bash
# Cross-compilation setup
rustup target add aarch64-unknown-none
cargo build --target aarch64-unknown-none

# QEMU ARM64 testing
qemu-system-aarch64 -M virt -cpu cortex-a72 \
  -kernel target/aarch64-unknown-none/debug/kernel \
  -serial stdio

# Raspberry Pi serial console
sudo screen /dev/ttyUSB0 115200
```

### Appendix D: Contact Information
- **Project Lead:** Björn Le Bâtisseur
- **Technical Lead:** Ingrid L'Ingénieure
- **Hardware:** Pierre Le Propriétaire
- **Research:** Sofia La Savante
- **Finance:** Claude Le Comptable

---

*This document will be updated as the porting project progresses. Last updated: February 15, 2026*