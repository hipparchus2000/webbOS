# Week 5 GUI & Input Implementation Plan

**Document Type:** Implementation Planning  
**Author:** Sofia La Savante (Research & Architecture)  
**Date:** February 15, 2026  
**Version:** 1.0  
**Status:** Planning Phase

---

## Executive Summary

This document provides a lightweight but detailed implementation plan for porting webbOS GUI and Input subsystems to the Raspberry Pi 5 platform. Based on comprehensive research (Sofia's Week 5 research) and technical architecture specifications (Ingrid's planning output), this plan defines the architecture, components, integration points, dependencies, testing strategy, and risk mitigation for the GUI/Input implementation.

**Key Technical Decisions:**
- **Graphics API:** OpenGL ES 3.1 (broad compatibility)
- **Display Server:** Wayland with Labwc compositor
- **Input Stack:** libinput for unified input management
- **GPU Memory:** CMA 256MB dynamic allocation
- **Target Resolution:** 1080p60 (performance/quality balance)

---

## 1. Architecture Overview

### 1.1 Display Pipeline (GPU → HDMI/DSI)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           DISPLAY PIPELINE ARCHITECTURE                       │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  User Space                    ┌─────────────────────┐                       │
│  ┌──────────────────┐          │   WebbOS Browser    │                       │
│  │  Application     │─────────▶│   Engine            │                       │
│  │  (GUI/Content)   │          │   (Skia/Vulkan)     │                       │
│  └──────────────────┘          └──────────┬──────────┘                       │
│                                           │                                   │
│  ┌────────────────────────────────────────▼──────────────────────────────┐   │
│  │                    Wayland Client API (wl_display)                    │   │
│  │     - wl_surface (drawing surfaces)                                   │   │
│  │     - xdg_surface (window management)                                 │   │
│  └────────────────────────────────────────┬───────────────────────────────┘   │
│                                           │                                   │
│  System Services           ┌──────────────▼──────────────┐                    │
│  ┌─────────────────────────▼─────────────────────────────▼────────────────┐  │
│  │                     Wayland Compositor (Labwc)                          │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │  Window     │  │  Hardware   │  │   Damage    │  │   VSync       │  │  │
│  │  │  Manager    │  │  Planes     │  │  Tracking   │  │   Control     │  │  │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └───────┬───────┘  │  │
│  │         └─────────────────┴────────────────┴─────────────────┘        │  │
│  └─────────────────────────────────┬─────────────────────────────────────┘  │
│                                    │                                        │
│  ┌─────────────────────────────────▼─────────────────────────────────────┐  │
│  │                    Mesa GPU Driver (V3D)                               │  │
│  │         ┌──────────────────┐      ┌──────────────────┐                │  │
│  │         │  OpenGL ES 3.1   │      │   Vulkan 1.3     │                │  │
│  │         │   Rendering      │      │   Rendering      │                │  │
│  │         └────────┬─────────┘      └────────┬─────────┘                │  │
│  │                  └──────────────┬──────────┘                          │  │
│  └─────────────────────────────────┼─────────────────────────────────────┘  │
│                                    │                                        │
│  Kernel Space        ┌─────────────▼──────────────┐                        │
│  ┌───────────────────▼────────────────────────────▼──────────────────────┐  │
│  │                          DRM/KMS Subsystem                             │  │
│  │     ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐    │  │
│  │     │  CRTC    │  │  Planes  │  │ Encoders │  │   Connectors     │    │  │
│  │     │(Pipeline)│  │(Overlay) │  │ (Format) │  │(HDMI/DSI/DP)     │    │  │
│  │     └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬─────────┘    │  │
│  │          └─────────────┴─────────────┴─────────────────┘              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│  Hardware            ┌─────────────▼──────────────┐                        │
│  ┌───────────────────▼────────────────────────────▼──────────────────────┐  │
│  │                     VideoCore VII GPU                                  │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐   │  │
│  │  │   V3D    │  │   HVS    │  │  HDMI0   │  │       DSI            │   │  │
│  │  │  (3D)    │  │(Scaler)  │  │  (TX)    │  │   (Display)          │   │  │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────────┬───────────┘   │  │
│  │       └─────────────┴─────────────┴───────────────────┘               │  │
│  │                              │                                        │  │
│  │       ┌──────────────────────▼───────────────────────┐                │  │
│  │       │         Display Output (HDMI/DSI)            │                │  │
│  │       └──────────────────────────────────────────────┘                │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                               │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Pipeline Flow:**
1. **Application Layer:** WebbOS browser engine renders content using Skia/Vulkan
2. **Wayland Protocol:** Surfaces submitted to compositor via Wayland protocol
3. **Compositor (Labwc):** Handles window management, hardware planes, damage tracking
4. **Mesa/V3D:** OpenGL ES/Vulkan rendering with shader compilation
5. **DRM/KMS:** Kernel display subsystem for mode setting and page flipping
6. **VideoCore VII:** Hardware GPU with V3D (3D rendering) and HVS (video scaling)
7. **Physical Output:** HDMI 2.1 or DSI display interface

### 1.2 Input Pipeline (USB/GPIO → Events)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           INPUT PIPELINE ARCHITECTURE                         │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  Hardware Layer                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   USB xHCI   │  │  I2C (RP1)   │  │ GPIO (RP1)   │  │  Bluetooth 5.0   │  │
│  │  Controller  │  │  Controller  │  │ Controller   │  │   (SDIO)         │  │
│  │  (5 Gbps)    │  │              │  │ (40-pin)     │  │                  │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                 │                   │           │
│         ▼                 ▼                 ▼                   ▼           │
│  Device Drivers                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   USB HID    │  │  Goodix      │  │   GPIO       │  │  Bluetooth       │  │
│  │  (usbhid.ko) │  │  GT911       │  │   Keys       │  │  HID             │  │
│  │              │  │  (touch)     │  │  (gpio-keys) │  │  (hidp)          │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                 │                   │           │
│         └─────────────────┴─────────────────┴───────────────────┘           │
│                                   │                                          │
│                                   ▼                                          │
│  Kernel Subsystem                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                         Input Subsystem (input.c)                     │   │
│  │  ┌────────────────────────────────────────────────────────────────┐  │   │
│  │  │                    evdev (Event Device)                         │  │   │
│  │  │  Event Types:                                                   │  │   │
│  │  │  - EV_KEY (buttons, keys)                                      │  │   │
│  │  │  - EV_REL (relative movement - mouse)                          │  │   │
│  │  │  - EV_ABS (absolute position - touch)                          │  │   │
│  │  │  - EV_SYN (synchronization)                                    │  │   │
│  │  │  Protocols:                                                     │  │   │
│  │  │  - MT (Multi-Touch) for capacitive touchscreens                │  │   │
│  │  │  - MSC (Miscellaneous) for tablet-specific data                │  │   │
│  │  └────────────────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────┬───────────────────────────────────┘   │
│                                     │                                        │
│                                     ▼                                        │
│  Userspace Libraries                                                          │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                         libinput                                     │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐ │   │
│  │  │   Pointer    │  │   Keyboard   │  │         Touch                │ │   │
│  │  │  Handling    │  │   Handling   │  │    (Multitouch/Gestures)     │ │   │
│  │  │  - Accel     │  │  - Mapping   │  │    - Tap detection           │ │   │
│  │  │  - Gestures  │  │  - Composing │  │    - Gesture recognition     │ │   │
│  │  └──────┬───────┘  └──────┬───────┘  └──────────────┬───────────────┘ │   │
│  │         └─────────────────┴─────────────────────────┘                 │   │
│  └──────────────────────────────────┬─────────────────────────────────────┘   │
│                                     │                                        │
│                                     ▼                                        │
│  WebbOS Integration                                                           │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    WebbOS Input Manager                              │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐ │   │
│  │  │   Event      │  │   Focus      │  │      Gesture                 │ │   │
│  │  │   Routing    │  │   Management │  │    Recognition               │ │   │
│  │  │  (dispatch)  │  │  (window)    │  │   (custom impl)              │ │   │
│  │  └──────────────┘  └──────────────┘  └──────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                               │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Input Pipeline Flow:**
1. **Hardware:** USB xHCI, I2C touch controllers, GPIO buttons, Bluetooth HID
2. **Device Drivers:** Kernel drivers translate hardware signals to input events
3. **evdev:** Unified event device interface in kernel space
4. **libinput:** Userspace library providing device abstraction and gesture recognition
5. **WebbOS Input Manager:** Application-level event routing and focus management

### 1.3 Graphics Acceleration Integration Points

| Integration Point | Component | API/Protocol | Responsibility |
|-------------------|-----------|--------------|----------------|
| **Application** | WebbOS Browser | OpenGL ES 3.1 / Vulkan 1.3 | Content rendering |
| **Window System** | Wayland Compositor | Wayland Protocol | Surface management |
| **Rendering** | Mesa V3D | EGL / GBM | Context creation, buffer management |
| **Kernel** | DRM/KMS | DRM ioctl | Display control, mode setting |
| **Hardware** | VideoCore VII | N/A (Hardware) | GPU execution |

**Key Integration APIs:**

```c
// EGL Context Creation (Application Layer)
EGLDisplay eglGetPlatformDisplay(EGL_PLATFORM_WAYLAND_EXT, 
                                  wl_display, NULL);
EGLContext eglCreateContext(egl_display, config, EGL_NO_CONTEXT, 
                            context_attribs);

// GBM Buffer Allocation (Mesa → DRM)
gbm_device *gbm_create_device(int drm_fd);
gbm_surface *gbm_surface_create(gbm_device, width, height, 
                                 GBM_FORMAT_XRGB8888, flags);

// DRM Atomic Commit (Kernel)
struct drm_mode_atomic_req_ptr req = drmModeAtomicAlloc();
drmModeAtomicAddProperty(req, plane_id, property_id, value);
drmModeAtomicCommit(drm_fd, req, flags, user_data);
```

---

## 2. Component Breakdown

### 2.1 Display Driver Requirements

| Component | Requirement | Pi 5 Implementation | Status |
|-----------|-------------|---------------------|--------|
| **DRM Driver** | Kernel 6.6+ with VC4/VC5 | `vc4-drm` (VideoCore IV/V/VI/VII) | ✅ Available |
| **KMS Support** | Atomic modesetting | Full atomic commit support | ✅ Supported |
| **Output** | HDMI 2.1 / DSI | Dual 4K60 HDMI, DSI via MIPI | ✅ Supported |
| **Color Depth** | 8/10/12-bit | 4K60@12-bit (YCbCr 422) | ✅ Supported |
| **HDR** | Metadata support | HDR10 (limited) | ⚠️ Partial |

**Driver Configuration:**
```bash
# /boot/firmware/config.txt
dtoverlay=vc4-kms-v3d,cma-256
dtoverlay=disable-wifi  # Optional: Free up bandwidth
dtoverlay=disable-bt    # Optional: Free up bandwidth
```

**Required Kernel Modules:**
```
vc4                    # Main DRM driver
v3d                    # 3D acceleration
drm_kms_helper         # KMS support
drm                    # Core DRM
```

### 2.2 Input Driver Requirements

| Input Type | Driver | Kernel Module | Userspace |
|------------|--------|---------------|-----------|
| **USB Keyboard** | usbhid | `usbhid`, `hid-generic` | libinput |
| **USB Mouse** | usbhid | `usbhid`, `hid-generic` | libinput |
| **Touchscreen** | Goodix/FT5x06 | `goodix`, `edt-ft5x06` | libinput |
| **GPIO Buttons** | gpio-keys | `gpio_keys` | libinput |
| **Bluetooth HID** | hidp | `hidp`, `bluez` | libinput |

**Device Tree Overlays Required:**
```dts
// Goodix GT911 Touch Overlay
/dts-v1/;
/plugin/;

/ {
    compatible = "raspberrypi,5-model-b", "brcm,bcm2712";
    
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

### 2.3 Graphics Library Integration

| Library | Version | Purpose | Integration Point |
|---------|---------|---------|-------------------|
| **Mesa** | 23.0+ | OpenGL/Vulkan drivers | System package |
| **libEGL** | 1.5 | Context creation | Application |
| **libGLESv2** | 3.1 | OpenGL ES rendering | Application |
| **libvulkan** | 1.3 | Vulkan rendering | Application (optional) |
| **libgbm** | Latest | Buffer management | Mesa/Compositor |
| **libdrm** | 2.4.100+ | DRM interface | Compositor/Direct |

**Required Packages (Debian/Raspberry Pi OS):**
```bash
# Core graphics
libgl1-mesa-dri libglx-mesa0 mesa-vulkan-drivers

# OpenGL ES
libgles2-mesa libegl1-mesa

# Development headers
libgles2-mesa-dev libegl1-mesa-dev libdrm-dev

# Wayland
libwayland-client0 libwayland-server0 libwayland-bin
libwayland-dev wayland-protocols

# Input
libinput10 libinput-dev libinput-bin

# Compositor
labwc wayland-utils
```

### 2.4 Window Management System

**Selected Compositor: Labwc**

| Feature | Requirement | Labwc Support |
|---------|-------------|---------------|
| **Protocol** | Wayland Core | ✅ Full |
| **XDG Shell** | Window management | ✅ xdg-shell stable |
| **Layer Shell** | Desktop components | ✅ wlr-layer-shell |
| **Damage Tracking** | Performance | ✅ Automatic |
| **VSync** | Tear-free | ✅ DRM atomic |
| **Hardware Planes** | Efficiency | ✅ DRM planes |

**Configuration (/etc/xdg/labwc/rc.xml):**
```xml
<?xml version="1.0"?>
<labwc_config>
    <core>
        <gap>0</gap>
        <adaptiveSync>no</adaptiveSync>
    </core>
    <theme>
        <name>webbos</name>
        <cornerRadius>0</cornerRadius>
    </theme>
    <libinput>
        <device category="default">
            <naturalScroll>no</naturalScroll>
            <leftHanded>no</leftHanded>
        </device>
    </libinput>
</labwc_config>
```

---

## 3. Integration Points

### 3.1 GUI Integration with Existing webbOS Desktop

**Current Architecture Assessment:**

| Component | Current State | Pi 5 Adaptation | Migration Strategy |
|-----------|---------------|-----------------|-------------------|
| **Graphics Backend** | X11/Framebuffer | Wayland | Progressive migration |
| **Rendering** | Software/OpenGL | OpenGL ES 3.1 | API compatibility layer |
| **Windowing** | Custom/X11 | Wayland native | Replace with wlroots |
| **Input** | Direct evdev | libinput | Unified input layer |

**Integration Architecture:**

```
┌─────────────────────────────────────────────────────────────────┐
│                     webbOS Desktop Layer                        │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Platform Abstraction Layer (PAL) - NEW                   │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │ │
│  │  │   Display    │  │    Input     │  │   Window     │    │ │
│  │  │   Backend    │  │   Backend    │  │   Manager    │    │ │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘    │ │
│  │         │                 │                 │            │ │
│  │  ┌──────▼─────────────────▼─────────────────▼──────┐     │ │
│  │  │         Platform Detection & Routing            │     │ │
│  │  │    (x86 → X11/Wayland, Pi 5 → Wayland only)     │     │ │
│  │  └─────────────────────────────────────────────────┘     │ │
│  └───────────────────────────────────────────────────────────┘ │
│                              │                                  │
│  ┌───────────────────────────▼──────────────────────────────┐  │
│  │              WebbOS Browser Engine                        │  │
│  │         (HTML/CSS/JS → Skia/Vulkan)                      │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**Key Integration Files:**
```
/src/platform/
├── display/
│   ├── display_backend.h      # Abstract display interface
│   ├── display_wayland.c      # Wayland implementation (Pi 5)
│   └── display_x11.c          # X11 implementation (x86)
├── input/
│   ├── input_backend.h        # Abstract input interface
│   ├── input_libinput.c       # libinput implementation
│   └── input_evdev.c          # Direct evdev fallback
└── window/
    ├── window_manager.h
    └── window_wayland.c
```

### 3.2 Input Event Handling with Existing System

**Event Flow Integration:**

```
┌─────────────────────────────────────────────────────────────────┐
│                     Input Event Flow                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Hardware ──▶ Kernel ──▶ libinput ──▶ webbOS Input Manager      │
│              (evdev)    (gestures)    (application routing)     │
│                                                                 │
│  Event Types:                                                   │
│  ┌──────────────┬─────────────────────────────────────────────┐ │
│  │  POINTER     │  Motion, Button press/release, Scroll       │ │
│  │  KEYBOARD    │  Key press/release, Modifiers               │ │
│  │  TOUCH       │  Down/Motion/Up, Multi-touch, Gestures      │ │
│  │  GESTURE     │  Swipe, Pinch, Tap (libinput synthesized)   │ │
│  └──────────────┴─────────────────────────────────────────────┘ │
│                                                                 │
│  Integration Points:                                            │
│  1. Register with libinput udev context                         │
│  2. Set up event loop (epoll/kqueue)                            │
│  3. Translate to webbOS internal events                         │
│  4. Route to focused window/application                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Event Translation Layer:**
```c
// Input event mapping
struct webbos_input_event {
    enum event_type { POINTER, KEYBOARD, TOUCH, GESTURE } type;
    uint32_t timestamp;
    union {
        struct { float x, y; uint32_t buttons; } pointer;
        struct { uint32_t keycode; bool pressed; } keyboard;
        struct { int id; float x, y; enum { DOWN, MOVE, UP } state; } touch;
        struct { enum { SWIPE, PINCH, TAP } type; float dx, dy; } gesture;
    } data;
};

// libinput → webbOS translation
void translate_libinput_event(struct libinput_event *event) {
    switch (libinput_event_get_type(event)) {
        case LIBINPUT_EVENT_POINTER_MOTION:
            // Translate to webbos_input_event.pointer
            break;
        case LIBINPUT_EVENT_TOUCH_DOWN:
        case LIBINPUT_EVENT_TOUCH_MOTION:
            // Translate to webbos_input_event.touch
            break;
        // ... other event types
    }
}
```

### 3.3 Display Output with Existing Graphics Stack

**Compatibility Matrix:**

| webbOS Component | Current Backend | Pi 5 Backend | Compatibility |
|------------------|-----------------|--------------|---------------|
| **Renderer** | Skia/OpenGL | Skia/OpenGL ES | ✅ Compatible |
| **Compositor** | Custom/X11 | Labwc/Wayland | ⚠️ Migration needed |
| **Video** | VA-API/OMX | V4L2 M2M | ⚠️ API change |
| **Fonts** | FreeType | FreeType | ✅ Compatible |
| **Images** | libpng, libjpeg | libpng, libjpeg | ✅ Compatible |

**Graphics Stack Bridge:**

```
┌─────────────────────────────────────────────────────────────────┐
│              Graphics Stack Integration                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  webbOS Application                                             │
│         │                                                       │
│         ▼                                                       │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Skia Graphics Library                                    │ │
│  │  ┌───────────────────┬───────────────────┐               │ │
│  │  │  OpenGL Backend   │  Vulkan Backend   │               │ │
│  │  │  (Pi 5 Primary)   │  (Pi 5 Optional)  │               │ │
│  │  └─────────┬─────────┴─────────┬─────────┘               │ │
│  │            │                   │                         │ │
│  └────────────┼───────────────────┼─────────────────────────┘ │
│               │                   │                           │
│  ┌────────────▼───────────────────▼───────────────────────────┐│
│  │              EGL Context (Mesa)                            ││
│  │  ┌──────────────────────────────────────────────────────┐  ││
│  │  │  Platform: EGL_PLATFORM_WAYLAND_EXT                  │  ││
│  │  │  API: EGL_OPENGL_ES_API                              │  ││
│  │  │  Version: ES 3.1                                     │  ││
│  │  └──────────────────────────────────────────────────────┘  ││
│  └────────────────────────────────────────────────────────────┘│
│               │                                                │
│  ┌────────────▼────────────────────────────────────────────────┤
│  │              Wayland Compositor (Labwc)                     │
│  │         (Surface composition, hardware planes)              │
│  └────────────────────────────────────────────────────────────┘
│               │
│  ┌────────────▼────────────────────────────────────────────────┐
│  │              DRM/KMS → VideoCore VII → Display              │
│  └─────────────────────────────────────────────────────────────┘
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Dependency Analysis

### 4.1 Hardware Dependencies (Pi 5 Specific)

| Component | Dependency | Impact if Missing | Mitigation |
|-----------|------------|-------------------|------------|
| **VideoCore VII** | GPU acceleration | Software rendering (slow) | LLVMpipe fallback |
| **4K HDMI** | Dual display output | Limited to 1080p or single display | Single display config |
| **RP1 I/O** | USB/GPIO controller | No USB/GPIO input | Alternative input methods |
| **Active Cooler** | Thermal management | Performance throttling | Passive + throttling acceptance |
| **CMA Memory** | GPU memory allocation | GPU allocation failures | Smaller framebuffer |

**Hardware Feature Matrix:**

```
┌──────────────────┬──────────────┬──────────────┬──────────────────────────┐
│ Feature          │ Required     │ Pi 5 Support │ Fallback Strategy        │
├──────────────────┼──────────────┼──────────────┼──────────────────────────┤
│ OpenGL ES 3.1    │ Yes          │ ✅ Native    │ None needed              │
│ Vulkan 1.3       │ No           │ ✅ Native    │ OpenGL ES 3.1            │
│ Dual 4K60        │ No           │ ✅ Native    │ Single 1080p             │
│ DSI Touch        │ No           │ ✅ Native    │ USB touch / No touch     │
│ Hardware Video   │ No           │ ⚠️ Partial   │ CPU decode               │
│ Decode           │              │ (No H.264)   │                          │
│ Thermal Control  │ Recommended  │ ⚠️ Requires  │ Throttle acceptance      │
│                  │              │   active     │                          │
│                  │              │   cooling    │                          │
└──────────────────┴──────────────┴──────────────┴──────────────────────────┘
```

### 4.2 Software Dependencies

**Core Dependencies:**

| Package | Min Version | Purpose | Source |
|---------|-------------|---------|--------|
| `linux-image` | 6.6+ | Kernel with VC4/RP1 support | Raspberry Pi OS |
| `mesa` | 23.0+ | OpenGL/Vulkan drivers | Raspberry Pi OS |
| `libwayland-client` | 1.20+ | Wayland protocol | Debian repos |
| `libinput` | 1.20+ | Input handling | Debian repos |
| `labwc` | 0.6+ | Wayland compositor | Debian repos |
| `libdrm` | 2.4.100+ | DRM interface | Debian repos |

**Build Dependencies:**

```bash
# Essential build packages
sudo apt install build-essential cmake pkg-config

# Graphics development
sudo apt install libgles2-mesa-dev libegl1-mesa-dev \
    libwayland-dev wayland-protocols libdrm-dev

# Input development
sudo apt install libinput-dev libudev-dev

# Additional libraries
sudo apt install libpixman-1-dev libxkbcommon-dev
```

### 4.3 Build System Requirements

**Cross-Compilation Setup:**

```makefile
# Makefile excerpt for Pi 5 cross-compilation

# Toolchain
CROSS_COMPILE ?= aarch64-linux-gnu-
CC = $(CROSS_COMPILE)gcc
CXX = $(CROSS_COMPILE)g++

# Pi 5 specific flags
CFLAGS += -march=armv8.2-a+crc+crypto -mtune=cortex-a76
CFLAGS += -DPLATFORM_PI5 -DUSE_WAYLAND -DUSE_GLES31

# Include paths (sysroot)
CFLAGS += -I$(SYSROOT)/usr/include
CFLAGS += -I$(SYSROOT)/usr/include/libdrm
CFLAGS += -I$(SYSROOT)/usr/include/wayland

# Library paths
LDFLAGS += -L$(SYSROOT)/usr/lib/aarch64-linux-gnu

# Required libraries
LIBS += -lwayland-client -lwayland-egl
LIBS += -lGLESv2 -lEGL
LIBS += -ldrm -lgbm
LIBS += -linput -ludev
```

**CMake Configuration:**

```cmake
# CMakeLists.txt for Pi 5 GUI build

if(PLATFORM_PI5)
    # Architecture flags
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -march=armv8.2-a+crc+crypto")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -mtune=cortex-a76")
    
    # Feature flags
    add_definitions(-DUSE_WAYLAND=1)
    add_definitions(-DUSE_GLES31=1)
    add_definitions(-DUSE_LIBINPUT=1)
    
    # Find packages
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(WAYLAND wayland-client wayland-egl REQUIRED)
    pkg_check_modules(GLES glesv2 egl REQUIRED)
    pkg_check_modules(DRM libdrm REQUIRED)
    pkg_check_modules(INPUT libinput REQUIRED)
endif()
```

---

## 5. Testing Strategy

### 5.1 QEMU Testing for Display/Input

**QEMU Configuration for Pi 5:**

```bash
# QEMU system emulation for Pi 5 (limited GPU support)
qemu-system-aarch64 \
    -M virt,highmem=off \
    -cpu cortex-a76 \
    -m 4G \
    -smp 4 \
    -bios /usr/share/qemu-efi-aarch64/QEMU_EFI.fd \
    -drive file=webbos-pi5-test.img,format=raw,if=sd,id=hd0 \
    -device usb-host,vendorid=0x046d,productid=0xc52b \
    -device virtio-gpu-pci \
    -display sdl,gl=on \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device virtio-net-pci,netdev=net0

# Limitations:
# - No VideoCore VII emulation (uses virtio-gpu)
# - Performance not representative
# - Input testing via USB passthrough only
```

**QEMU Testing Scope:**

| Test Category | QEMU Support | Testing Approach |
|---------------|--------------|------------------|
| **Input Protocol** | ✅ Full | libinput event verification |
| **Wayland Client** | ✅ Full | Protocol compliance |
| **Application Logic** | ✅ Full | Functional testing |
| **GPU Rendering** | ⚠️ Limited | virtio-gpu (not VideoCore) |
| **Display Output** | ❌ None | Cannot test DRM/KMS |
| **Performance** | ❌ None | Not representative |

### 5.2 Hardware Testing Requirements

**Test Hardware Setup:**

```
┌─────────────────────────────────────────────────────────────────┐
│                 Pi 5 Hardware Test Setup                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Required Hardware:                                             │
│  ┌──────────────────┬─────────────────────────────────────────┐│
│  │ Raspberry Pi 5   │ 4GB or 8GB model                        ││
│  │ Active Cooler    │ Required for sustained testing          ││
│  │ Power Supply     │ 5V 5A (official)                        ││
│  │ MicroSD Card     │ 32GB+ (Class A2 for performance)        ││
│  │ HDMI Monitor     │ 1080p minimum, 4K preferred             ││
│  │ USB Keyboard     │ For input testing                       ││
│  │ USB Mouse        │ For pointer testing                     ││
│  │ USB Touch (opt)  │ For touch testing                       ││
│  │ DSI Display(opt) │ For DSI testing                         ││
│  └──────────────────┴─────────────────────────────────────────┘│
│                                                                 │
│  Test Configurations:                                           │
│  1. Single 1080p HDMI                                           │
│  2. Dual 1080p HDMI                                             │
│  3. Single 4K HDMI                                              │
│  4. DSI + HDMI                                                  │
│  5. Touch overlay on DSI                                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Automated Test Script:**

```bash
#!/bin/bash
# hardware-test-gui.sh - Automated GUI hardware testing

# Test 1: Display detection
echo "=== Display Detection Test ==="
modetest -M vc4 -c  # List connectors
modetest -M vc4 -p  # List properties

# Test 2: OpenGL ES capabilities
echo "=== OpenGL ES Test ==="
es2_info | head -30
glmark2-es2-wayland --fullscreen --size 1920x1080

# Test 3: Input device detection
echo "=== Input Device Test ==="
libinput list-devices

# Test 4: Event monitoring (5 second sample)
echo "=== Input Event Test (5s) ==="
timeout 5 libinput debug-events || true

# Test 5: Thermal baseline
echo "=== Thermal Baseline ==="
echo "Idle temperature: $(vcgencmd measure_temp)"

# Test 6: Stress test with thermal monitoring
echo "=== Stress Test (60s) ==="
stress-ng --cpu 4 --timeout 60 &
STRESS_PID=$!
for i in {1..12}; do
    sleep 5
    echo "Temp: $(vcgencmd measure_temp), Throttle: $(vcgencmd get_throttled)"
done
wait $STRESS_PID

echo "=== All Tests Complete ==="
```

### 5.3 Performance Benchmarking Approach

**Benchmark Suite:**

| Benchmark | Tool | Metric | Target |
|-----------|------|--------|--------|
| **GPU Graphics** | glmark2-es2 | Score | >1000 |
| **Window Compositing** | weston-simple-egl | FPS | 60 fps |
| **Input Latency** | Custom | ms | <16 ms |
| **Memory Usage** | smem | MB | <512 MB GUI |
| **Thermal Performance** | stress-ng | °C | <80°C sustained |

**Benchmark Script:**

```bash
#!/bin/bash
# performance-benchmark.sh

RESULTS_FILE="benchmark-$(date +%Y%m%d-%H%M%S).txt"

echo "=== WebbOS Pi 5 Performance Benchmark ===" | tee $RESULTS_FILE
echo "Date: $(date)" | tee -a $RESULTS_FILE
echo "" | tee -a $RESULTS_FILE

# 1. glmark2-es2
echo "1. GPU Benchmark (glmark2-es2-wayland)..." | tee -a $RESULTS_FILE
timeout 120 glmark2-es2-wayland --fullscreen --size 1920x1080 2>&1 | \
    tee -a $RESULTS_FILE | grep "glmark2 Score"

# 2. Memory baseline
echo "" | tee -a $RESULTS_FILE
echo "2. Memory Usage..." | tee -a $RESULTS_FILE
cat /proc/meminfo | grep -E "MemTotal|MemAvailable|Buffers|Cached" | \
    tee -a $RESULTS_FILE

# 3. Thermal stress test
echo "" | tee -a $RESULTS_FILE
echo "3. Thermal Stress Test (5 min)..." | tee -a $RESULTS_FILE
echo "Baseline: $(vcgencmd measure_temp)" | tee -a $RESULTS_FILE
stress-ng --cpu 4 --timeout 300 &
STRESS_PID=$!
for i in {1..30}; do
    sleep 10
    echo "$(date '+%H:%M:%S'): $(vcgencmd measure_temp)" | tee -a $RESULTS_FILE
done
wait $STRESS_PID
echo "Post-stress: $(vcgencmd measure_temp)" | tee -a $RESULTS_FILE

# 4. Throttling check
echo "" | tee -a $RESULTS_FILE
echo "4. Throttling Status..." | tee -a $RESULTS_FILE
THROTTLED=$(vcgencmd get_throttled)
echo "Throttled: $THROTTLED" | tee -a $RESULTS_FILE
if [ "$THROTTLED" != "throttled=0x0" ]; then
    echo "WARNING: Throttling detected!" | tee -a $RESULTS_FILE
fi

echo "" | tee -a $RESULTS_FILE
echo "=== Benchmark Complete ===" | tee -a $RESULTS_FILE
echo "Results saved to: $RESULTS_FILE"
```

---

## 6. Risk Assessment

### 6.1 Technical Challenges

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Wayland Migration Complexity** | High | High | Phased migration; X11 compatibility layer |
| **Video Decode Performance** | Medium | Medium | Use HEVC; CPU decode fallback |
| **Thermal Throttling** | High | Medium | Active cooling; thermal monitoring |
| **USB Device Compatibility** | Low | Low | Test with target devices; hub compatibility |
| **DSI Touch Calibration** | Medium | Low | Standard drivers; calibration tools |

### 6.2 Performance Bottlenecks

**Identified Bottlenecks:**

```
┌─────────────────────────────────────────────────────────────────┐
│                 Performance Risk Matrix                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. MEMORY BANDWIDTH                                            │
│     Risk: 4K dual display saturates LPDDR4X                     │
│     Impact: Frame drops, stuttering                             │
│     Mitigation:                                                 │
│     - Limit to single 4K or dual 1080p                          │
│     - Use hardware planes for composition                       │
│     - Implement texture compression                             │
│                                                                 │
│  2. GPU COMPUTE                                                 │
│     Risk: Complex shaders overwhelm V3D                         │
│     Impact: Low FPS, jank                                       │
│     Mitigation:                                                 │
│     - Optimize shaders for mobile                               │
│     - Use GPU profiling tools                                   │
│     - Fallback to simpler effects                               │
│                                                                 │
│  3. THERMAL THROTTLING                                          │
│     Risk: >80°C triggers frequency reduction                    │
│     Impact: Performance degradation                             │
│     Mitigation:                                                 │
│     - Active cooling required                                   │
│     - Dynamic quality scaling                                   │
│     - Thermal-aware scheduling                                  │
│                                                                 │
│  4. INPUT LATENCY                                               │
│     Risk: RP1 PCIe adds input latency                           │
│     Impact: Perceived lag                                       │
│     Mitigation:                                                 │
│     - Interrupt-driven input                                    │
│     - Minimize input pipeline depth                             │
│     - Predictive input handling                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 6.3 Compatibility Issues

**Compatibility Risk Matrix:**

| Component | Risk Level | Issue | Workaround |
|-----------|------------|-------|------------|
| **X11 Apps** | High | XWayland required | Ship XWayland |
| **Legacy Input** | Medium | Old touch controllers | Updated overlays |
| **Custom Displays** | Medium | EDID issues | Mode forcing |
| **Bluetooth HID** | Low | Pairing issues | BlueZ updates |
| **WebGL Content** | Medium | Performance variance | Content optimization |

### 6.4 Risk Mitigation Strategies

**Mitigation Plan:**

```
┌─────────────────────────────────────────────────────────────────┐
│              Risk Mitigation Strategies                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  HIGH PRIORITY                                                  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ 1. Wayland Migration Strategy                             │ │
│  │    ┌─────────────────────────────────────────────────┐    │ │
│  │    │ Phase 1: Implement Wayland backend alongside X11  │    │ │
│  │    │ Phase 2: Test on Pi 5 hardware                    │    │ │
│  │    │ Phase 3: Migrate internal apps to native Wayland  │    │ │
│  │    │ Phase 4: Enable XWayland for legacy apps          │    │ │
│  │    │ Phase 5: Deprecate X11 backend                    │    │ │
│  │    └─────────────────────────────────────────────────┘    │ │
│  │                                                           │ │
│  │ 2. Thermal Management                                     │ │
│  │    ┌─────────────────────────────────────────────────┐    │ │
│  │    │ • Active cooling mandatory for production       │    │ │
│  │    │ • Thermal monitoring daemon                       │    │ │
│  │    │ • Dynamic GPU frequency scaling                   │    │ │
│  │    │ • Quality reduction at high temperatures          │    │ │
│  │    └─────────────────────────────────────────────────┘    │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  MEDIUM PRIORITY                                                │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ 3. Video Decode Fallback                                  │ │
│  │    • Prefer HEVC over H.264                               │ │
│  │    • CPU decode for legacy content                        │ │
│  │    • Quality/bitrate adaptation                           │ │
│  │                                                           │ │
│  │ 4. Input Device Testing Matrix                            │ │
│  │    • Test 20+ common USB HID devices                      │ │
│  │    • Test 5+ touch controllers                            │ │
│  │    • Maintain compatibility list                          │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  LOW PRIORITY                                                   │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ 5. Display Compatibility                                  │ │
│  │    • EDID database updates                                │ │
│  │    • User-configurable modes                              │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 7. Component Dependency Matrix

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                    COMPONENT DEPENDENCY MATRIX                                │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│ Component          │ Depends On              │ Required For        │ Status   │
├────────────────────┼─────────────────────────┼─────────────────────┼──────────┤
│ WebbOS Browser     │ libwayland-client       │ -                   │ Core     │
│                    │ libEGL (Mesa)           │                     │          │
│                    │ libGLESv2 (Mesa)        │                     │          │
│                    │ Skia/Vulkan             │                     │          │
├────────────────────┼─────────────────────────┼─────────────────────┼──────────┤
│ Wayland Compositor │ libwayland-server       │ WebbOS Browser      │ Core     │
│ (Labwc)            │ libdrm                  │                     │          │
│                    │ libinput                │                     │          │
│                    │ libxkbcommon            │                     │          │
├────────────────────┼─────────────────────────┼─────────────────────┼──────────┤
│ Input Manager      │ libinput                │ WebbOS Browser      │ Core     │
│                    │ libudev                 │                     │          │
├────────────────────┼─────────────────────────┼─────────────────────┼──────────┤
│ Mesa GPU Driver    │ Kernel DRM (vc4)        │ All graphics        │ Core     │
│ (V3D)              │                         │                     │          │
├────────────────────┼─────────────────────────┼─────────────────────┼──────────┤
│ DRM/KMS Driver     │ Kernel 6.6+             │ Mesa, Compositor    │ Core     │
│ (vc4)              │ VideoCore VII firmware  │                     │          │
├────────────────────┼─────────────────────────┼─────────────────────┼──────────┤
│ Display Output     │ DRM/KMS                 │ Visual output       │ Core     │
│ (HDMI/DSI)         │ RP1 I/O controller      │                     │          │
├────────────────────┼─────────────────────────┼─────────────────────┼──────────┤
│ Input Devices      │ Kernel evdev            │ Input Manager       │ Core     │
│ (USB/GPIO/I2C)     │ RP1 USB xHCI            │                     │          │
│                    │ RP1 I2C/GPIO            │                     │          │
├────────────────────┼─────────────────────────┼─────────────────────┼──────────┤
│ XWayland (opt)     │ Xorg server             │ Legacy X11 apps     │ Optional │
│                    │ Wayland compositor      │                     │          │
└────────────────────┴─────────────────────────┴─────────────────────┴──────────┘

Legend:
  [Core]    - Required for basic functionality
  [Optional] - Required only for specific use cases
```

---

## 8. Testing Checklist

### 8.1 Unit Testing

- [ ] **DRM Driver Tests**
  - [ ] Device enumeration
  - [ ] Mode setting
  - [ ] Page flipping
  - [ ] Atomic commits
  - [ ] Hardware planes

- [ ] **Input Driver Tests**
  - [ ] USB HID event parsing
  - [ ] Touch event handling
  - [ ] GPIO button mapping
  - [ ] Multi-touch protocol
  - [ ] Gesture recognition

- [ ] **Graphics Library Tests**
  - [ ] EGL context creation
  - [ ] OpenGL ES 3.1 features
  - [ ] Shader compilation
  - [ ] Buffer allocation
  - [ ] Texture upload

### 8.2 Integration Testing

- [ ] **Display Pipeline**
  - [ ] Application → Wayland → Compositor → DRM → Display
  - [ ] Surface allocation and destruction
  - [ ] Damage tracking
  - [ ] VSync handling
  - [ ] Multi-display

- [ ] **Input Pipeline**
  - [ ] Hardware → Kernel → libinput → Application
  - [ ] Event routing
  - [ ] Focus management
  - [ ] Gesture propagation

- [ ] **End-to-End Scenarios**
  - [ ] Browser launch and navigation
  - [ ] Video playback
  - [ ] Touch interaction
  - [ ] Keyboard/mouse input
  - [ ] Window management

### 8.3 System Testing

- [ ] **Boot Sequence**
  - [ ] DRM driver load
  - [ ] Compositor start
  - [ ] Application launch
  - [ ] Input device detection

- [ ] **Long-Running Tests**
  - [ ] 24-hour stability
  - [ ] Memory leak detection
  - [ ] Thermal stability
  - [ ] Performance consistency

- [ ] **Stress Tests**
  - [ ] Maximum GPU load
  - [ ] Maximum input rate
  - [ ] Thermal limits
  - [ ] Memory pressure

---

## 9. Deliverables Summary

| Deliverable | Location | Status |
|-------------|----------|--------|
| Implementation Plan | `/docs/gui-input/implementation-plan.md` | ✅ This document |
| Architecture Diagrams | Embedded above | ✅ ASCII diagrams |
| Dependency Matrix | Section 7 | ✅ Completed |
| Testing Checklist | Section 8 | ✅ Completed |
| Risk Mitigation | Section 6.4 | ✅ Strategies defined |

**Related Documents:**
- Sofia's Research: `/docs/gui-input/Week5_GUI_Input_Subsystems_Research.md`
- Executive Summary: `/docs/gui-input/Week5_Executive_Summary.md`
- Ingrid's Architecture: `/docs/gui-input/implementation/`

---

## 10. Next Steps

1. **Review** this implementation plan with the development team
2. **Set up** Raspberry Pi 5 development hardware
3. **Begin** Phase 1: DRM abstraction layer implementation
4. **Establish** continuous integration for cross-compilation
5. **Schedule** weekly hardware testing sessions

---

*Document prepared by: Sofia La Savante*  
*Research & Architecture Specialist*  
*Date: February 15, 2026*

*"Research thoroughly, plan carefully, execute precisely."*
