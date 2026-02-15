# Week 5 Executive Summary: GUI & Input Subsystems Research

**Completed by:** Sofia La Savante  
**Date:** February 15, 2026  
**Duration:** 1-hour research sprint

---

## Research Objectives

Conduct comprehensive research on Raspberry Pi 5 GUI and Input subsystems to inform the vfat32/webbOS porting effort, focusing on:
- Display output capabilities (HDMI, DSI, Composite)
- Graphics drivers and GPU acceleration
- Input device support
- Display server architecture
- Touchscreen integration
- Performance and thermal considerations

---

## Key Findings

### 1. Graphics Capabilities - EXCELLENT ✅

The Raspberry Pi 5's VideoCore VII GPU provides significant improvements:
- **OpenGL ES 3.1** and **Vulkan 1.3** hardware acceleration
- **Dual 4Kp60 HDMI** outputs (major upgrade from Pi 4)
- **HDR support** for modern displays
- **Fully open-source Mesa drivers** by Igalia

**Implication for webbOS:** Modern graphics APIs available, no proprietary driver concerns.

### 2. Display Output - VERSATILE ✅

| Interface | Status | Use Case |
|-----------|--------|----------|
| HDMI 2.1 | Dual 4K60 | Primary displays |
| DSI | Touch capable | Integrated displays |
| Composite | Via pads | Legacy fallback |

**Implication for webbOS:** Flexible display options for different product configurations.

### 3. Input Support - COMPREHENSIVE ✅

- **USB HID:** Full keyboard, mouse, gamepad support via xHCI
- **GPIO:** 40-pin header with interrupt support
- **I2C/SPI:** Touch controller support (GT911, FT6236)
- **Bluetooth 5.0:** Wireless peripherals

**Implication for webbOS:** All standard input methods available.

### 4. Display Server - MODERN ✅

Raspberry Pi 5 uses **Wayland** (not X11):
- Default compositor: **Labwc** (transitioning from Wayfire)
- Full hardware acceleration via DRM/KMS
- Lower latency than X11
- Better resource management

**Implication for webbOS:** Must ensure Wayland compatibility; X11 is deprecated path.

### 5. Touchscreen Support - STANDARD ✅

- **Capacitive multi-touch:** 5-10 points (controller dependent)
- **Common controllers:** GT911, FT6236 (both supported)
- **Calibration:** Available via tslib/libinput
- **Gestures:** libinput provides standard gestures

**Implication for webbOS:** Touch interfaces fully supported.

### 6. Performance Considerations - MANAGEABLE ⚠️

**Thermal:**
- Soft throttle at 80°C (-100MHz per degree)
- Hard throttle at 85°C (forced idle)
- Active cooling recommended for sustained loads

**Power:**
- Idle: ~3W
- Typical: ~5-7W
- Maximum: ~12W

**Memory:**
- CMA (Contiguous Memory Allocator) replaces fixed GPU split
- 320MB CMA default, adjustable
- Shared between CPU and GPU

**Implication for webbOS:** Thermal management required; power-efficient design beneficial.

---

## Critical Recommendations

### Immediate Actions

1. **Graphics Stack:** Target OpenGL ES 3.1 or Vulkan 1.3
2. **Display Server:** Migrate to Wayland (Labwc compositor)
3. **Input Handling:** Use libinput for unified input management
4. **Thermal Design:** Plan for active cooling in product design

### Technical Decisions

| Decision | Recommendation | Rationale |
|----------|---------------|-----------|
| Graphics API | OpenGL ES 3.1 | Broad compatibility |
| Display Server | Wayland/Labwc | Official Pi 5 support |
| Input Stack | libinput | Modern, unified |
| CMA Size | 256MB | Sufficient for 4K GUI |
| Display Resolution | 1080p60 default | Performance/quality balance |

### Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| Wayland migration complexity | Medium | Plan adequate development time |
| Thermal throttling | Medium | Implement active cooling |
| USB device compatibility | Low | Test with target peripherals |
| DSI touch issues | Low | Use tested display combinations |

---

## Research Artifacts

**Generated Documents:**
1. `Week5_GUI_Input_Subsystems_Research.md` - Full technical report
2. `Week5_Executive_Summary.md` - This document

**Key Resources Identified:**
- Raspberry Pi 5 Product Brief
- Mesa/V3D driver documentation
- Wayland protocol specifications
- libinput developer documentation

---

## Next Steps

### For Ingrid L'Ingénieure (Technical Lead)
1. Review graphics API compatibility with current webbOS code
2. Assess Wayland migration effort
3. Identify any X11 dependencies to refactor

### For Björn Le Bâtisseur (Project Structure)
1. Add GUI/input dependencies to build system
2. Set up cross-compilation for Wayland/EGL
3. Create testing environment configuration

### For Pierre Le Propriétaire (Hardware)
1. Source active cooling solution
2. Select target display (HDMI vs DSI)
3. Test input devices with Pi 5

---

## Conclusion

The Raspberry Pi 5 provides a **strong platform** for the webbOS GUI with modern graphics capabilities, comprehensive input support, and a forward-looking Wayland-based display stack. The primary considerations are:

1. **Wayland migration** is essential (X11 not recommended)
2. **Thermal management** must be addressed
3. **Hardware acceleration** is excellent and should be utilized

**Overall Assessment:** ✅ **GREEN** - Proceed with Pi 5 GUI implementation

---

*Research completed successfully. Full technical details available in the companion document.*
