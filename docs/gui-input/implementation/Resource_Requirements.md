# Resource Requirements: GUI & Input Subsystems

**Document:** Hardware Resource Specification  
**Architect:** Ingrid L'Ingénieure  
**Date:** February 15, 2026

---

## 1. Memory Requirements

### 1.1 Contiguous Memory Allocator (CMA)

**CMA is the GPU memory pool for Pi 5. Unlike Pi 4, there's no fixed split.**

#### Recommended CMA Sizes by Configuration

| Configuration | CMA Size | Use Case |
|--------------|----------|----------|
| Minimal (720p) | 128MB | Headless + basic GUI |
| Standard (1080p) | 256MB | **Recommended default** |
| High-End (4K) | 320MB | 4K desktop |
| Maximum | 512MB | 4K + video decode |

#### Configuration Method

```bash
# Option 1: Device Tree Overlay (recommended)
# /boot/firmware/config.txt
dtoverlay=vc4-kms-v3d,cma-256

# Option 2: Kernel Command Line
# /boot/firmware/cmdline.txt
cma=256M

# Option 3: Dynamic (if needed)
echo 268435456 > /sys/kernel/mm/cma/cma_size  # 256MB in bytes
```

### 1.2 Memory Breakdown by Component

#### 1080p60 Configuration (256MB CMA)

```
┌────────────────────────────────────────────────────────────┐
│ CMA Memory Allocation (256 MB total)                       │
├────────────────────────────────────────────────────────────┤
│ Framebuffers (48 MB)                                       │
│   ├─ Primary framebuffer: 1920 × 1080 × 4 bytes = 8.3 MB   │
│   ├─ Secondary buffer (double buffering): 8.3 MB           │
│   ├─ Compositor surfaces: 16 MB                            │
│   └─ Scratch buffers: 15 MB                                │
├────────────────────────────────────────────────────────────┤
│ GPU Working Memory (128 MB)                                │
│   ├─ Texture cache: 64 MB                                  │
│   ├─ Render targets: 32 MB                                 │
│   ├─ Vertex/Index buffers: 16 MB                           │
│   └─ Command buffers: 16 MB                                │
├────────────────────────────────────────────────────────────┤
│ Mesa Driver Overhead (48 MB)                               │
│   ├─ Shader cache: 32 MB                                   │
│   ├─ GPU BO cache: 12 MB                                   │
│   └─ Driver structures: 4 MB                               │
├────────────────────────────────────────────────────────────┤
│ Reserved/Free (32 MB)                                      │
│   └─ Available for video decode or spikes                  │
└────────────────────────────────────────────────────────────┘
```

#### 4K60 Configuration (512MB CMA)

```
┌────────────────────────────────────────────────────────────┐
│ CMA Memory Allocation (512 MB total)                       │
├────────────────────────────────────────────────────────────┤
│ Framebuffers (192 MB)                                      │
│   ├─ Primary framebuffer: 3840 × 2160 × 4 bytes = 33 MB    │
│   ├─ Secondary buffer: 33 MB                               │
│   ├─ Tertiary buffer (triple buffering): 33 MB             │
│   ├─ Compositor surfaces: 64 MB                            │
│   └─ Scratch buffers: 29 MB                                │
├────────────────────────────────────────────────────────────┤
│ GPU Working Memory (256 MB)                                │
│   ├─ Texture cache: 128 MB                                 │
│   ├─ Render targets: 64 MB                                 │
│   ├─ Vertex/Index buffers: 32 MB                           │
│   └─ Command buffers: 32 MB                                │
├────────────────────────────────────────────────────────────┤
│ Mesa Driver Overhead (48 MB)                               │
│   ├─ Shader cache: 32 MB                                   │
│   ├─ GPU BO cache: 12 MB                                   │
│   └─ Driver structures: 4 MB                               │
├────────────────────────────────────────────────────────────┤
│ Reserved/Free (16 MB)                                      │
└────────────────────────────────────────────────────────────┘
```

### 1.3 System RAM Requirements

#### WebbOS System Memory

| Component | 1080p Mode | 4K Mode | Notes |
|-----------|------------|---------|-------|
| Kernel | 32 MB | 32 MB | Fixed overhead |
| Browser Engine | 64 MB | 128 MB | Page cache, JS heap |
| Desktop Shell | 16 MB | 16 MB | HTML/CSS/JS runtime |
| Compositor | 16 MB | 32 MB | Wayland/Labwc |
| Mesa Libraries | 8 MB | 8 MB | Shared libraries |
| Input Subsystem | 4 MB | 4 MB | libinput, drivers |
| Network Stack | 8 MB | 8 MB | Buffers, TLS |
| **Total System** | **~148 MB** | **~248 MB** | Without CMA |

#### Total Memory Requirements

| Configuration | System RAM | CMA | Total | Notes |
|--------------|------------|-----|-------|-------|
| 1080p Basic | 256 MB | 128 MB | 384 MB | Minimum viable |
| 1080p Standard | 512 MB | 256 MB | 768 MB | **Recommended** |
| 1080p Premium | 512 MB | 320 MB | 832 MB | With video |
| 4K Standard | 1 GB | 320 MB | 1.32 GB | 4K desktop |
| 4K Premium | 2 GB | 512 MB | 2.5 GB | Full features |

**Recommendation:** Raspberry Pi 5 with 4GB RAM for 1080p, 8GB for 4K.

### 1.4 Memory Optimization Strategies

#### Dynamic Memory Management

```rust
// Memory pressure handling
pub struct MemoryManager {
    cma_allocator: CmaAllocator,
    pressure_level: PressureLevel,
}

impl MemoryManager {
    /// Handle memory pressure
    pub fn on_pressure(&mut self, level: PressureLevel) {
        match level {
            PressureLevel::Low => {
                // Normal operation
            }
            PressureLevel::Medium => {
                // Reduce texture cache
                self.reduce_cache_size(0.75);
            }
            PressureLevel::High => {
                // Aggressive cleanup
                self.reduce_cache_size(0.5);
                self.drop_unused_buffers();
                self.enable_aggressive_gc();
            }
            PressureLevel::Critical => {
                // Emergency mode
                self.drop_all_caches();
                self.reduce_framebuffer_count(1); // Single buffer
            }
        }
    }
}
```

#### Texture Compression

| Format | Compression | Quality | Use Case |
|--------|-------------|---------|----------|
| Uncompressed RGBA32 | 1:1 | Best | UI elements, text |
| DXT1/BC1 | 8:1 | Good | Opaque textures |
| DXT5/BC3 | 4:1 | Good | Transparent textures |
| ETC2 | 4:1 | Good | GLES standard |
| ASTC | Variable | Excellent | Modern standard |

**Recommendation:** Use ETC2 for general textures (4:1 compression), uncompressed for UI.

---

## 2. CPU Requirements

### 2.1 Cortex-A76 Specifications

**Raspberry Pi 5 CPU:**
- 4× Cortex-A76 @ 2.4 GHz
- 64KB L1 I-cache, 64KB L1 D-cache per core
- 512KB L2 cache per core
- 2MB L3 cache (shared)

### 2.2 CPU Usage by Task

#### Normal Desktop Operation (1080p60)

| Task | CPU Usage | Core | Frequency |
|------|-----------|------|-----------|
| Compositor (Labwc) | 3-5% | Any | 1.5 GHz |
| Browser rendering | 10-20% | 1-2 cores | 1.5-2.0 GHz |
| Input processing | 1-2% | Any | 1.5 GHz |
| JavaScript execution | 5-15% | 1 core | 2.0-2.4 GHz |
| Network I/O | 2-5% | Any | 1.5 GHz |
| **Total Average** | **25-35%** | Spread | Variable |

#### Heavy Load Scenario (4K60 + Complex Page)

| Task | CPU Usage | Core | Frequency |
|------|-----------|------|-----------|
| Compositor | 5-10% | Any | 2.0 GHz |
| Browser rendering | 25-40% | 2 cores | 2.4 GHz |
| Input processing | 2-3% | Any | 2.0 GHz |
| JavaScript execution | 15-30% | 1-2 cores | 2.4 GHz |
| Thermal management | <1% | Any | 2.0 GHz |
| **Total Average** | **50-70%** | All cores | 2.4 GHz |

### 2.3 Threading Model

```
┌─────────────────────────────────────────────────────────────────┐
│                      WebbOS Thread Layout                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Core 0: System Services                                        │
│   ├─ Input polling thread (1ms intervals)                       │
│   ├─ Thermal monitoring (100ms intervals)                       │
│   └─ IPC/message handling                                       │
│                                                                 │
│  Core 1: Browser Engine                                         │
│   ├─ HTML/CSS parsing                                           │
│   ├─ Layout computation                                         │
│   └─ JavaScript execution (main thread)                         │
│                                                                 │
│  Core 2: Rendering                                              │
│   ├─ Display list building                                      │
│   ├─ Skia command generation                                    │
│   └─ GPU command submission                                     │
│                                                                 │
│  Core 3: Network & I/O                                          │
│   ├─ Network packet processing                                  │
│   ├─ TLS encryption/decryption                                  │
│   ├─ File system operations                                     │
│   └─ Background tasks                                           │
│                                                                 │
│  Any Core: Wayland compositor (event-driven)                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2.4 CPU Frequency Scaling

**Governor Strategy:**

```
┌─────────────────────────────────────────────────────────────────┐
│                  CPU Frequency Governor                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Load < 30%:   1.5 GHz (powersave)                              │
│  Load 30-60%:  2.0 GHz (balanced)                               │
│  Load > 60%:   2.4 GHz (performance)                            │
│                                                                 │
│  Override conditions:                                           │
│   - Thermal throttling active: Max 1.8 GHz                      │
│   - Video playback: Lock at 2.0 GHz (avoid stutter)             │
│   - User input: Boost to 2.4 GHz for 500ms (responsiveness)     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. GPU Requirements

### 3.1 VideoCore VII Specifications

| Specification | Value |
|--------------|-------|
| Architecture | Broadcom VideoCore VII |
| Process | 16nm |
| Clock Speed | 800 MHz (configurable) |
| Compute | OpenGL ES 3.1, Vulkan 1.3 |
| 3D Block | V3D 7.x |
| Video Decode | HEVC 4Kp60 |
| HVS | Hardware Video Scaler |

### 3.2 GPU Performance by Resolution

#### Frame Time Budget (60 FPS = 16.67ms per frame)

| Resolution | Pixels/Frame | GPU Time (typical) | Headroom |
|------------|--------------|-------------------|----------|
| 720p | 0.92 MP | 3-5 ms | 70% |
| 1080p | 2.07 MP | 6-10 ms | 40-60% |
| 1440p | 3.69 MP | 10-14 ms | 15-40% |
| 4K | 8.29 MP | 12-16 ms | 0-25% |

#### GPU Clock Requirements

| Resolution | Target FPS | GPU Clock | Thermal Impact |
|------------|------------|-----------|----------------|
| 1080p | 60 | 400-600 MHz | Low |
| 1080p | 60 + effects | 600-800 MHz | Medium |
| 4K | 30 | 400-600 MHz | Low |
| 4K | 60 | 800 MHz | High |

### 3.3 GPU Bandwidth Requirements

#### Memory Bandwidth Calculation

```
┌─────────────────────────────────────────────────────────────────┐
│              Bandwidth Usage at 1080p60                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Framebuffer scanout:                                            │
│   1920 × 1080 × 4 bytes × 60 Hz = 497 MB/s                       │
│                                                                 │
│  Double buffering:                                               │
│   497 MB/s × 2 = 994 MB/s                                        │
│                                                                 │
│  Compositor overhead (damage tracking):                          │
│   ~200 MB/s (estimated)                                          │
│                                                                 │
│  Texture sampling (UI elements):                                 │
│   ~500 MB/s                                                      │
│                                                                 │
│  Total estimated: ~1.7 GB/s                                      │
│                                                                 │
│  Available bandwidth: ~34 GB/s (LPDDR4X-4267)                    │
│  GPU usage: ~5%                                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### 4K Bandwidth

| Operation | Bandwidth | Notes |
|-----------|-----------|-------|
| Framebuffer scanout | 1.98 GB/s | Single buffer |
| Double buffering | 3.96 GB/s | Standard config |
| Triple buffering | 5.94 GB/s | Smoothness mode |
| Compositor + textures | 2-4 GB/s | Depends on scene |
| **Total 4K60** | **~10 GB/s** | ~30% of available |

### 3.4 GPU Optimization Guidelines

#### Rendering Best Practices

```rust
// optimization/gpu_best_practices.rs

pub struct GpuOptimizer;

impl GpuOptimizer {
    /// Minimize state changes
    pub fn sort_draw_calls(draws: &mut [DrawCall]) {
        // Sort by shader program
        // Sort by texture
        // Sort by blend mode
        draws.sort_by_key(|d| (d.shader_id, d.texture_id, d.blend_mode));
    }
    
    /// Use scissor rectangles for partial updates
    pub fn enable_scissor(rect: Rect) {
        // glEnable(GL_SCISSOR_TEST)
        // glScissor(rect.x, rect.y, rect.w, rect.h)
    }
    
    /// Batch small draw calls
    pub fn batch_draws(draws: &[DrawCall]) -> Vec<Batch> {
        // Group non-overlapping draws
        // Use instancing where applicable
    }
    
    /// Use hardware planes
    pub fn assign_to_planes(layers: &[Layer]) -> PlaneAssignment {
        // Primary plane: Main web content
        // Overlay plane: Video element
        // Cursor plane: Mouse pointer
    }
}
```

---

## 4. Thermal Requirements

### 4.1 Thermal Specifications

| Temperature | State | Action | Performance Impact |
|-------------|-------|--------|-------------------|
| < 60°C | Optimal | Full performance | None |
| 60-70°C | Normal | Monitor | None |
| 70-80°C | Warm | Prepare throttling | None yet |
| 80-85°C | Soft throttle | Reduce frequency | -100MHz/°C |
| > 85°C | Hard throttle | Emergency measures | Severe |

### 4.2 Cooling Options Comparison

| Solution | Cost | Effectiveness | Noise | Recommended For |
|----------|------|---------------|-------|-----------------|
| Passive (case only) | $0 | Poor | Silent | Light use only |
| Heatsink only | $5 | Fair | Silent | 1080p desktop |
| Active Cooler | $5 | Good | Low | **Recommended** |
| Case with fan | $15 | Good | Low | Clean setup |
| Custom cooling | $30+ | Excellent | Variable | Overclocking |

### 4.3 Thermal Design Power (TDP)

#### Power Consumption by Scenario

| Scenario | SoC Power | Display | Total | Cooling Required |
|----------|-----------|---------|-------|------------------|
| Idle | 2.5W | 0.5W | 3W | Passive OK |
| 1080p60 desktop | 5W | 1W | 6W | Heatsink OK |
| 1080p60 + web browsing | 7W | 1W | 8W | Active cooler |
| 4K60 desktop | 8W | 2W | 10W | Active cooler |
| Maximum stress | 12W | 2W | 14W | Active + airflow |

### 4.4 Thermal Management Implementation

```rust
// thermal/manager.rs

pub struct ThermalManager {
    temp_sensor: TempSensor,
    cooling_policy: CoolingPolicy,
    performance_state: PerformanceState,
}

pub struct PerformanceState {
    cpu_freq: u32,      // MHz
    gpu_freq: u32,      // MHz
    target_fps: u32,    // FPS cap
    quality_level: QualityLevel,
}

impl ThermalManager {
    /// Main monitoring loop
    pub fn monitor(&mut self) {
        let temp = self.temp_sensor.read();
        
        match temp {
            t if t < 70.0 => self.set_state(PerformanceState {
                cpu_freq: 2400,
                gpu_freq: 800,
                target_fps: 60,
                quality_level: QualityLevel::High,
            }),
            t if t < 80.0 => self.set_state(PerformanceState {
                cpu_freq: 2200,
                gpu_freq: 700,
                target_fps: 60,
                quality_level: QualityLevel::High,
            }),
            t if t < 85.0 => self.set_state(PerformanceState {
                cpu_freq: 1800,
                gpu_freq: 600,
                target_fps: 60,
                quality_level: QualityLevel::Medium,
            }),
            _ => self.set_state(PerformanceState {
                cpu_freq: 1500,
                gpu_freq: 400,
                target_fps: 30,
                quality_level: QualityLevel::Low,
            }),
        }
    }
    
    /// Apply performance state
    fn set_state(&mut self, state: PerformanceState) {
        // Set CPU frequency
        self.set_cpu_freq(state.cpu_freq);
        
        // Set GPU frequency
        self.set_gpu_freq(state.gpu_freq);
        
        // Adjust rendering quality
        self.browser.set_quality(state.quality_level);
        
        // Cap frame rate if needed
        self.compositor.set_target_fps(state.target_fps);
    }
}
```

---

## 5. Storage Requirements

### 5.1 Disk Space

| Component | Size | Notes |
|-----------|------|-------|
| Mesa libraries | ~50 MB | libEGL, libGLESv2, libdrm |
| Wayland compositor | ~5 MB | Labwc binary |
| Wayland libraries | ~10 MB | libwayland-client, etc. |
| GPU firmware | ~20 MB | VideoCore VII firmware |
| WebbOS browser | ~30 MB | Binary + resources |
| **Total** | **~115 MB** | Installed size |

### 5.2 Runtime Storage

| Cache Type | Size | Location | Persist? |
|------------|------|----------|----------|
| Shader cache | 32-64 MB | /var/cache/mesa | Yes |
| Texture cache | 16 MB | /tmp | No |
| Browser cache | 64 MB | /home/user/.cache | Yes |
| Compositor cache | 8 MB | /tmp | No |

---

## 6. Network Requirements

### 6.1 Bandwidth (for web browsing)

| Activity | Downstream | Upstream | Latency Req |
|----------|------------|----------|-------------|
| Static page | 100 KB/s | 10 KB/s | < 200ms |
| Dynamic content | 500 KB/s | 50 KB/s | < 100ms |
| Streaming video | 5-15 Mbps | 50 KB/s | < 50ms |
| Video calls | 2-4 Mbps | 1-2 Mbps | < 30ms |

### 6.2 TLS Overhead

| Cipher Suite | CPU Impact | Throughput |
|--------------|------------|------------|
| AES-128-GCM | Low (hardware accel) | 10 Gbps+ |
| ChaCha20-Poly1305 | Medium | 2-5 Gbps |
| AES-256-GCM | Low (hardware accel) | 8 Gbps+ |

---

## 7. Hardware Requirements Summary

### 7.1 Minimum Configuration

```
┌─────────────────────────────────────────────────────────────────┐
│                    Minimum Viable Configuration                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Raspberry Pi 5 (any RAM variant)                               │
│   └─ 4GB RAM recommended (2GB workable with swap)               │
│                                                                 │
│  Storage                                                        │
│   └─ 16GB SD card (Class 10 or better)                          │
│                                                                 │
│  Display                                                        │
│   └─ 720p or 1080p HDMI display                                 │
│                                                                 │
│  Input                                                          │
│   └─ USB keyboard (mouse optional with touch)                   │
│                                                                 │
│  Cooling                                                        │
│   └─ Basic heatsink (case heatsink acceptable)                  │
│                                                                 │
│  Power Supply                                                   │
│   └─ 5V 3A (15W) USB-C                                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 7.2 Recommended Configuration

```
┌─────────────────────────────────────────────────────────────────┐
│                    Recommended Configuration                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Raspberry Pi 5 4GB or 8GB                                      │
│   └─ 4GB for 1080p, 8GB for 4K or heavy multitasking            │
│                                                                 │
│  Storage                                                        │
│   └─ 32GB+ SD card (A2 rated) or USB 3.0 SSD                    │
│                                                                 │
│  Display                                                        │
│   └─ 1080p HDMI monitor (dual monitor support available)        │
│   └─ Optional: DSI touchscreen for kiosks/appliances            │
│                                                                 │
│  Input                                                          │
│   └─ USB keyboard and mouse                                     │
│   └─ OR: Capacitive touchscreen (GT911 or FT6236 based)         │
│   └─ Optional: Bluetooth peripherals                            │
│                                                                 │
│  Cooling                                                        │
│   └─ Raspberry Pi Active Cooler ($5)                            │
│   └─ OR: Case with integrated fan                               │
│                                                                 │
│  Power Supply                                                   │
│   └─ Official Raspberry Pi 27W PSU (5V 5A)                      │
│      └─ Enables 1.6A USB current, better overclocking           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 7.3 Maximum Configuration

```
┌─────────────────────────────────────────────────────────────────┐
│                    Maximum Performance Setup                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Raspberry Pi 5 8GB                                             │
│                                                                 │
│  Storage                                                        │
│   └─ NVMe SSD via PCIe HAT (for maximum I/O)                    │
│                                                                 │
│  Display                                                        │
│   └─ Dual 4K60 monitors (HDMI 0 and HDMI 1)                     │
│   └─ OR: 4K primary + DSI touchscreen secondary                 │
│                                                                 │
│  Input                                                          │
│   └─ Full USB HID setup (keyboard, mouse, gamepad)              │
│   └─ 10-point capacitive touchscreen                            │
│   └─ Bluetooth audio + input devices                            │
│                                                                 │
│  Cooling                                                        │
│   └─ High-performance heatsink with 40mm fan                    │
│   └─ Case with good airflow                                     │
│                                                                 │
│  Power Supply                                                   │
│   └─ Official Raspberry Pi 27W PSU                              │
│   └─ Powered USB hub for peripherals                            │
│                                                                 │
│  Overclocking (optional)                                        │
│   └─ CPU: 2.8-3.0 GHz (with adequate cooling)                   │
│   └─ GPU: 1.0 GHz                                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 8. Resource Monitoring

### 8.1 Monitoring Commands

```bash
# Temperature
vcgencmd measure_temp
watch -n 1 vcgencmd measure_temp

# Frequency scaling
vcgencmd get_throttled
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq

# Memory usage
vcgencmd get_mem arm
vcgencmd get_mem gpu
cat /proc/meminfo | grep Cma

# GPU load (if available)
cat /sys/kernel/debug/dri/0/v3d_stats

# Process memory
ps aux --sort=-%mem | head -20
```

### 8.2 Performance Metrics

| Metric | Good | Warning | Critical |
|--------|------|---------|----------|
| CPU Temp | < 70°C | 70-80°C | > 80°C |
| GPU Temp | < 70°C | 70-80°C | > 80°C |
| CPU Usage | < 50% | 50-80% | > 90% |
| Memory Usage | < 70% | 70-90% | > 95% |
| Frame Time | < 16ms | 16-20ms | > 20ms |
| CMA Free | > 20% | 10-20% | < 10% |

---

## 9. Summary Table

| Resource | Minimum | Recommended | Maximum |
|----------|---------|-------------|---------|
| RAM | 2 GB | 4 GB | 8 GB |
| CMA | 128 MB | 256 MB | 512 MB |
| Storage | 16 GB | 32 GB | 128 GB+ |
| Display | 720p | 1080p60 | Dual 4K60 |
| Cooling | Passive | Active Cooler | Custom |
| Power | 15W | 25W | 25W |

---

*"Effektivitet är nyckeln - we optimize for what we have."* - Ingrid L'Ingénieure
