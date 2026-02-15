# Technical Architecture: GUI & Input Subsystems

**Document:** Implementation Architecture Specification  
**Architect:** Ingrid L'Ingénieure  
**Date:** February 15, 2026  
**Version:** 1.0

---

## 1. Architecture Overview

### 1.1 Design Philosophy

**Layered Architecture with Clean Abstractions:**
- Hardware abstraction at driver level
- Platform-agnostic APIs for upper layers
- Graceful fallbacks for missing hardware features
- Performance-first design for resource-constrained environment

### 1.2 High-Level Component Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                            User Space                                        │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    WebbOS Browser Engine                              │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐ │   │
│  │  │  HTML/CSS    │  │  JavaScript  │  │       Rendering              │ │   │
│  │  │   Parser     │  │    Engine    │  │    (Skia/Vulkan)             │ │   │
│  │  └──────┬───────┘  └──────┬───────┘  └──────────────┬───────────────┘ │   │
│  │         └─────────────────┴─────────────────────────┘                  │   │
│  │                            │                                           │   │
│  │  ┌─────────────────────────▼──────────────────────────────┐           │   │
│  │  │              Web Content Display Layer                  │           │   │
│  │  │         (Surface allocation, damage tracking)           │           │   │
│  │  └─────────────────────────┬──────────────────────────────┘           │   │
│  └────────────────────────────┼──────────────────────────────────────────┘   │
├───────────────────────────────┼──────────────────────────────────────────────┤
│                               │                    System Libraries          │
│  ┌────────────────────────────▼──────────────────────────────┐               │
│  │                    Wayland Client API                      │               │
│  │  - wl_display      - wl_surface      - wl_touch           │               │
│  │  - wl_compositor   - xdg_surface     - wl_pointer         │               │
│  └────────────────────────────┬──────────────────────────────┘               │
├───────────────────────────────┼──────────────────────────────────────────────┤
│                               ▼                    System Services           │
│  ┌──────────────────────────────────────────────────────────────┐           │
│  │                  Wayland Compositor (Labwc)                   │           │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │           │
│  │  │   Window     │  │   Hardware   │  │     Damage           │ │           │
│  │  │   Manager    │  │   Planes     │  │   Tracking           │ │           │
│  │  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘ │           │
│  │         └─────────────────┴─────────────────────┘             │           │
│  │                           │                                  │           │
│  │  ┌────────────────────────▼──────────────────────────┐       │           │
│  │  │              Rendering Backend (EGL)               │       │           │
│  │  │       (Surface composition, shader effects)        │       │           │
│  │  └────────────────────────┬───────────────────────────┘       │           │
│  └───────────────────────────┼───────────────────────────────────┘           │
├──────────────────────────────┼───────────────────────────────────────────────┤
│                              ▼                     Kernel Space              │
│  ┌──────────────────────────────────────────────────────────────┐           │
│  │                    Mesa GPU Driver (V3D)                      │           │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │           │
│  │  │   Command    │  │   Shader     │  │   Memory             │ │           │
│  │  │   Stream     │  │   Compiler   │  │   Management         │ │           │
│  │  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘ │           │
│  │         └─────────────────┴─────────────────────┘             │           │
│  │                           │                                  │           │
│  │  ┌────────────────────────▼──────────────────────────┐       │           │
│  │  │              DRM/KMS Subsystem                      │       │           │
│  │  │  (Mode setting, framebuffer, page flipping)         │       │           │
│  │  └────────────────────────┬───────────────────────────┘       │           │
│  └───────────────────────────┼───────────────────────────────────┘           │
├──────────────────────────────┼───────────────────────────────────────────────┤
│                              ▼                                               │
│  ┌──────────────────────────────────────────────────────────────┐           │
│  │              VideoCore VII Hardware Interface                 │           │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │           │
│  │  │  V3D     │  │   HVS    │  │   HDMI   │  │    DSI       │  │           │
│  │  │ (GPU)    │  │ (Scaler) │  │  (TX)    │  │  (Display)   │  │           │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────────┘  │           │
│  └──────────────────────────────────────────────────────────────┘           │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│                           Input Subsystem Architecture                       │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Hardware Layer                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │  USB xHCI    │  │  I2C (RP1)   │  │  GPIO (RP1)  │  │  Bluetooth 5.0   │ │
│  │  Controller  │  │  Controller  │  │  Controller  │  │  (SDIO)          │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘ │
│         │                 │                 │                   │           │
├─────────┼─────────────────┼─────────────────┼───────────────────┼───────────┤
│         ▼                 ▼                 ▼                   ▼           │
│  Device Drivers                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │  USB HID     │  │  Touchscreen │  │  GPIO Keys   │  │  BT HID          │ │
│  │  (kbd,mouse) │  │  (GT911,etc) │  │  (buttons)   │  │  (wireless)      │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘ │
│         │                 │                 │                   │           │
├─────────┼─────────────────┼─────────────────┼───────────────────┼───────────┤
│         ▼                 ▼                 ▼                   ▼           │
│  Kernel Subsystem                                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                           evdev (input event device)                  │   │
│  │  ┌────────────────────────────────────────────────────────────────┐  │   │
│  │  │  Event Types: EV_KEY, EV_REL, EV_ABS, EV_SYN                   │  │   │
│  │  │  Protocols: MT (multi-touch), MSC (miscellaneous)               │  │   │
│  │  └────────────────────────────────────────────────────────────────┘  │   │
│  └────────────────────────────────┬─────────────────────────────────────┘   │
│                                   │                                          │
├───────────────────────────────────┼──────────────────────────────────────────┤
│                                   ▼         Userspace Input Libraries        │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                           libinput                                    │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐ │   │
│  │  │   Pointer    │  │   Keyboard   │  │        Touch                 │ │   │
│  │  │  Accel/Gest  │  │   Mapping    │  │    Gestures/Multitouch       │ │   │
│  │  └──────┬───────┘  └──────┬───────┘  └──────────────┬───────────────┘ │   │
│  │         └─────────────────┴─────────────────────────┘                 │   │
│  └────────────────────────────────┬─────────────────────────────────────┘   │
│                                   │                                          │
├───────────────────────────────────┼──────────────────────────────────────────┤
│                                   ▼         WebbOS Integration               │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    WebbOS Input Manager                               │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐ │   │
│  │  │   Event      │  │   Focus      │  │    Gesture                   │ │   │
│  │  │   Routing    │  │   Management │  │    Recognition               │ │   │
│  │  └──────────────┘  └──────────────┘  └──────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Display Subsystem Deep Dive

### 2.1 DRM/KMS Architecture

**Direct Rendering Manager (DRM)** is the kernel subsystem for GPU management:

```rust
// kernel/drivers/drm/mod.rs

/// DRM Device represents a GPU
pub struct DrmDevice {
    /// File descriptor for ioctl operations
    fd: RawFd,
    
    /// Available connectors (HDMI, DSI, etc.)
    connectors: Vec<DrmConnector>,
    
    /// CRTCs (CRT Controllers - display pipelines)
    crtcs: Vec<DrmCrtc>,
    
    /// Hardware planes for composition
    planes: Vec<DrmPlane>,
    
    /// Encoder for signal conversion
    encoders: Vec<DrmEncoder>,
}

impl DrmDevice {
    /// Open DRM device node
    pub fn open(path: &str) -> Result<Self> {
        let fd = open(path, O_RDWR | O_CLOEXEC)?;
        // ... probe capabilities
    }
    
    /// Atomic mode setting - all properties set atomically
    pub fn atomic_commit(&self, req: &AtomicRequest) -> Result<()> {
        // DRM_IOCTL_ATOMIC_COMMIT
        // Non-blocking with EVENT flag for page flip completion
    }
    
    /// Wait for page flip completion
    pub fn wait_vblank(&self) -> Result<()> {
        // DRM_IOCTL_WAIT_VBLANK
    }
}
```

**Key DRM Concepts for Pi 5:**

| Component | Pi 5 Implementation | Purpose |
|-----------|---------------------|---------|
| Connector | HDMI-A-1, HDMI-A-2, DSI-1 | Physical display connection |
| CRTC | 2 available | Display pipeline (scanout) |
| Plane | Primary + Overlay + Cursor | Hardware composition layers |
| Encoder | HDMI/TMDS, DSI | Signal format conversion |
| Framebuffer | GEM objects | GPU memory for display |

### 2.2 VideoCore VII Integration

**V3D Block (GPU Compute):**
```rust
// kernel/drivers/gpu/v3d/mod.rs

/// V3D GPU context for command submission
pub struct V3dContext {
    /// DRM render node
    render_fd: RawFd,
    
    /// GPU job manager
    job_manager: V3dJobManager,
    
    /// Shader compiler cache
    shader_cache: LruCache<u64, ShaderBinary>,
}

impl V3dContext {
    /// Submit rendering commands to GPU
    pub fn submit_job(&mut self, job: V3dJob) -> Result<JobHandle> {
        // 1. Validate job structure
        // 2. Allocate GPU memory for BOs (Buffer Objects)
        // 3. Submit to V3D kernel driver
        // 4. Return handle for completion tracking
    }
    
    /// Create GEM buffer for texture/render target
    pub fn create_bo(&self, size: usize) -> Result<GemHandle> {
        // DRM_IOCTL_MODE_CREATE_DUMB or V3D-specific
    }
}
```

**HVS (Hardware Video Scaler):**
- Composites planes together
- Handles scaling and format conversion
- Part of display pipeline, not 3D rendering

### 2.3 Memory Management (CMA)

**Contiguous Memory Allocator (CMA) is critical for Pi 5:**

```rust
// kernel/mm/cma.rs

/// CMA memory pool for GPU buffers
pub struct CmaAllocator {
    /// Total CMA size (configured via dtoverlay)
    total_size: usize,
    
    /// Available memory tracking
    free_regions: BTreeSet<MemoryRegion>,
    
    /// Allocated buffer tracking
    allocations: HashMap<GemHandle, Allocation>,
}

impl CmaAllocator {
    /// Allocate contiguous memory for GPU
    pub fn allocate(&mut self, size: usize, align: usize) -> Result<GemHandle> {
        // Find suitable free region
        // Return GEM handle for DRM operations
    }
    
    /// Map GEM buffer for CPU access
    pub fn map_buffer(&self, handle: GemHandle) -> Result<*mut u8> {
        // DRM_IOCTL_MODE_MAP_DUMB
        // mmap() for CPU access
    }
}
```

**CMA Configuration:**
```
# /boot/firmware/config.txt
dtoverlay=vc4-kms-v3d,cma-256

# Or via kernel command line:
cma=256M
```

---

## 3. Graphics Acceleration Pipeline

### 3.1 OpenGL ES 3.1 Context

**EGL Initialization:**
```rust
// drivers/gpu/egl/context.rs

pub struct EglDisplay {
    native_display: NativeDisplayType,
    egl_display: EGLDisplay,
    egl_version: (i32, i32),
}

impl EglDisplay {
    pub fn initialize_drm(drm_fd: RawFd) -> Result<Self> {
        // 1. Get EGL device from DRM fd
        let device = eglGetPlatformDisplayEXT(
            EGL_PLATFORM_DEVICE_EXT,
            drm_device as *mut _,
            std::ptr::null()
        );
        
        // 2. Initialize EGL
        let mut major = 0;
        let mut minor = 0;
        eglInitialize(display, &mut major, &mut minor);
        
        // 3. Bind OpenGL ES API
        eglBindAPI(EGL_OPENGL_ES_API);
        
        // 4. Choose config with required attributes
        let attribs = [
            EGL_SURFACE_TYPE, EGL_WINDOW_BIT,
            EGL_RENDERABLE_TYPE, EGL_OPENGL_ES3_BIT,
            EGL_RED_SIZE, 8,
            EGL_GREEN_SIZE, 8,
            EGL_BLUE_SIZE, 8,
            EGL_ALPHA_SIZE, 8,
            EGL_NONE
        ];
        
        Ok(Self { /* ... */ })
    }
}
```

**OpenGL ES Context Attributes:**
```rust
// Required for webbOS browser rendering
let context_attribs = [
    EGL_CONTEXT_CLIENT_VERSION, 3,  // OpenGL ES 3.x
    EGL_CONTEXT_MINOR_VERSION, 1,   // OpenGL ES 3.1
    EGL_CONTEXT_FLAGS_KHR, EGL_CONTEXT_OPENGL_DEBUG_BIT_KHR,
    EGL_NONE
];
```

### 3.2 Wayland Surface Management

**Wayland EGL Integration:**
```rust
// desktop/wayland/egl_surface.rs

pub struct WaylandEglSurface {
    /// Wayland surface
    surface: WlSurface,
    
    /// XDG toplevel window
    xdg_surface: XdgToplevel,
    
    /// EGL window surface
    egl_surface: EGLSurface,
    
    /// Surface dimensions
    width: u32,
    height: u32,
    
    /// Damage region tracker
    damage: RegionTracker,
}

impl WaylandEglSurface {
    /// Create EGL surface bound to Wayland
    pub fn create(
        display: &EglDisplay,
        wl_display: &WlDisplay,
        width: u32,
        height: u32
    ) -> Result<Self> {
        // 1. Create wl_surface
        // 2. Create xdg_surface/xdg_toplevel
        // 3. Create EGL window surface
        let native_window = wl_egl_window_create(surface, width, height);
        let egl_surface = eglCreateWindowSurface(
            display.egl_display,
            config,
            native_window,
            std::ptr::null()
        );
    }
    
    /// Present with damage tracking
    pub fn present(&mut self, damage_rects: &[Rect]) -> Result<()> {
        // 1. Set damage region (for partial redraws)
        for rect in damage_rects {
            wl_surface_damage(self.surface, rect.x, rect.y, rect.w, rect.h);
        }
        
        // 2. Attach buffer and commit
        eglSwapBuffers(self.display, self.egl_surface);
        wl_surface_commit(self.surface);
    }
}
```

### 3.3 Rendering Pipeline Integration

**WebbOS Browser → GPU:**
```
┌─────────────────────────────────────────────────────────────────┐
│                    Rendering Pipeline Flow                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Content Update                                              │
│     ┌─────────┐    ┌─────────┐    ┌─────────┐                  │
│     │  HTML   │───▶│   CSS   │───▶│ Layout  │                  │
│     │ Change  │    │  Style  │    │  Calc   │                  │
│     └─────────┘    └─────────┘    └────┬────┘                  │
│                                         │                       │
│  2. Paint/Draw                          ▼                       │
│     ┌─────────┐    ┌─────────┐    ┌─────────┐                  │
│     │  Skia   │◀───│ Display │◀───│ Render  │                  │
│     │ Record  │    │  List   │    │  Tree   │                  │
│     └────┬────┘    └─────────┘    └─────────┘                  │
│          │                                                      │
│  3. GPU Execution                     ▼                         │
│     ┌─────────┐    ┌─────────┐    ┌─────────┐                  │
│     │  V3D    │◀───│  Shader │◀───│ Command │                  │
│     │ Execute │    │  Exec   │    │  Buffer │                  │
│     └────┬────┘    └─────────┘    └─────────┘                  │
│          │                                                      │
│  4. Display Output                    ▼                         │
│     ┌─────────┐    ┌─────────┐    ┌─────────┐                  │
│     │  KMS    │◀───│   HVS   │◀───│  Scan   │                  │
│     │ Commit  │    │ Compose │    │  Out    │                  │
│     └────┬────┘    └─────────┘    └─────────┘                  │
│          │                                                      │
│          ▼                                                      │
│     [Display Output]                                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Input Subsystem Deep Dive

### 4.1 evdev Architecture

**Linux Input Event Device:**
```rust
// drivers/input/evdev.rs

/// Event device for kernel input
pub struct EvdevDevice {
    fd: RawFd,
    name: String,
    capabilities: EvdevCapabilities,
    event_queue: VecDeque<InputEvent>,
}

#[repr(C)]
pub struct InputEvent {
    /// Event timestamp
    time: Timeval,
    /// Event type (EV_KEY, EV_REL, EV_ABS, etc.)
    type_: u16,
    /// Event code (KEY_A, REL_X, ABS_MT_POSITION_X, etc.)
    code: u16,
    /// Event value (0/1 for keys, delta for relative, position for absolute)
    value: i32,
}

impl EvdevDevice {
    /// Open input device
    pub fn open(path: &str) -> Result<Self> {
        // EVIOCGNAME, EVIOCGBIT for capabilities
    }
    
    /// Read pending events
    pub fn read_events(&mut self) -> Result<Vec<InputEvent>> {
        // read() in O_NONBLOCK mode
        // Parse input_event structures
    }
    
    /// Grab exclusive access
    pub fn grab(&mut self, grab: bool) -> Result<()> {
        // EVIOCGRAB ioctl
    }
}
```

**Event Types for Pi 5 Input:**

| Type | Code Example | Usage |
|------|--------------|-------|
| EV_KEY | KEY_A, BTN_LEFT | Keyboard, mouse buttons |
| EV_REL | REL_X, REL_Y | Mouse movement |
| EV_ABS | ABS_X, ABS_Y | Touchscreen, joysticks |
| EV_ABS | ABS_MT_SLOT, ABS_MT_POSITION_X | Multi-touch |
| EV_SYN | SYN_REPORT | Event batch delimiter |
| EV_MSC | MSC_TIMESTAMP | High-res timestamps |

### 4.2 USB HID Implementation

**xHCI Controller (RP1):**
```rust
// drivers/usb/xhci/mod.rs

pub struct XhciController {
    /// MMIO base address
    mmio_base: VirtAddr,
    
    /// Command ring for device commands
    cmd_ring: CommandRing,
    
    /// Event ring for completion events
    event_ring: EventRing,
    
    /// Transfer rings for each endpoint
    transfer_rings: Vec<TransferRing>,
    
    /// HID devices attached
    hid_devices: Vec<UsbHidDevice>,
}

impl XhciController {
    /// Initialize xHCI controller
    pub fn init(&mut self) -> Result<()> {
        // 1. Reset controller
        // 2. Set up operational registers
        // 3. Initialize device slots
        // 4. Start command ring
        // 5. Enable interrupts
    }
    
    /// Poll for HID events
    pub fn poll_hid_events(&mut self) -> Vec<HidEvent> {
        // Process event ring
        // Parse HID reports
        // Convert to generic events
    }
}
```

**HID Report Parser:**
```rust
// drivers/usb/hid_report.rs

pub struct HidReportParser {
    report_descriptor: Vec<u8>,
    report_size: usize,
}

impl HidReportParser {
    /// Parse raw HID report into events
    pub fn parse(&self, report: &[u8]) -> Vec<HidEvent> {
        // Parse report descriptor
        // Extract key states, axis values
        // Generate HidEvent list
    }
}

pub enum HidEvent {
    Key { keycode: u16, pressed: bool },
    MouseMove { x: i16, y: i16 },
    MouseButton { button: u8, pressed: bool },
    GamepadAxis { axis: u8, value: i16 },
}
```

### 4.3 Touchscreen Driver

**Goodix GT911 (Common DSI Touch):**
```rust
// drivers/input/touchscreen/gt911.rs

pub struct Gt911Driver {
    i2c: I2cBus,
    address: u8,
    irq_pin: GpioPin,
    
    /// Current touch points
    touch_points: [Option<TouchPoint>; 5],
    
    /// Configuration loaded from chip
    config: Gt911Config,
}

#[repr(C, packed)]
struct Gt911Point {
    track_id: u8,
    x_low: u8,
    x_high: u8,
    y_low: u8,
    y_high: u8,
    size_low: u8,
    size_high: u8,
    reserved: u8,
}

impl Gt911Driver {
    /// Initialize GT911 chip
    pub fn init(&mut self) -> Result<()> {
        // 1. Reset chip via INT pin sequence
        // 2. Read configuration
        // 3. Set resolution
        // 4. Enable multi-touch
        // 5. Configure interrupt
    }
    
    /// Read touch points from chip
    pub fn read_touches(&mut self) -> Result<Vec<TouchPoint>> {
        // Read 0x814E (touch status)
        // Read point data from 0x814F
        // Parse up to 5 points
        // Clear buffer by writing 0 to 0x814E
    }
}
```

**Multi-Touch Protocol (MT-B):**
```rust
// drivers/input/mt_protocol.rs

/// Convert driver touches to evdev events
pub fn touches_to_events(
    touches: &[TouchPoint],
    max_slots: usize
) -> Vec<InputEvent> {
    let mut events = Vec::new();
    
    for (slot, touch) in touches.iter().enumerate().take(max_slots) {
        // ABS_MT_SLOT - select slot
        events.push(InputEvent::abs(ABS_MT_SLOT, slot as i32));
        
        if let Some(t) = touch {
            // ABS_MT_TRACKING_ID - unique touch ID
            events.push(InputEvent::abs(ABS_MT_TRACKING_ID, t.id));
            // ABS_MT_POSITION_X - X coordinate
            events.push(InputEvent::abs(ABS_MT_POSITION_X, t.x));
            // ABS_MT_POSITION_Y - Y coordinate
            events.push(InputEvent::abs(ABS_MT_POSITION_Y, t.y));
            // ABS_MT_PRESSURE - pressure (if available)
            events.push(InputEvent::abs(ABS_MT_PRESSURE, t.pressure));
        } else {
            // Release slot
            events.push(InputEvent::abs(ABS_MT_TRACKING_ID, -1));
        }
    }
    
    // SYN_REPORT - end of event batch
    events.push(InputEvent::syn(SYN_REPORT));
    
    events
}
```

### 4.4 libinput Integration

**Unified Input Library:**
```rust
// input/libinput_wrapper.rs

pub struct LibinputContext {
    udev: UdevContext,
    libinput: Libinput,
    seat: String,
}

impl LibinputContext {
    /// Create libinput context
    pub fn new_udev(seat: &str) -> Result<Self> {
        // libinput_udev_create_context
        // Assign seat
        // Resume device discovery
    }
    
    /// Dispatch and process events
    pub fn dispatch(&mut self) -> Result<Vec<LibinputEvent>> {
        // libinput_dispatch
        // Get events from queue
        // Convert to webbOS events
    }
}

pub enum LibinputEvent {
    PointerMotion { x: f64, y: f64 },
    PointerButton { button: u32, pressed: bool },
    KeyboardKey { key: u32, pressed: bool },
    TouchDown { id: i32, x: f64, y: f64 },
    TouchMove { id: i32, x: f64, y: f64 },
    TouchUp { id: i32 },
    GestureSwipeBegin { fingers: u32 },
    GestureSwipeEnd { cancelled: bool },
}
```

---

## 5. Performance Optimization Strategy

### 5.1 Rendering Optimization

**GPU Rendering Pipeline:**
```rust
// optimization/gpu_pipeline.rs

pub struct OptimizedRenderer {
    /// Reuse command buffers
    command_pool: CommandPool,
    
    /// Batch draw calls
    batcher: DrawBatcher,
    
    /// Texture atlas for UI elements
    atlas: TextureAtlas,
    
    /// GPU memory pools
    buffer_pool: BufferPool,
}

impl OptimizedRenderer {
    /// Optimize display list for GPU
    pub fn optimize(&mut self, display_list: DisplayList) -> GpuCommandBuffer {
        // 1. Sort by shader/texture (minimize state changes)
        // 2. Batch non-overlapping draws
        // 3. Upload to GPU buffers
        // 4. Record command buffer
    }
    
    /// Use hardware planes for video/overlays
    pub fn assign_planes(
        &self,
        layers: &[Layer]
    ) -> Vec<PlaneAssignment> {
        // Primary plane: main content
        // Overlay plane: video
        // Cursor plane: mouse pointer
    }
}
```

### 5.2 Memory Optimization

**CMA Usage Strategy:**
```rust
// optimization/cma_strategy.rs

pub struct CmaOptimizer {
    /// Track allocation patterns
    alloc_tracker: AllocationTracker,
    
    /// Pre-allocate common sizes
    pools: HashMap<usize, MemoryPool>,
}

impl CmaOptimizer {
    /// Allocate with reuse
    pub fn allocate_optimized(&mut self, size: usize) -> GemHandle {
        // Check pool for available buffer
        // If not, allocate from CMA
        // Track for later pooling
    }
    
    /// Defragment CMA periodically
    pub fn defragment(&mut self) {
        // Identify movable allocations
        // Compact memory
        // Update GPU mappings
    }
}
```

**Memory Budget by Use Case:**

| Use Case | CMA Size | Framebuffer | GPU Working | Notes |
|----------|----------|-------------|-------------|-------|
| 1080p Desktop | 256MB | 16MB | 64MB | Comfortable |
| 1080p Video | 256MB | 16MB | 128MB | + Video decode |
| 4K Desktop | 320MB | 48MB | 128MB | Monitor usage |
| 4K Video | 512MB | 48MB | 256MB | Maximum config |

### 5.3 Thermal Management

**Dynamic Performance Scaling:**
```rust
// optimization/thermal_manager.rs

pub struct ThermalGovernor {
    temp_sensor: TempSensor,
    gpu_manager: GpuManager,
    cpu_manager: CpuManager,
    
    current_policy: ThermalPolicy,
}

#[derive(Clone, Copy)]
pub enum ThermalPolicy {
    Performance,    // Full speed until 80°C
    Balanced,       // Scale at 75°C
    Powersave,      // Conservative scaling
    Emergency,      // Maximum throttling
}

impl ThermalGovernor {
    /// Monitor and adjust
    pub fn tick(&mut self) {
        let temp = self.temp_sensor.read();
        
        match temp {
            t if t < 70.0 => self.set_policy(ThermalPolicy::Performance),
            t if t < 80.0 => self.set_policy(ThermalPolicy::Balanced),
            t if t < 85.0 => self.set_policy(ThermalPolicy::Powersave),
            _ => self.set_policy(ThermalPolicy::Emergency),
        }
    }
    
    fn set_policy(&mut self, policy: ThermalPolicy) {
        match policy {
            ThermalPolicy::Performance => {
                self.gpu_manager.set_freq(800);
                self.enable_full_rendering();
            }
            ThermalPolicy::Balanced => {
                self.gpu_manager.set_freq(600);
                self.enable_full_rendering();
            }
            ThermalPolicy::Powersave => {
                self.gpu_manager.set_freq(400);
                self.reduce_frame_rate(30);
            }
            ThermalPolicy::Emergency => {
                self.gpu_manager.set_freq(200);
                self.reduce_quality();
            }
        }
    }
}
```

---

## 6. Integration with WebbOS Browser

### 6.1 Graphics Backend Selection

**Compile-Time Backend Selection:**
```rust
// browser/rendering/backend.rs

pub trait GraphicsBackend: Send + Sync {
    fn initialize(&mut self) -> Result<()>;
    fn create_surface(&mut self, size: Size) -> Result<Box<dyn Surface>>;
    fn present(&mut self) -> Result<()>;
    fn resize(&mut self, size: Size) -> Result<()>;
}

/// Backend factory
pub fn create_backend() -> Result<Box<dyn GraphicsBackend>> {
    #[cfg(all(target_arch = "aarch64", target_feature = "pi5"))]
    {
        // Try Pi 5 accelerated path first
        if let Ok(backend) = DrmEglBackend::new() {
            return Ok(Box::new(backend));
        }
        
        // Fall back to software
        warn!("GPU acceleration unavailable, using software renderer");
        Ok(Box::new(SoftwareBackend::new()))
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        Ok(Box::new(VesaBackend::new()))
    }
}
```

### 6.2 Input Event Mapping

**Unified Input Mapping:**
```rust
// browser/input/mapping.rs

pub struct InputMapper {
    /// Map physical keys to browser keys
    key_map: HashMap<u32, WebKey>,
    
    /// Touch to mouse emulation
    touch_emulation: bool,
    
    /// Gesture recognizers
    gestures: GestureRecognizerSet,
}

impl InputMapper {
    /// Convert system event to browser event
    pub fn map_event(&mut self, event: SystemEvent) -> Option<WebEvent> {
        match event {
            SystemEvent::Keyboard { keycode, pressed } => {
                self.key_map.get(&keycode)
                    .map(|&k| WebEvent::Key { key: k, pressed })
            }
            SystemEvent::PointerMove { x, y } => {
                Some(WebEvent::MouseMove { x, y })
            }
            SystemEvent::Touch { id, phase, x, y } => {
                // Update gesture recognizers
                let gesture = self.gestures.process(id, phase, x, y);
                
                // Generate appropriate event
                match gesture {
                    Gesture::Tap => Some(WebEvent::Click { x, y }),
                    Gesture::Pan { dx, dy } => Some(WebEvent::Scroll { dx, dy }),
                    Gesture::Pinch { scale } => Some(WebEvent::Zoom { scale }),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
```

---

## 7. Build System Integration

### 7.1 Feature Flags

```toml
# kernel/Cargo.toml
[features]
default = ["x86_64"]

# Platform selection
x86_64 = ["vesa-display", "ps2-input"]
rpi5 = ["drm-display", "wayland-backend", "usb-hid", "touch-input"]

# Display backends
vesa-display = []
drm-display = ["mesa-v3d", "gbm"]

# Input backends
ps2-input = []
usb-hid = ["libusb"]
touch-input = ["i2c-gpio"]

# Graphics APIs
mesa-v3d = []
gbm = []  # Generic Buffer Management
```

### 7.2 Cross-Compilation Setup

```rust
// build.rs - Platform detection

fn main() {
    let target = env::var("TARGET").unwrap();
    
    match target.as_str() {
        "aarch64-unknown-none" => {
            // Pi 5 target
            println!("cargo:rustc-cfg=platform=\"rpi5\"");
            
            // Link Mesa libraries
            println!("cargo:rustc-link-lib=EGL");
            println!("cargo:rustc-link-lib=GLESv2");
            println!("cargo:rustc-link-lib=drm");
            println!("cargo:rustc-link-lib=gbm");
        }
        "x86_64-unknown-none" => {
            // x86 target
            println!("cargo:rustc-cfg=platform=\"x86_64\"");
        }
        _ => {}
    }
}
```

---

## 8. Testing Architecture

### 8.1 Unit Test Structure

```rust
// tests/drm_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_drm_mode_selection() {
        let modes = vec![
            Mode { width: 1920, height: 1080, refresh: 60 },
            Mode { width: 3840, height: 2160, refresh: 30 },
            Mode { width: 1280, height: 720, refresh: 60 },
        ];
        
        let selected = select_optimal_mode(&modes);
        assert_eq!(selected.width, 1920);
        assert_eq!(selected.height, 1080);
    }
    
    #[test]
    fn test_touch_coordinate_transform() {
        let calibration = TouchCalibration {
            min_x: 0, max_x: 800,
            min_y: 0, max_y: 480,
            screen_width: 1920,
            screen_height: 1080,
        };
        
        let (sx, sy) = calibration.transform(400, 240);
        assert_eq!(sx, 960);
        assert_eq!(sy, 540);
    }
}
```

### 8.2 Integration Test Harness

```rust
// tests/integration/display_test.rs

#[test]
fn test_full_display_pipeline() {
    // Initialize DRM
    let drm = DrmDevice::open("/dev/dri/card0").unwrap();
    
    // Find connector
    let connector = drm.find_connected_connector().unwrap();
    
    // Set mode
    let mode = connector.preferred_mode();
    drm.set_mode(&connector, &mode).unwrap();
    
    // Create framebuffer
    let fb = drm.create_framebuffer(1920, 1080).unwrap();
    
    // Page flip
    drm.page_flip(&fb).unwrap();
    
    // Verify display
    thread::sleep(Duration::from_millis(100));
    assert!(drm.is_display_active());
}
```

---

## 9. Documentation References

### 9.1 Kernel Documentation
- DRM/KMS: `Documentation/gpu/drm-kms.rst`
- VC4 driver: `drivers/gpu/drm/vc4/`
- Input subsystem: `Documentation/input/`

### 9.2 Mesa Documentation
- V3D driver: `src/gallium/drivers/v3d/`
- EGL: `docs/egl.html`
- OpenGL ES: `docs/opengles.html`

### 9.3 Wayland Documentation
- Core protocol: `wayland-protocols/`
- Labwc: `https://github.com/labwc/labwc/wiki`
- Weston: `wayland.freedesktop.org`

---

## 10. Summary

This architecture provides:

1. **Clean abstraction layers** for hardware independence
2. **Efficient GPU utilization** via DRM/KMS and Mesa
3. **Modern display stack** with Wayland
4. **Comprehensive input support** for all Pi 5 input methods
5. **Performance optimization** strategies for thermal and memory constraints
6. **Thorough testing** at all layers

**Next Steps:**
1. Implement DRM abstraction layer
2. Integrate Mesa EGL context
3. Port browser to Wayland
4. Add multi-input support
5. Optimize for thermal constraints

---

*"Robust och pålitlig - det är så vi bygger system."* - Ingrid L'Ingénieure
