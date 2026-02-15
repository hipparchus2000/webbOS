# Week 5 Implementation Roadmap: GUI & Input Subsystems

**Architect:** Ingrid L'Ingénieure, Technical Implementation Lead  
**Date:** February 15, 2026  
**Based on:** Sofia La Savante's Research Report  
**Project:** WebbOS Raspberry Pi 5 Porting - Phase 2

---

## 🔧 Executive Summary

This roadmap provides the detailed implementation plan for porting webbOS GUI and Input subsystems to the Raspberry Pi 5 platform. Building on Sofia's comprehensive research, we now have a clear technical path forward.

**Current State:**
- WebbOS x86_64 uses VESA framebuffer (1024x768) and PS/2 input
- Raspberry Pi 5 has VideoCore VII GPU with dual 4K60 HDMI output
- Must migrate from x86-specific to ARM64/RPi5-specific drivers

**Target State:**
- Native VideoCore VII GPU acceleration via DRM/KMS
- Wayland-based display server (Labwc compositor)
- Multi-input support (USB HID, GPIO, Touch, Bluetooth)
- 1080p60 primary target, 4K60 capable

---

## 📋 Implementation Phases

### Phase 1: Foundation (Weeks 5.1-5.2) - Display Driver Core

| Task | Duration | Owner | Dependencies |
|------|----------|-------|--------------|
| DRM/KMS driver abstraction layer | 3 days | Ingrid | Kernel base |
| VC4/V3D Mesa driver integration | 2 days | Ingrid | DRM layer |
| Framebuffer device migration | 2 days | Ingrid | DRM layer |
| CMA memory allocator setup | 1 day | Ingrid | - |

**Milestone 1:** `Basic framebuffer output on Pi 5`

### Phase 2: Graphics Acceleration (Weeks 5.3-5.4) - GPU Integration

| Task | Duration | Owner | Dependencies |
|------|----------|-------|--------------|
| OpenGL ES 3.1 context creation | 2 days | Ingrid | Mesa driver |
| EGL display initialization | 2 days | Ingrid | OpenGL ES |
| Hardware compositor integration | 3 days | Ingrid | EGL |
| GPU memory management (CMA) | 1 day | Ingrid | - |

**Milestone 2:** `Hardware-accelerated rendering functional`

### Phase 3: Display Server (Weeks 5.5-5.6) - Wayland Migration

| Task | Duration | Owner | Dependencies |
|------|----------|-------|--------------|
| Wayland protocol implementation | 3 days | Ingrid | EGL |
| Labwc compositor integration | 2 days | Ingrid | Wayland |
| WebbOS browser Wayland backend | 3 days | Ingrid | Labwc |
| X11 compatibility layer (if needed) | 2 days | Ingrid | - |

**Milestone 3:** `WebbOS desktop running on Wayland`

### Phase 4: Input Subsystem (Weeks 5.7-5.8) - Multi-Input Support

| Task | Duration | Owner | Dependencies |
|------|----------|-------|--------------|
| USB HID driver (keyboard/mouse) | 2 days | Ingrid | USB stack |
| libinput integration | 2 days | Ingrid | HID driver |
| GPIO button support | 2 days | Ingrid | GPIO driver |
| Touchscreen driver (GT911/FT6236) | 2 days | Ingrid | I2C driver |

**Milestone 4:** `Full input device support`

### Phase 5: Touch Integration (Weeks 5.9-5.10) - Touch UI

| Task | Duration | Owner | Dependencies |
|------|----------|-------|--------------|
| Multi-touch protocol support | 2 days | Ingrid | Touch driver |
| Gesture recognition library | 2 days | Ingrid | Multi-touch |
| Touch calibration system | 1 day | Ingrid | - |
| Touch-optimized desktop shell | 2 days | Ingrid | Gesture lib |

**Milestone 5:** `Touch interface fully functional`

### Phase 6: Optimization (Week 5.11) - Performance Tuning

| Task | Duration | Owner | Dependencies |
|------|----------|-------|--------------|
| Thermal monitoring integration | 1 day | Ingrid | - |
| GPU/CPU load balancing | 2 days | Ingrid | All above |
| Memory bandwidth optimization | 1 day | Ingrid | - |
| Frame timing/VSync tuning | 1 day | Ingrid | - |

**Milestone 6:** `Optimized for 60fps at 1080p`

---

## 🏗️ Technical Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    WebbOS Desktop Environment                       │
│              (HTML/CSS/JS - Single File Architecture)               │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Wayland Compositor (Labwc)                      │   │
│  │  - Window management    - Hardware planes                    │   │
│  │  - Damage tracking      - VSync                              │   │
│  └────────────────────┬────────────────────────────────────────┘   │
├───────────────────────┼─────────────────────────────────────────────┤
│  ┌────────────────────▼────────────────────────────────────────┐   │
│  │              EGL / OpenGL ES 3.1                             │   │
│  │  - Context management   - Surface creation                   │   │
│  │  - Buffer swapping      - GPU commands                       │   │
│  └────────────────────┬────────────────────────────────────────┘   │
├───────────────────────┼─────────────────────────────────────────────┤
│  ┌────────────────────▼────────────────────────────────────────┐   │
│  │              Mesa V3D Driver (VideoCore VII)                 │   │
│  │  - GPU command submission   - Shader compilation             │   │
│  │  - Memory allocation        - Performance counters           │   │
│  └────────────────────┬────────────────────────────────────────┘   │
├───────────────────────┼─────────────────────────────────────────────┤
│  ┌────────────────────▼────────────────────────────────────────┐   │
│  │              DRM/KMS (Direct Rendering Manager)              │   │
│  │  - Mode setting           - Framebuffer management           │   │
│  │  - Hardware planes        - Connector/Encoder control        │   │
│  └────────────────────┬────────────────────────────────────────┘   │
├───────────────────────┼─────────────────────────────────────────────┤
│  ┌────────────────────▼────────────────────────────────────────┐   │
│  │              Kernel Display Driver (vc4)                     │   │
│  │  - HDMI 2.1 output        - DSI interface                    │   │
│  │  - CMA memory           - Interrupt handling                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                         Input Subsystem                             │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   USB HID    │  │    GPIO      │  │   Touch      │              │
│  │  (xHCI)      │  │   (RP1)      │  │  (I2C/SPI)   │              │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘              │
├─────────┼─────────────────┼─────────────────┼───────────────────────┤
│  ┌──────▼─────────────────▼─────────────────▼──────────────────┐   │
│  │                    evdev (Kernel)                           │   │
│  │  - Raw event delivery    - Device enumeration               │   │
│  └──────┬──────────────────────────────────────────────────────┘   │
├─────────┼───────────────────────────────────────────────────────────┤
│  ┌──────▼──────────────────────────────────────────────────────┐   │
│  │                    libinput                                  │   │
│  │  - Gesture recognition   - Pointer acceleration             │   │
│  │  - Touch handling        - Device quirks                    │   │
│  └──────┬──────────────────────────────────────────────────────┘   │
├─────────┼───────────────────────────────────────────────────────────┤
│  ┌──────▼──────────────────────────────────────────────────────┐   │
│  │              WebbOS Input Manager                            │   │
│  │  - Event routing         - Input focus                      │   │
│  │  - Key mapping           - Touch gestures                   │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 📝 Detailed Task Specifications

### Task 1: DRM/KMS Abstraction Layer

**Objective:** Create portable abstraction over DRM/KMS for display management

**Implementation Details:**
```rust
// drivers/gpu/drm/drm_device.rs
pub struct DrmDevice {
    fd: FileDescriptor,
    connectors: Vec<Connector>,
    encoders: Vec<Encoder>,
    crtcs: Vec<Crtc>,
    planes: Vec<Plane>,
}

impl DrmDevice {
    pub fn open(card: &str) -> Result<Self>;
    pub fn set_mode(&mut self, connector: &Connector, mode: &Mode) -> Result<()>;
    pub fn page_flip(&mut self, fb: Framebuffer) -> Result<()>;
    pub fn atomic_commit(&mut self, request: &AtomicRequest) -> Result<()>;
}
```

**Configuration:**
```
# /boot/firmware/config.txt additions
dtoverlay=vc4-kms-v3d
dtoverlay=cma-256
drm.debug=0x1e
```

**Acceptance Criteria:**
- [ ] Can enumerate display connectors
- [ ] Can set video modes
- [ ] Can perform page flips without tearing
- [ ] Error handling for hotplug events

---

### Task 2: Mesa V3D Integration

**Objective:** Integrate Mesa V3D driver for GPU acceleration

**Implementation Details:**
```rust
// drivers/gpu/mesa/v3d_context.rs
pub struct V3dContext {
    display: EGLDisplay,
    context: EGLContext,
    surface: EGLSurface,
}

impl V3dContext {
    pub fn initialize(drm_device: &DrmDevice) -> Result<Self>;
    pub fn create_surface(&mut self, width: u32, height: u32) -> Result<EGLSurface>;
    pub fn make_current(&mut self) -> Result<()>;
    pub fn swap_buffers(&mut self) -> Result<()>;
}
```

**Dependencies:**
- Mesa 23.0+ with V3D driver
- libEGL, libGLESv2
- DRM render nodes (`/dev/dri/renderD128`)

**Acceptance Criteria:**
- [ ] EGL display initialization succeeds
- [ ] OpenGL ES 3.1 context created
- [ ] Can render test triangle
- [ ] glmark2-es2-wayland runs successfully

---

### Task 3: Wayland Integration

**Objective:** Port webbOS browser to use Wayland instead of raw framebuffer

**Implementation Details:**
```rust
// desktop/wayland_backend.rs
pub struct WaylandBackend {
    display: WlDisplay,
    registry: WlRegistry,
    compositor: WlCompositor,
    shell: XdgWmBase,
    seat: WlSeat,
}

impl GraphicsBackend for WaylandBackend {
    fn initialize() -> Result<Self>;
    fn create_window(&mut self, width: u32, height: u32) -> Result<Window>;
    fn poll_events(&mut self) -> Vec<InputEvent>;
    fn present(&mut self) -> Result<()>;
}
```

**Compositor Selection:**
- **Primary:** Labwc (lightweight, stable, official Pi 5 support)
- **Alternative:** Wayfire (if 3D effects needed)

**Acceptance Criteria:**
- [ ] WebbOS browser creates Wayland surface
- [ ] Desktop displays correctly at 1080p
- [ ] Window resizes properly
- [ ] Input events received via Wayland

---

### Task 4: USB HID Input

**Objective:** Implement USB HID driver for keyboard/mouse

**Implementation Details:**
```rust
// drivers/input/usb_hid.rs
pub struct UsbHidDriver {
    xhci: XhciController,
    devices: Vec<HidDevice>,
}

pub struct HidDevice {
    device_type: HidType,  // Keyboard, Mouse, Gamepad
    event_queue: VecDeque<InputEvent>,
}

impl InputDriver for UsbHidDriver {
    fn poll(&mut self) -> Vec<InputEvent>;
    fn get_capabilities(&self) -> InputCapabilities;
}
```

**USB Controller:** RP1 xHCI (PCIe connected)

**Acceptance Criteria:**
- [ ] USB keyboard input working
- [ ] USB mouse input working
- [ ] Hotplug detection
- [ ] Multiple device support

---

### Task 5: Touchscreen Driver

**Objective:** Support DSI touchscreens (GT911/FT6236 controllers)

**Implementation Details:**
```rust
// drivers/input/touchscreen.rs
pub struct TouchscreenDriver {
    i2c: I2cBus,
    controller: TouchController,
    calibration: TouchCalibration,
}

pub enum TouchController {
    GoodixGt911 { addr: u8 },
    FocalTechFt6236 { addr: u8 },
}

impl TouchscreenDriver {
    pub fn detect(i2c: &I2cBus) -> Result<Self>;
    pub fn read_touch_points(&mut self) -> Vec<TouchPoint>;
    pub fn calibrate(&mut self, points: &[CalibrationPoint]) -> Result<()>;
}
```

**Device Tree Overlay:**
```dts
/dts-v1/;
/plugin/;

/ {
    compatible = "raspberrypi,5-model-b";
    
    fragment@0 {
        target = <&i2c_csi_dsi>;
        __overlay__ {
            gt911: gt911@5d {
                compatible = "goodix,gt911";
                reg = <0x5d>;
                interrupt-parent = <&gpio>;
                interrupts = <4 2>;
                touchscreen-size-x = <800>;
                touchscreen-size-y = <480>;
            };
        };
    };
};
```

**Acceptance Criteria:**
- [ ] Touch detection working
- [ ] Multi-touch (5+ points) functional
- [ ] Calibration accurate to ±2 pixels
- [ ] Gesture events generated

---

### Task 6: Thermal Management

**Objective:** Monitor and manage thermal constraints

**Implementation Details:**
```rust
// drivers/thermal/pi5_thermal.rs
pub struct ThermalManager {
    temp_sensor: TempSensor,
    gpu_manager: GpuManager,
    throttle_state: ThrottleState,
}

impl ThermalManager {
    pub fn read_temperature(&self) -> f32;
    pub fn get_throttle_status(&self) -> ThrottleStatus;
    pub fn adjust_gpu_frequency(&mut self, freq_mhz: u32) -> Result<()>;
    pub fn should_reduce_quality(&self) -> bool;
}
```

**Thermal Thresholds:**
| Temperature | Action |
|-------------|--------|
| < 70°C | Full performance |
| 70-80°C | Monitor closely |
| 80-85°C | Soft throttle (reduce GPU freq) |
| > 85°C | Hard throttle (emergency) |

**Acceptance Criteria:**
- [ ] Temperature reading accurate
- [ ] Throttle detection working
- [ ] Automatic GPU frequency scaling
- [ ] Performance degradation graceful

---

## 📊 Resource Requirements

### Memory Budget

| Component | Minimum | Recommended | Notes |
|-----------|---------|-------------|-------|
| CMA (GPU) | 128MB | 256MB | Shared CPU/GPU memory |
| Framebuffer (1080p) | 8MB | 16MB | Double/triple buffering |
| Framebuffer (4K) | 32MB | 64MB | Double/triple buffering |
| Mesa/GPU driver | 16MB | 32MB | Shader cache, buffers |
| Wayland compositor | 8MB | 16MB | Labwc overhead |
| WebbOS browser | 32MB | 64MB | Page cache, textures |
| **Total (1080p)** | **192MB** | **384MB** | + System overhead |
| **Total (4K)** | **240MB** | **512MB** | + System overhead |

### CPU Requirements

| Task | CPU Usage | Core | Notes |
|------|-----------|------|-------|
| Display compositor | 5-10% | Any | Hardware accelerated |
| Input polling | 1-2% | Any | Interrupt-driven |
| Browser rendering | 10-30% | 1-2 cores | Depends on content |
| GPU command submission | 2-5% | Any | Efficient batching |
| Thermal monitoring | <1% | Any | Periodic check |

### GPU Requirements

| Resolution | GPU Load | Clock | Notes |
|------------|----------|-------|-------|
| 1080p60 | 30-50% | 400-600MHz | Comfortable margin |
| 1080p60 + effects | 50-70% | 600-800MHz | Still safe |
| 4K30 | 40-60% | 600-800MHz | Reduced refresh OK |
| 4K60 | 70-90% | 800MHz | Monitor thermals |

### Power Budget

| Configuration | Power Draw | PSU Required |
|---------------|------------|--------------|
| Base (idle) | 2.5-3W | 5V 3A (15W) |
| 1080p60 desktop | 5-7W | 5V 3A (15W) |
| 4K60 desktop | 8-10W | 5V 5A (25W) |
| Maximum load | 12-15W | 5V 5A (25W) |

---

## 🔌 Integration Points with Existing WebbOS

### 1. Graphics Backend Abstraction

**Current (x86):**
```rust
// kernel/drivers/graphics/vesa.rs
pub struct VesaFramebuffer {
    base: PhysAddr,
    width: u32,
    height: u32,
    bpp: u32,
}
```

**New (Pi 5):**
```rust
// kernel/drivers/graphics/drm_backend.rs
pub struct DrmGraphicsBackend {
    drm_device: DrmDevice,
    egl_context: V3dContext,
    surface: EGLSurface,
}

impl GraphicsBackend for DrmGraphicsBackend {
    fn init() -> Result<Self>;
    fn present(&mut self, buffer: &[u8]) -> Result<()>;
    fn get_info(&self) -> GraphicsInfo;
}
```

### 2. Input Abstraction

**Current (x86):**
```rust
// kernel/drivers/input/ps2.rs
pub struct Ps2Keyboard { ... }
pub struct Ps2Mouse { ... }
```

**New (Pi 5 - Unified):**
```rust
// kernel/drivers/input/unified.rs
pub struct UnifiedInputManager {
    usb_hid: UsbHidDriver,
    touchscreen: Option<TouchscreenDriver>,
    gpio_buttons: Option<GpioInput>,
    bluetooth: Option<BluetoothHid>,
}

impl InputManager for UnifiedInputManager {
    fn poll_events(&mut self) -> Vec<InputEvent>;
    fn register_callback(&mut self, event_type: EventType, cb: Callback);
}
```

### 3. Browser Rendering Integration

**Changes Required:**
```rust
// browser/rendering/gpu_accelerated.rs
pub enum RenderBackend {
    Software(SoftwareRenderer),     // Current x86
    Gles(GlesRenderer),             // Pi 5 OpenGL ES
    Vulkan(VulkanRenderer),         // Future Pi 5 Vulkan
}

impl RenderBackend {
    pub fn new_auto_detect() -> Result<Self> {
        #[cfg(target_arch = "aarch64")]
        if let Ok(gles) = GlesRenderer::new() {
            return Ok(Self::Gles(gles));
        }
        
        Ok(Self::Software(SoftwareRenderer::new()))
    }
}
```

---

## 🧪 Testing Strategy

### Test Categories

| Category | Tool/Method | Coverage Target |
|----------|-------------|-----------------|
| Unit Tests | Rust `cargo test` | 80% |
| Integration Tests | Custom test harness | 75% |
| Hardware Tests | Real Pi 5 hardware | Critical paths |
| Performance Tests | Custom benchmarks | All graphics paths |
| Thermal Tests | Stress testing | Thermal management |

### Display Testing

```bash
# 1. DRM mode test
modetest -M vc4 -s 32:1920x1080-60

# 2. GL rendering test
glmark2-es2-wayland

# 3. WebbOS specific test
./webbos_test --display-test --resolution 1920x1080

# 4. Multi-display test (if available)
./webbos_test --display-test --dual-display
```

### Input Testing

```bash
# 1. List input devices
libinput list-devices

# 2. Debug input events
libinput debug-events

# 3. Touch testing
evtest /dev/input/event0

# 4. WebbOS input test
./webbos_test --input-test --device /dev/input/event0
```

### Performance Testing

```bash
# 1. Frame rate measurement
./webbos_test --fps-test --duration 60

# 2. Memory usage
./webbos_test --memory-test --track-gpu

# 3. Thermal stress test
stress-ng --cpu 4 --gpu 1 --timeout 600 &
./webbos_test --thermal-test --monitor

# 4. Latency test
./webbos_test --input-latency --samples 1000
```

### Thermal Testing

```bash
# Monitor during stress test
watch -n 1 'vcgencmd measure_temp && vcgencmd get_throttled'

# Expected results:
# - Idle: 45-55°C
# - 1080p60 desktop: 60-70°C
# - 4K60 stress: <80°C (with active cooling)
```

---

## ⚠️ Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Mesa driver issues | Medium | High | Keep software renderer fallback |
| Wayland complexity | Medium | Medium | Start with Weston, migrate to Labwc |
| Thermal throttling | High | Medium | Implement active cooling + quality scaling |
| USB device compatibility | Low | Low | Test with target peripherals early |
| Touch calibration issues | Medium | Low | Include runtime calibration tool |
| Memory pressure | Medium | Medium | Monitor CMA usage, optimize allocations |

---

## 📅 Timeline Summary

```
Week 5.1-5.2:  [████] Display Driver Core
Week 5.3-5.4:  [████] Graphics Acceleration
Week 5.5-5.6:  [████] Wayland Migration
Week 5.7-5.8:  [████] Input Subsystem
Week 5.9-5.10: [████] Touch Integration
Week 5.11:     [█   ] Optimization

Total: 11 weeks (~2.5 months for GUI/Input completion)
```

---

## ✅ Success Criteria

1. **Display:** WebbOS boots to 1080p60 desktop without user intervention
2. **Performance:** 60fps maintained during normal desktop use
3. **Input:** Keyboard, mouse, and touch all functional simultaneously
4. **Thermal:** No throttling during normal use with active cooling
5. **Compatibility:** All existing webbOS apps run without modification
6. **Stability:** 24-hour uptime test passes without crashes

---

## 📝 Notes

- **Funktion före form:** All components must work reliably before optimization
- **Backup strategy:** Keep x86 VESA backend for development/testing
- **Documentation:** Update AGENTS.md with any architecture changes
- **Security:** Input events must be properly sanitized before processing

---

*"Det fungerar perfekt!" - Ingrid L'Ingénieure*

**Next Step:** Begin Phase 1 implementation - DRM/KMS abstraction layer
