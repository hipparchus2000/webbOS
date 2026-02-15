# Integration Points with Existing WebbOS GUI

**Document:** Integration Architecture Specification  
**Architect:** Ingrid L'Ingénieure  
**Date:** February 15, 2026

---

## 1. Integration Overview

### 1.1 Current WebbOS Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Current WebbOS (x86_64)                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  Desktop Environment                     │   │
│  │              (HTML/CSS/JS - Single File)                │   │
│  └─────────────────────────┬───────────────────────────────┘   │
│                            │                                    │
│  ┌─────────────────────────▼───────────────────────────────┐   │
│  │              WebbOS Browser Engine                       │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────────┐ │   │
│  │  │  HTML   │  │   CSS   │  │   JS    │  │  Rendering │ │   │
│  │  │ Parser  │  │ Parser  │  │ Engine  │  │   (CPU)    │ │   │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  └─────┬──────┘ │   │
│  │       └─────────────┴─────────────┴─────────────┘        │   │
│  │                     │                                     │   │
│  │  ┌──────────────────▼──────────────────┐                 │   │
│  │  │         VESA Framebuffer            │                 │   │
│  │  │    1024x768 @ 32-bit, CPU blit      │                 │   │
│  │  └──────────────────┬──────────────────┘                 │   │
│  └─────────────────────┼────────────────────────────────────┘   │
├────────────────────────┼────────────────────────────────────────┤
│  ┌─────────────────────▼─────────────────────┐                  │
│  │              PS/2 Input                   │                  │
│  │         Keyboard + Mouse                  │                  │
│  └───────────────────────────────────────────┘                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Target WebbOS Architecture (Pi 5)

```
┌─────────────────────────────────────────────────────────────────┐
│                    WebbOS (Raspberry Pi 5)                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  Desktop Environment                     │   │
│  │              (HTML/CSS/JS - Unchanged!)                 │   │
│  └─────────────────────────┬───────────────────────────────┘   │
│                            │                                    │
│  ┌─────────────────────────▼───────────────────────────────┐   │
│  │              WebbOS Browser Engine                       │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────────┐ │   │
│  │  │  HTML   │  │   CSS   │  │   JS    │  │  Rendering │ │   │
│  │  │ Parser  │  │ Parser  │  │ Engine  │  │  (GPU!)    │ │   │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  └─────┬──────┘ │   │
│  │       └─────────────┴─────────────┴─────────────┘        │   │
│  │                     │                                     │   │
│  │  ┌──────────────────▼──────────────────┐                 │   │
│  │  │        Wayland EGL Surface          │                 │   │
│  │  │   Hardware accelerated rendering    │                 │   │
│  │  └──────────────────┬──────────────────┘                 │   │
│  └─────────────────────┼────────────────────────────────────┘   │
├────────────────────────┼────────────────────────────────────────┤
│  ┌─────────────────────▼─────────────────────┐                  │
│  │           Wayland Compositor              │                  │
│  │              (Labwc)                      │                  │
│  └─────────────────────┬─────────────────────┘                  │
├────────────────────────┼────────────────────────────────────────┤
│  ┌─────────────────────▼─────────────────────┐                  │
│  │          DRM/KMS + Mesa EGL               │                  │
│  │     VideoCore VII GPU acceleration        │                  │
│  └─────────────────────┬─────────────────────┘                  │
├────────────────────────┼────────────────────────────────────────┤
│  ┌─────────────────────▼─────────────────────┐                  │
│  │        Unified Input Manager              │                  │
│  │  USB HID + Touch + GPIO + Bluetooth       │                  │
│  └───────────────────────────────────────────┘                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Abstraction Layers

### 2.1 Graphics Backend Abstraction

**Design Goal:** Single codebase supports both x86 VESA and Pi 5 DRM

```rust
// kernel/drivers/graphics/mod.rs

/// Platform-agnostic graphics backend trait
pub trait GraphicsBackend: Send + Sync {
    /// Initialize the backend
    fn initialize(&mut self) -> Result<()>;
    
    /// Get display capabilities
    fn get_capabilities(&self) -> DisplayCapabilities;
    
    /// Create a drawing surface
    fn create_surface(&mut self, config: SurfaceConfig) -> Result<Box<dyn Surface>>;
    
    /// Present current frame
    fn present(&mut self) -> Result<()>;
    
    /// Handle display mode changes (hotplug, etc.)
    fn handle_events(&mut self) -> Vec<DisplayEvent>;
}

/// Surface for drawing operations
pub trait Surface: Send + Sync {
    /// Get raw buffer for software rendering
    fn lock_buffer(&mut self) -> Result<&mut [u8]>;
    
    /// Unlock and mark as dirty
    fn unlock_buffer(&mut self, dirty_rects: &[Rect]);
    
    /// Get native handle for GPU rendering
    fn get_native_handle(&self) -> NativeSurfaceHandle;
    
    /// Resize surface
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    
    /// Get dimensions
    fn size(&self) -> (u32, u32);
}

/// Backend factory - compile-time or runtime selection
pub fn create_backend() -> Result<Box<dyn GraphicsBackend>> {
    #[cfg(feature = "vesa")]
    {
        Ok(Box::new(VesaBackend::new()))
    }
    
    #[cfg(feature = "drm")]
    {
        Ok(Box::new(DrmBackend::new()?))
    }
    
    #[cfg(not(any(feature = "vesa", feature = "drm")))]
    compile_error!("Must enable one graphics backend feature");
}
```

### 2.2 Input Backend Abstraction

```rust
// kernel/drivers/input/mod.rs

/// Unified input device interface
pub trait InputBackend: Send + Sync {
    /// Poll for new input events (non-blocking)
    fn poll_events(&mut self) -> Vec<InputEvent>;
    
    /// Wait for and return next event (blocking)
    fn wait_event(&mut self, timeout: Duration) -> Option<InputEvent>;
    
    /// Get device capabilities
    fn get_capabilities(&self) -> InputCapabilities;
    
    /// Set key repeat rate
    fn set_key_repeat(&mut self, delay_ms: u32, rate_hz: u32);
    
    /// Set pointer acceleration
    fn set_pointer_accel(&mut self, accel: f32);
}

/// Factory for platform-specific input backends
pub fn create_input_backends() -> Vec<Box<dyn InputBackend>> {
    let mut backends = Vec::new();
    
    #[cfg(feature = "ps2")]
    {
        backends.push(Box::new(Ps2Input::new()) as Box<dyn InputBackend>);
    }
    
    #[cfg(feature = "usb_hid")]
    {
        backends.push(Box::new(UsbHidInput::new()) as Box<dyn InputBackend>);
    }
    
    #[cfg(feature = "touchscreen")]
    {
        if let Ok(touch) = TouchscreenInput::detect() {
            backends.push(Box::new(touch) as Box<dyn InputBackend>);
        }
    }
    
    backends
}
```

---

## 3. Specific Integration Points

### 3.1 Boot Integration

**Current x86 Boot:**
```rust
// Current: bootloader sets up VESA mode
pub fn boot() {
    // UEFI boot
    uefi::initialize();
    
    // Set VESA mode 1024x768
    let mode = VesaMode::find(1024, 768, 32)
        .expect("Required mode not available");
    mode.set();
    
    // Pass framebuffer to kernel
    let fb_info = FramebufferInfo {
        base: mode.framebuffer,
        width: 1024,
        height: 768,
        pitch: mode.pitch,
        bpp: 32,
    };
    
    kernel::boot(BootInfo { framebuffer: fb_info, .. });
}
```

**Pi 5 Boot:**
```rust
// Pi 5: bootloader passes device tree, kernel probes DRM
pub fn boot() {
    // Device tree already parsed by firmware
    let dtb = DeviceTree::from_firmware();
    
    // Kernel will initialize DRM/KMS
    kernel::boot(BootInfo { 
        device_tree: dtb,
        // No pre-set framebuffer - driver will create
        .. 
    });
}

// In kernel init
fn kernel_init() {
    // Probe DRM devices
    let drm_devices = drm::probe_devices();
    
    // Initialize first connected display
    for dev in drm_devices {
        if let Some(connector) = dev.find_connected() {
            let mode = connector.preferred_mode();
            dev.set_mode(&connector, &mode)?;
            break;
        }
    }
}
```

### 3.2 Browser Rendering Integration

**Current CPU Rendering:**
```rust
// browser/rendering/cpu_renderer.rs

impl Renderer for CpuRenderer {
    fn render(&mut self, display_list: &DisplayList) {
        // Lock framebuffer
        let buffer = self.surface.lock_buffer();
        
        // Software rendering
        for cmd in &display_list.commands {
            match cmd {
                DrawCmd::Rect { rect, color } => {
                    self.fill_rect(buffer, rect, color);
                }
                DrawCmd::Text { pos, text, font } => {
                    self.render_text(buffer, pos, text, font);
                }
                // ...
            }
        }
        
        // Unlock and present
        self.surface.unlock_buffer(&[full_screen_rect]);
        self.backend.present();
    }
}
```

**Pi 5 GPU Rendering:**
```rust
// browser/rendering/gpu_renderer.rs

impl Renderer for GpuRenderer {
    fn render(&mut self, display_list: &DisplayList) {
        // Get EGL surface
        let surface = self.egl_surface;
        
        // Make current
        self.egl.make_current(&surface);
        
        // Build GPU commands via Skia or similar
        let canvas = self.skia_canvas;
        
        for cmd in &display_list.commands {
            match cmd {
                DrawCmd::Rect { rect, color } => {
                    canvas.draw_rect(rect, &Paint::from(color));
                }
                DrawCmd::Text { pos, text, font } => {
                    canvas.draw_text(text, pos.x, pos.y, &font.paint);
                }
                // ... GPU-accelerated primitives
            }
        }
        
        // Flush to GPU
        canvas.flush();
        
        // Swap buffers (vsync)
        self.egl.swap_buffers(&surface);
    }
}
```

### 3.3 Desktop Shell Integration

**No changes required!** The desktop is HTML/CSS/JS.

**System API Layer:**
```rust
// desktop/system_api.rs

/// System API exposed to JavaScript via bindings
impl SystemApi {
    /// Get display information
    pub fn get_display_info(&self) -> DisplayInfo {
        let backend = get_graphics_backend();
        let caps = backend.get_capabilities();
        
        DisplayInfo {
            width: caps.width,
            height: caps.height,
            refresh_rate: caps.refresh_rate,
            scale_factor: caps.scale_factor,
        }
    }
    
    /// Listen for input events
    pub fn on_input(&self, callback: js_sys::Function) {
        let input_manager = get_input_manager();
        
        input_manager.subscribe(move |event| {
            let js_event = event.to_js_value();
            callback.call1(&JsValue::NULL, &js_event).ok();
        });
    }
}
```

### 3.4 Input Event Mapping

**Event Translation:**

```rust
// input/event_translation.rs

/// Convert platform input to webbOS events
pub fn translate_input(event: PlatformEvent) -> WebbosEvent {
    match event {
        // Mouse events
        PlatformEvent::MouseMove { x, y } => {
            WebbosEvent::PointerMove {
                x: x as f64,
                y: y as f64,
                pointer_type: PointerType::Mouse,
            }
        }
        PlatformEvent::MouseButton { button, pressed } => {
            WebbosEvent::PointerButton {
                button: button as u16,
                pressed,
            }
        }
        
        // Keyboard events
        PlatformEvent::Key { keycode, pressed } => {
            let key = map_keycode(keycode);
            WebbosEvent::Key {
                key,
                code: keycode_to_code(keycode),
                pressed,
            }
        }
        
        // Touch events
        PlatformEvent::TouchDown { id, x, y } => {
            WebbosEvent::TouchStart {
                identifier: id as i32,
                client_x: x as f64,
                client_y: y as f64,
            }
        }
        PlatformEvent::TouchMove { id, x, y } => {
            WebbosEvent::TouchMove {
                identifier: id as i32,
                client_x: x as f64,
                client_y: y as f64,
            }
        }
        PlatformEvent::TouchUp { id } => {
            WebbosEvent::TouchEnd {
                identifier: id as i32,
            }
        }
        
        // Gesture events (from libinput)
        PlatformEvent::GestureSwipe { dx, dy } => {
            WebbosEvent::Wheel {
                delta_x: dx * -100.0, // Invert for natural scrolling
                delta_y: dy * -100.0,
            }
        }
        PlatformEvent::GesturePinch { scale } => {
            WebbosEvent::Zoom { scale }
        }
    }
}

/// USB HID keycode to WebbOS key mapping
fn map_keycode(keycode: u16) -> WebbosKey {
    match keycode {
        0x04 => WebbosKey::Character('a'),
        0x05 => WebbosKey::Character('b'),
        // ... full mapping
        0x28 => WebbosKey::Enter,
        0x29 => WebbosKey::Escape,
        0x2C => WebbosKey::Space,
        // Arrow keys
        0x50 => WebbosKey::ArrowLeft,
        0x51 => WebbosKey::ArrowDown,
        0x52 => WebbosKey::ArrowUp,
        0x4F => WebbosKey::ArrowRight,
        _ => WebbosKey::Unidentified,
    }
}
```

---

## 4. Build System Integration

### 4.1 Cargo Features

```toml
# kernel/Cargo.toml

[features]
default = ["x86_64"]

# Platform selection (mutually exclusive)
x86_64 = ["vesa", "ps2", "serial_debug"]
rpi5 = ["drm", "wayland", "usb_hid", "touchscreen", "gpio", "bluetooth"]

# Graphics backends
vesa = []
drm = ["mesa", "gbm", "libdrm"]
wayland = ["wayland-client", "wayland-protocols"]

# Input backends
ps2 = []
usb_hid = ["libusb", "libinput"]
touchscreen = ["i2c"]
gpio = ["libgpiod"]
bluetooth = ["bluez"]

# Graphics APIs
mesa = ["egl", "gles"]
vulkan = ["ash"]  # Future

# Debugging
serial_debug = []
fs_debug = []
```

### 4.2 Conditional Compilation

```rust
// graphics/mod.rs

#[cfg(feature = "vesa")]
pub mod vesa;
#[cfg(feature = "vesa")]
pub use vesa::VesaBackend as DefaultBackend;

#[cfg(feature = "drm")]
pub mod drm;
#[cfg(feature = "drm")]
pub use drm::DrmBackend as DefaultBackend;

// input/mod.rs

#[cfg(feature = "ps2")]
pub mod ps2;

#[cfg(feature = "usb_hid")]
pub mod usb_hid;

pub fn create_default_backends() -> Vec<Box<dyn InputBackend>> {
    let mut backends = Vec::new();
    
    #[cfg(feature = "ps2")]
    {
        backends.push(Box::new(ps2::Ps2Input::new()));
    }
    
    #[cfg(feature = "usb_hid")]
    {
        backends.push(Box::new(usb_hid::UsbHidInput::new()));
    }
    
    backends
}
```

### 4.3 Cross-Compilation

```makefile
# Makefile

# x86_64 build (default)
build-x86:
	cargo build --target x86_64-unknown-none --features x86_64

# Raspberry Pi 5 build
build-rpi5:
	cargo build --target aarch64-unknown-none --features rpi5

# With Docker cross-compiler
build-rpi5-docker:
	docker run --rm -v $(PWD):/workspace \
		rustembedded/cross:aarch64-unknown-none \
		cargo build --features rpi5
```

---

## 5. Runtime Adaptation

### 5.1 Runtime Backend Detection

```rust
// runtime/backend_detection.rs

/// Detect and initialize appropriate backends at runtime
pub fn initialize_platform() -> Result<Platform> {
    // Detect platform
    let platform = detect_platform()?;
    
    match platform {
        Platform::X86_64 => initialize_x86(),
        Platform::RaspberryPi5 => initialize_rpi5(),
        _ => Err(Error::UnsupportedPlatform),
    }
}

fn detect_platform() -> Result<Platform> {
    // Check CPU architecture
    #[cfg(target_arch = "x86_64")]
    return Ok(Platform::X86_64);
    
    #[cfg(target_arch = "aarch64")]
    {
        // Check device tree for Pi 5
        if device_tree_matches("raspberrypi,5-model-b") {
            return Ok(Platform::RaspberryPi5);
        }
    }
    
    Err(Error::UnknownPlatform)
}

fn initialize_rpi5() -> Result<Platform> {
    // 1. Initialize DRM/KMS
    let drm = DrmDevice::open("/dev/dri/card0")?;
    
    // 2. Initialize Mesa EGL
    let egl = EglDisplay::initialize_drm(&drm)?;
    
    // 3. Initialize Wayland connection
    let wayland = WaylandDisplay::connect_auto()?;
    
    // 4. Initialize input
    let mut inputs: Vec<Box<dyn InputBackend>> = vec![
        Box::new(UsbHidInput::new()),
    ];
    
    // Add touchscreen if detected
    if TouchscreenInput::is_present() {
        inputs.push(Box::new(TouchscreenInput::open()?));
    }
    
    Ok(Platform {
        graphics: Box::new(WaylandGraphics::new(wayland, egl)),
        inputs,
        thermal: Some(ThermalManager::new()),
    })
}
```

### 5.2 Feature Fallbacks

```rust
// graphics/renderer.rs

pub struct AdaptiveRenderer {
    primary: Box<dyn Renderer>,
    fallback: Box<dyn Renderer>,
    using_fallback: bool,
}

impl AdaptiveRenderer {
    pub fn new() -> Result<Self> {
        // Try GPU renderer first
        if let Ok(renderer) = GpuRenderer::new() {
            return Ok(Self {
                primary: Box::new(renderer),
                fallback: Box::new(CpuRenderer::new()),
                using_fallback: false,
            });
        }
        
        // Fall back to CPU
        warn!("GPU renderer unavailable, using CPU fallback");
        Ok(Self {
            primary: Box::new(CpuRenderer::new()),
            fallback: Box::new(CpuRenderer::new()),
            using_fallback: true,
        })
    }
    
    fn render(&mut self, display_list: &DisplayList) {
        if !self.using_fallback {
            if let Err(e) = self.primary.render(display_list) {
                error!("GPU render failed: {}, switching to fallback", e);
                self.using_fallback = true;
                self.fallback.render(display_list).unwrap();
            }
        } else {
            self.fallback.render(display_list).unwrap();
        }
    }
}
```

---

## 6. API Compatibility

### 6.1 JavaScript API Stability

**The `webbos` global object remains unchanged:**

```javascript
// desktop/apps/file_manager.js

// Works on both x86 and Pi 5 without changes!
async function loadDirectory(path) {
    const entries = await webbos.fs.readdir(path);
    
    for (const entry of entries) {
        // Create UI element
        const el = document.createElement('div');
        el.textContent = entry.name;
        el.className = entry.is_dir ? 'folder' : 'file';
        
        // Handle click (works with mouse OR touch!)
        el.addEventListener('click', () => {
            if (entry.is_dir) {
                navigateTo(entry.path);
            } else {
                openFile(entry.path);
            }
        });
        
        container.appendChild(el);
    }
}

// Input handling - unified across platforms
webbos.input.on('keydown', (e) => {
    if (e.key === 'F5') {
        refresh();
    }
});

// Touch gestures (automatically available on Pi 5 with touchscreen)
webbos.input.on('swipe', (e) => {
    if (e.direction === 'left') {
        goBack();
    }
});
```

### 6.2 Rust API Compatibility

```rust
// api/system.rs

/// Stable API for desktop environment
pub mod stable {
    /// Get system information (unchanged)
    pub fn get_system_info() -> SystemInfo {
        SystemInfo {
            os_name: "WebbOS",
            version: env!("CARGO_PKG_VERSION"),
            uptime: get_uptime(),
            platform: get_platform_name(), // "x86_64" or "rpi5"
        }
    }
    
    /// Launch application (unchanged)
    pub fn launch_app(app_id: &str) -> Result<Process> {
        process_manager().spawn(app_id)
    }
    
    /// Register input callback (works with all input types)
    pub fn on_input<F: Fn(InputEvent)>(callback: F) -> Subscription {
        input_manager().subscribe(callback)
    }
}

/// Platform-specific extensions
pub mod platform {
    /// Pi 5 specific features
    #[cfg(feature = "rpi5")]
    pub mod rpi5 {
        pub fn get_gpu_temperature() -> f32 {
            thermal::read_gpu_temp()
        }
        
        pub fn set_performance_profile(profile: Profile) {
            power_manager().set_profile(profile);
        }
    }
}
```

---

## 7. Migration Path

### 7.1 Phase 1: Abstraction (Week 1-2)

1. Create `GraphicsBackend` trait
2. Create `InputBackend` trait
3. Refactor existing VESA code to use traits
4. Verify x86 build still works

### 7.2 Phase 2: DRM Implementation (Week 3-4)

1. Implement `DrmBackend`
2. Implement `EglSurface`
3. Add Mesa integration
4. Test on Pi 5 hardware

### 7.3 Phase 3: Wayland Integration (Week 5-6)

1. Implement Wayland client
2. Create compositor integration
3. Port browser to use Wayland EGL
4. Test desktop shell

### 7.4 Phase 4: Input Unification (Week 7-8)

1. Implement USB HID
2. Implement touchscreen driver
3. Integrate libinput
4. Test all input methods

### 7.5 Phase 5: Optimization (Week 9-10)

1. Performance tuning
2. Thermal management
3. Power optimization
4. Final validation

---

## 8. Compatibility Matrix

| Feature | x86_64 (VESA) | RPi5 (DRM) | Notes |
|---------|---------------|------------|-------|
| Desktop Shell | ✅ | ✅ | Identical |
| Browser Engine | ✅ | ✅ | GPU accel on Pi 5 |
| Window Manager | ✅ | ✅ | Wayland on Pi 5 |
| Keyboard Input | ✅ (PS/2) | ✅ (USB) | Both supported |
| Mouse Input | ✅ (PS/2) | ✅ (USB) | Both supported |
| Touch Input | ❌ | ✅ | Pi 5 only |
| GPU Acceleration | ❌ | ✅ | Pi 5 only |
| 4K Output | ❌ | ✅ | Pi 5 only |
| Multi-Display | ❌ | ✅ | Pi 5 only |
| Thermal Management | ❌ | ✅ | Pi 5 only |
| Bluetooth Input | ❌ | ✅ | Pi 5 only |

---

## 9. Configuration Files

### 9.1 WebbOS Config (Platform Agnostic)

```json
// /etc/webbos/config.json
{
    "desktop": {
        "wallpaper": "/usr/share/wallpapers/default.jpg",
        "theme": "light",
        "icon_size": 48
    },
    "browser": {
        "homepage": "https://webbos.local",
        "enable_javascript": true,
        "cache_size_mb": 64
    },
    "input": {
        "mouse_accel": 1.0,
        "key_repeat_delay_ms": 500,
        "key_repeat_rate_hz": 30,
        "touch_emulation": false
    }
}
```

### 9.2 Platform-Specific Config

```json
// /etc/webbos/platform.json (auto-generated on boot)
{
    "platform": "rpi5",
    "graphics": {
        "backend": "drm",
        "resolution": "1920x1080@60",
        "scale_factor": 1.0,
        "gpu_accel": true
    },
    "thermal": {
        "fan_curve": "balanced",
        "throttle_warning": true
    },
    "power": {
        "usb_current_limit": "1.6A"
    }
}
```

---

## 10. Summary

### Key Integration Principles

1. **Abstraction First:** Clean traits enable platform independence
2. **Feature Flags:** Compile-time selection of backends
3. **API Stability:** JavaScript and stable Rust APIs unchanged
4. **Graceful Fallback:** Software renderer if GPU unavailable
5. **Unified Input:** Single API for all input types

### Files Modified

| File | Changes |
|------|---------|
| `kernel/drivers/graphics/mod.rs` | Add `GraphicsBackend` trait |
| `kernel/drivers/input/mod.rs` | Add `InputBackend` trait |
| `browser/rendering/mod.rs` | Use `GraphicsBackend` |
| `desktop/system_api.rs` | Platform detection |
| `Cargo.toml` | Add feature flags |
| `build.rs` | Platform-specific linking |

### Files Added

```
drivers/
├── graphics/
│   ├── drm.rs          # DRM/KMS backend
│   └── vesa.rs         # Existing, refactored
├── gpu/
│   └── mesa.rs         # Mesa EGL integration
├── input/
│   ├── usb_hid.rs      # USB HID driver
│   ├── touchscreen.rs  # Touch driver
│   └── libinput.rs     # libinput wrapper
└── thermal/
    └── rpi5.rs         # Thermal management
```

---

*"Vi automatisera det - one codebase, multiple platforms."* - Ingrid L'Ingénieure
