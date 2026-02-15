# Testing Strategy: GUI & Input Subsystems

**Document:** Comprehensive Testing Plan  
**Architect:** Ingrid L'Ingénieure  
**Date:** February 15, 2026  
**Test Coverage Target:** 80% unit, 75% integration

---

## 1. Testing Philosophy

**"Testa allt, lita på inget"** (Test everything, trust nothing)

### Principles

1. **Automated First:** All tests must be automatable
2. **Hardware-in-Loop:** Test on real Pi 5 hardware, not just emulation
3. **Regression Protection:** Every bug gets a test
4. **Performance Baselines:** Track performance metrics over time
5. **Thermal Validation:** Test under thermal stress

---

## 2. Test Categories

```
┌─────────────────────────────────────────────────────────────────┐
│                    Testing Pyramid                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                      ▲                                          │
│                     ╱ ╲                                         │
│                    ╱ E2E╲          (5% of tests)                │
│                   ╱────────╲        Full system workflows       │
│                  ╱            ╲                                 │
│                 ╱  Integration  ╲  (15% of tests)               │
│                ╱──────────────────╲  Component interactions     │
│               ╱                      ╲                          │
│              ╱      Unit Tests         ╲ (80% of tests)         │
│             ╱────────────────────────────╲ Individual functions │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Unit Testing

### 3.1 DRM/KMS Unit Tests

```rust
// tests/unit/drm_tests.rs

#[cfg(test)]
mod drm_tests {
    use super::*;

    /// Test connector detection
    #[test]
    fn test_connector_parsing() {
        let raw_connector = vec![
            0x01, 0x00, 0x00, 0x00, // connector_id = 1
            0x0A, 0x00, 0x00, 0x00, // connector_type = HDMI-A
            0x01, 0x00, 0x00, 0x00, // connection = connected
            0x00, 0x00, 0x00, 0x00, // mm_width
            0x00, 0x00, 0x00, 0x00, // mm_height
        ];
        
        let connector = DrmConnector::parse(&raw_connector).unwrap();
        
        assert_eq!(connector.id, 1);
        assert_eq!(connector.connector_type, ConnectorType::HdmiA);
        assert!(connector.is_connected());
    }

    /// Test mode validation
    #[test]
    fn test_mode_selection() {
        let modes = vec![
            Mode { 
                hdisplay: 1920, vdisplay: 1080, 
                vrefresh: 60, flags: DRM_MODE_FLAG_PHSYNC 
            },
            Mode { 
                hdisplay: 3840, vdisplay: 2160, 
                vrefresh: 30, flags: DRM_MODE_FLAG_PHSYNC 
            },
            Mode { 
                hdisplay: 1280, vdisplay: 720, 
                vrefresh: 60, flags: DRM_MODE_FLAG_PHSYNC 
            },
        ];
        
        // Should select 1080p60 as optimal
        let selected = select_optimal_mode(&modes, None);
        assert_eq!(selected.hdisplay, 1920);
        assert_eq!(selected.vdisplay, 1080);
        assert_eq!(selected.vrefresh, 60);
    }

    /// Test atomic request building
    #[test]
    fn test_atomic_request_building() {
        let mut req = AtomicRequest::new();
        
        req.set_property(1, "CRTC_ID", 0);  // connector -> crtc
        req.set_property(0, "MODE_ID", 1);  // crtc -> mode
        req.set_property(0, "ACTIVE", 1);   // crtc -> active
        
        assert_eq!(req.property_count(), 3);
        assert!(req.has_property(1, "CRTC_ID"));
    }

    /// Test framebuffer size calculation
    #[test]
    fn test_framebuffer_size() {
        let fb = FramebufferInfo {
            width: 1920,
            height: 1080,
            bpp: 32,
            pitch: 1920 * 4,
        };
        
        assert_eq!(fb.size(), 1920 * 1080 * 4);
        assert_eq!(fb.pitch(), 7680);
    }
}
```

### 3.2 GPU Driver Unit Tests

```rust
// tests/unit/gpu_tests.rs

#[cfg(test)]
mod gpu_tests {
    use super::*;

    /// Test V3D command buffer building
    #[test]
    fn test_v3d_command_buffer() {
        let mut builder = V3dCommandBuilder::new();
        
        builder.add_render_target(RenderTarget {
            width: 1920,
            height: 1080,
            format: PixelFormat::Rgba8Unorm,
        });
        
        builder.add_draw_call(DrawCall {
            vertex_count: 6,
            instance_count: 1,
            shader: ShaderId(1),
        });
        
        let buffer = builder.build();
        
        assert!(buffer.validate().is_ok());
        assert_eq!(buffer.command_count(), 2);
    }

    /// Test shader compilation cache
    #[test]
    fn test_shader_cache() {
        let mut cache = ShaderCache::new(1024 * 1024); // 1MB cache
        
        let source = b"vertex shader source code";
        let hash = hash_shader(source);
        
        // First lookup - miss
        assert!(cache.get(&hash).is_none());
        
        // Insert
        let binary = vec![0xDE, 0xAD, 0xBE, 0xEF];
        cache.insert(hash, binary.clone());
        
        // Second lookup - hit
        assert_eq!(cache.get(&hash), Some(&binary));
    }

    /// Test GEM buffer allocation
    #[test]
    fn test_gem_buffer_lifecycle() {
        let allocator = FakeGemAllocator::new(256 * 1024 * 1024);
        
        // Allocate buffer
        let handle = allocator.allocate(1024 * 1024).unwrap();
        assert!(allocator.is_valid(handle));
        
        // Free buffer
        allocator.free(handle).unwrap();
        assert!(!allocator.is_valid(handle));
    }

    /// Test buffer alignment requirements
    #[test]
    fn test_buffer_alignment() {
        let allocator = GemAllocator::new();
        
        // 4KB alignment required
        let buf1 = allocator.allocate(100).unwrap();
        assert_eq!(buf1.offset % 4096, 0);
        
        // Larger buffers need larger alignment
        let buf2 = allocator.allocate(1024 * 1024).unwrap();
        assert_eq!(buf2.offset % 4096, 0);
    }
}
```

### 3.3 Input System Unit Tests

```rust
// tests/unit/input_tests.rs

#[cfg(test)]
mod input_tests {
    use super::*;

    /// Test touch coordinate transformation
    #[test]
    fn test_touch_coordinate_transform() {
        let calibration = TouchCalibration {
            touch_min_x: 0,
            touch_max_x: 800,
            touch_min_y: 0,
            touch_max_y: 480,
            screen_width: 1920,
            screen_height: 1080,
            rotation: Rotation::None,
        };
        
        // Center touch -> center screen
        let (sx, sy) = calibration.transform(400, 240);
        assert_eq!(sx, 960);
        assert_eq!(sy, 540);
        
        // Corner touches
        let (tlx, tly) = calibration.transform(0, 0);
        assert_eq!((tlx, tly), (0, 0));
        
        let (brx, bry) = calibration.transform(800, 480);
        assert_eq!((brx, bry), (1920, 1080));
    }

    /// Test multi-touch tracking
    #[test]
    fn test_multitouch_tracking() {
        let mut tracker = TouchTracker::new();
        
        // First touch
        let events = tracker.process_event(TouchEvent::Down {
            id: 0,
            x: 100,
            y: 200,
        });
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, 0);
        
        // Second touch
        let events = tracker.process_event(TouchEvent::Down {
            id: 1,
            x: 300,
            y: 400,
        });
        assert_eq!(events.len(), 1);
        
        // Move first touch
        let events = tracker.process_event(TouchEvent::Move {
            id: 0,
            x: 150,
            y: 250,
        });
        assert_eq!(events[0].id, 0);
        
        // Release second touch
        let events = tracker.process_event(TouchEvent::Up { id: 1 });
        assert!(tracker.get_touch(1).is_none());
    }

    /// Test gesture recognition
    #[test]
    fn test_gesture_recognition() {
        let mut recognizer = GestureRecognizer::new();
        
        // Simulate tap
        recognizer.add_event(TouchEvent::Down { id: 0, x: 100, y: 100 });
        recognizer.add_event(TouchEvent::Up { id: 0 });
        
        assert_eq!(recognizer.recognize(), Some(Gesture::Tap { x: 100, y: 100 }));
        
        // Simulate swipe
        recognizer.reset();
        recognizer.add_event(TouchEvent::Down { id: 0, x: 0, y: 0 });
        recognizer.add_event(TouchEvent::Move { id: 0, x: 100, y: 0 });
        recognizer.add_event(TouchEvent::Move { id: 0, x: 200, y: 0 });
        recognizer.add_event(TouchEvent::Up { id: 0 });
        
        match recognizer.recognize() {
            Some(Gesture::Swipe { dx, .. }) => assert!(dx > 100),
            _ => panic!("Expected swipe gesture"),
        }
    }

    /// Test HID report parsing
    #[test]
    fn test_hid_keyboard_report() {
        let report = HidReport {
            descriptor: KEYBOARD_DESCRIPTOR,
            data: vec![0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00],
        };
        
        let parser = HidReportParser::new(&report.descriptor);
        let events = parser.parse(&report.data);
        
        // 0x04 = 'a' key in USB HID
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], HidEvent::Key { 
            keycode: 0x04, 
            pressed: true 
        });
    }

    /// Test key mapping
    #[test]
    fn test_key_mapping() {
        let mut mapper = KeyMapper::new_us();
        
        assert_eq!(
            mapper.map(0x04), // USB HID 'a'
            Some(Key::Character('a'))
        );
        
        assert_eq!(
            mapper.map(0x28), // USB HID Return
            Some(Key::Return)
        );
        
        // With shift
        mapper.set_modifier(Modifier::Shift);
        assert_eq!(
            mapper.map(0x04), // USB HID 'A' with shift
            Some(Key::Character('A'))
        );
    }
}
```

### 3.4 EGL/OpenGL ES Unit Tests

```rust
// tests/unit/egl_tests.rs

#[cfg(test)]
mod egl_tests {
    use super::*;

    /// Test EGL configuration selection
    #[test]
    fn test_egl_config_selection() {
        let configs = vec![
            EglConfig {
                id: 1,
                red_size: 8, green_size: 8, blue_size: 8, alpha_size: 0,
                depth_size: 24, stencil_size: 8,
                renderable_type: EGL_OPENGL_ES3_BIT,
            },
            EglConfig {
                id: 2,
                red_size: 8, green_size: 8, blue_size: 8, alpha_size: 8,
                depth_size: 24, stencil_size: 8,
                renderable_type: EGL_OPENGL_ES3_BIT,
            },
        ];
        
        // Should select config with alpha for compositing
        let requirements = EglRequirements {
            need_alpha: true,
            min_depth: 24,
            target_api: Api::OpenGlEs31,
        };
        
        let selected = select_config(&configs, &requirements).unwrap();
        assert_eq!(selected.id, 2);
        assert_eq!(selected.alpha_size, 8);
    }

    /// Test surface attribute validation
    #[test]
    fn test_surface_attributes() {
        let attribs = SurfaceAttributes {
            width: 1920,
            height: 1080,
            format: PixelFormat::Rgba8Unorm,
            swap_interval: 1,
        };
        
        assert!(attribs.validate().is_ok());
        
        // Invalid dimensions
        let invalid = SurfaceAttributes {
            width: 0,
            height: 1080,
            format: PixelFormat::Rgba8Unorm,
            swap_interval: 1,
        };
        assert!(invalid.validate().is_err());
    }
}
```

---

## 4. Integration Testing

### 4.1 Display Pipeline Integration

```rust
// tests/integration/display_pipeline.rs

#[test]
#[ignore = "requires hardware"]
fn test_full_display_pipeline() {
    // 1. Initialize DRM
    let drm = DrmDevice::open("/dev/dri/card0")
        .expect("Failed to open DRM device");
    
    // 2. Find connected display
    let connector = drm.find_connected_connector()
        .expect("No connected display found");
    
    let mode = connector.preferred_mode();
    println!("Using mode: {}x{}@{}Hz", 
        mode.hdisplay, mode.vdisplay, mode.vrefresh);
    
    // 3. Allocate framebuffer
    let fb_info = FramebufferInfo {
        width: mode.hdisplay,
        height: mode.vdisplay,
        bpp: 32,
        pitch: mode.hdisplay * 4,
    };
    let fb = drm.create_framebuffer(&fb_info)
        .expect("Failed to create framebuffer");
    
    // 4. Set mode
    drm.set_mode(&connector, &mode, &fb)
        .expect("Failed to set mode");
    
    // 5. Clear to red
    let buffer = drm.map_framebuffer(&fb);
    fill_buffer(buffer, &fb_info, Color::RED);
    
    // 6. Page flip
    drm.page_flip(&fb, PageFlipFlags::NONE)
        .expect("Page flip failed");
    
    // 7. Verify (visual inspection needed)
    thread::sleep(Duration::from_secs(2));
    
    // Cleanup
    drm.unset_mode(&connector)
        .expect("Failed to unset mode");
}

#[test]
#[ignore = "requires hardware"]
fn test_double_buffering() {
    let drm = DrmDevice::open("/dev/dri/card0").unwrap();
    let connector = drm.find_connected_connector().unwrap();
    let mode = connector.preferred_mode();
    
    // Create two framebuffers
    let fb1 = drm.create_framebuffer_sized(mode.hdisplay, mode.vdisplay);
    let fb2 = drm.create_framebuffer_sized(mode.hdisplay, mode.vdisplay);
    
    // Initial setup
    drm.set_mode(&connector, &mode, &fb1).unwrap();
    
    // Test multiple page flips
    for i in 0..60 {
        let fb = if i % 2 == 0 { &fb1 } else { &fb2 };
        let color = if i % 2 == 0 { Color::RED } else { Color::BLUE };
        
        fill_buffer(drm.map_framebuffer(fb), &fb.info, color);
        
        let start = Instant::now();
        drm.page_flip(fb, PageFlipFlags::NONE).unwrap();
        drm.wait_vblank().unwrap();
        
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(20), 
            "Frame time exceeded 20ms: {:?}", elapsed);
    }
}
```

### 4.2 Input Integration Tests

```rust
// tests/integration/input_integration.rs

#[test]
#[ignore = "requires hardware"]
fn test_usb_keyboard_input() {
    let mut input_manager = InputManager::new();
    
    // Connect to evdev
    let keyboard = EvdevDevice::open("/dev/input/event0")
        .expect("Failed to open keyboard device");
    
    input_manager.add_device(keyboard);
    
    println!("Press 'A' key within 5 seconds...");
    
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        let events = input_manager.poll_events();
        
        for event in events {
            if let InputEvent::Key { keycode, pressed, .. } = event {
                if keycode == KEY_A && pressed {
                    return; // Test passed
                }
            }
        }
        
        thread::sleep(Duration::from_millis(10));
    }
    
    panic!("Did not receive 'A' key press");
}

#[test]
#[ignore = "requires hardware"]
fn test_touchscreen_basic() {
    let touch = TouchscreenDriver::detect()
        .expect("No touchscreen detected");
    
    println!("Touch the screen within 10 seconds...");
    
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        let points = touch.read_touches();
        
        if !points.is_empty() {
            println!("Detected {} touch points:", points.len());
            for (i, point) in points.iter().enumerate() {
                println!("  Point {}: x={}, y={}, pressure={}",
                    i, point.x, point.y, point.pressure);
            }
            return; // Test passed
        }
        
        thread::sleep(Duration::from_millis(16));
    }
    
    panic!("No touch detected");
}

#[test]
#[ignore = "requires hardware"]
fn test_multitouch_gesture() {
    let mut recognizer = GestureRecognizer::new();
    let touch = TouchscreenDriver::detect().unwrap();
    
    println!("Perform a two-finger pinch gesture...");
    
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(15) {
        let points = touch.read_touches();
        
        for point in &points {
            recognizer.add_event(TouchEvent::Move {
                id: point.id as i32,
                x: point.x,
                y: point.y,
            });
        }
        
        if let Some(Gesture::Pinch { scale }) = recognizer.recognize() {
            println!("Detected pinch gesture with scale: {}", scale);
            assert!(scale > 0.5 && scale < 2.0);
            return;
        }
        
        thread::sleep(Duration::from_millis(16));
    }
    
    panic!("No pinch gesture detected");
}
```

### 4.3 Wayland Integration Tests

```rust
// tests/integration/wayland_integration.rs

#[test]
#[ignore = "requires wayland compositor"]
fn test_wayland_connection() {
    // 1. Connect to Wayland display
    let display = WlDisplay::connect("wayland-0")
        .expect("Failed to connect to Wayland");
    
    // 2. Get registry
    let registry = display.get_registry();
    
    // 3. Round-trip to sync
    display.roundtrip()
        .expect("Round-trip failed");
    
    // 4. Verify we have required globals
    let globals = registry.list_globals();
    assert!(globals.iter().any(|g| g.name == "wl_compositor"));
    assert!(globals.iter().any(|g| g.name == "xdg_wm_base"));
}

#[test]
#[ignore = "requires wayland compositor"]
fn test_wayland_window_creation() {
    let display = WlDisplay::connect("wayland-0").unwrap();
    let registry = display.get_registry();
    
    // Bind compositor
    let compositor = registry.bind::<WlCompositor>(1)
        .expect("Compositor not available");
    
    // Create surface
    let surface = compositor.create_surface();
    
    // Commit (required)
    surface.commit();
    
    // Sync and check for errors
    display.roundtrip().unwrap();
    
    // Verify surface was created
    assert!(surface.is_valid());
}

#[test]
#[ignore = "requires wayland compositor with EGL"]
fn test_egl_wayland_surface() {
    let display = WlDisplay::connect("wayland-0").unwrap();
    
    // Initialize EGL
    let egl_display = EglDisplay::initialize_wayland(&display)
        .expect("Failed to initialize EGL");
    
    // Create window surface
    let window = egl_display.create_window_surface(800, 600)
        .expect("Failed to create window");
    
    // Make current and clear
    egl_display.make_current(&window);
    
    unsafe {
        glClearColor(1.0, 0.0, 0.0, 1.0);
        glClear(GL_COLOR_BUFFER_BIT);
    }
    
    // Swap buffers
    egl_display.swap_buffers(&window)
        .expect("Swap buffers failed");
    
    // Visual verification needed
    thread::sleep(Duration::from_secs(2));
}
```

---

## 5. Performance Testing

### 5.1 Frame Rate Benchmarks

```rust
// tests/performance/fps_benchmark.rs

#[test]
#[ignore = "requires hardware"]
fn benchmark_1080p60_sustained() {
    let drm = DrmDevice::open("/dev/dri/card0").unwrap();
    let connector = drm.find_connected_connector().unwrap();
    
    // Force 1080p60
    let mode = connector.find_mode(1920, 1080, 60)
        .expect("1080p60 not supported");
    
    // Run benchmark
    let mut frame_times = Vec::new();
    let duration = Duration::from_secs(10);
    let start = Instant::now();
    
    while start.elapsed() < duration {
        let frame_start = Instant::now();
        
        // Simulate typical frame work
        render_test_scene(&drm);
        drm.page_flip(&fb, PageFlipFlags::NONE).unwrap();
        drm.wait_vblank().unwrap();
        
        frame_times.push(frame_start.elapsed());
    }
    
    // Analyze results
    let avg_frame_time = frame_times.iter().sum::<Duration>() / frame_times.len();
    let max_frame_time = frame_times.iter().max().unwrap();
    
    println!("Average frame time: {:?}", avg_frame_time);
    println!("Max frame time: {:?}", max_frame_time);
    println!("99th percentile: {:?}", percentile(&frame_times, 0.99));
    
    // Assertions
    assert!(avg_frame_time < Duration::from_millis(16),
        "Average frame time exceeded 16ms (60 FPS target)");
    assert!(max_frame_time < Duration::from_millis(20),
        "Max frame time exceeded 20ms");
}

#[test]
#[ignore = "requires hardware"]
fn benchmark_4k30() {
    let drm = DrmDevice::open("/dev/dri/card0").unwrap();
    let connector = drm.find_connected_connector().unwrap();
    
    let mode = connector.find_mode(3840, 2160, 30)
        .expect("4K30 not supported");
    
    // Similar benchmark as above but for 4K
    // Target: 33ms per frame (30 FPS)
    let results = run_benchmark(&drm, &mode, Duration::from_secs(10));
    
    assert!(results.avg_frame_time < Duration::from_millis(33),
        "4K30 performance failed");
}
```

### 5.2 Memory Performance

```rust
// tests/performance/memory_benchmark.rs

#[test]
fn benchmark_memory_allocation() {
    let mut allocator = CmaAllocator::new(256 * 1024 * 1024);
    
    // Allocate and free in various sizes
    let sizes = vec![
        4096,           // 4KB
        65536,          // 64KB
        1024 * 1024,    // 1MB
        8 * 1024 * 1024, // 8MB
    ];
    
    for size in &sizes {
        let start = Instant::now();
        let handles: Vec<_> = (0..100)
            .map(|_| allocator.allocate(*size).unwrap())
            .collect();
        let alloc_time = start.elapsed() / 100;
        
        let start = Instant::now();
        for handle in handles {
            allocator.free(handle).unwrap();
        }
        let free_time = start.elapsed() / 100;
        
        println!("Size {}: alloc={:?}, free={:?}", 
            size, alloc_time, free_time);
        
        // Assert reasonable times
        assert!(alloc_time < Duration::from_micros(100));
        assert!(free_time < Duration::from_micros(50));
    }
}

#[test]
#[ignore = "requires hardware"]
fn benchmark_texture_upload() {
    let gpu = GpuContext::new().unwrap();
    
    // Test texture sizes from small to large
    let sizes = vec![
        (256, 256),     // 256KB
        (1024, 1024),   // 4MB
        (2048, 2048),   // 16MB
        (4096, 4096),   // 64MB
    ];
    
    for (w, h) in sizes {
        let data = vec![0u8; w * h * 4];
        
        let start = Instant::now();
        let texture = gpu.create_texture(w, h, &data);
        let upload_time = start.elapsed();
        
        let bandwidth = (w * h * 4) as f64 / upload_time.as_secs_f64() / 1e9;
        
        println!("Texture {}x{}: {:?} ({:.2} GB/s)",
            w, h, upload_time, bandwidth);
    }
}
```

### 5.3 Input Latency

```rust
// tests/performance/input_latency.rs

#[test]
#[ignore = "requires hardware"]
fn benchmark_input_latency() {
    let mut input = InputManager::new();
    input.add_device(EvdevDevice::open("/dev/input/event0").unwrap());
    
    let mut latencies = Vec::new();
    let samples = 100;
    
    println!("Press any key 100 times...");
    
    while latencies.len() < samples {
        let poll_start = Instant::now();
        let events = input.poll_events();
        let poll_time = poll_start.elapsed();
        
        if !events.is_empty() {
            latencies.push(poll_time);
        }
        
        thread::sleep(Duration::from_micros(100));
    }
    
    let avg = latencies.iter().sum::<Duration>() / latencies.len();
    let max = *latencies.iter().max().unwrap();
    
    println!("Input latency - avg: {:?}, max: {:?}", avg, max);
    
    assert!(avg < Duration::from_millis(8),
        "Average input latency too high");
}
```

---

## 6. Thermal Testing

### 6.1 Thermal Stress Tests

```rust
// tests/thermal/thermal_stress.rs

#[test]
#[ignore = "requires hardware"]
fn test_thermal_stability() {
    let thermal = ThermalMonitor::new();
    let gpu = GpuContext::new().unwrap();
    
    // Monitor baseline
    let baseline_temp = thermal.read_temperature();
    println!("Baseline temp: {:.1}°C", baseline_temp);
    
    // Run GPU stress
    let stress_duration = Duration::from_secs(300); // 5 minutes
    let start = Instant::now();
    
    let mut temps = Vec::new();
    let mut throttled = false;
    
    while start.elapsed() < stress_duration {
        // Stress GPU
        gpu.run_stress_test();
        
        // Record temperature
        let temp = thermal.read_temperature();
        temps.push(temp);
        
        // Check for throttling
        if thermal.is_throttled() {
            throttled = true;
            println!("WARNING: Thermal throttling detected at {:.1}°C", temp);
        }
        
        thread::sleep(Duration::from_secs(1));
    }
    
    // Analysis
    let max_temp = temps.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let avg_temp = temps.iter().sum::<f32>() / temps.len() as f32;
    
    println!("Max temp: {:.1}°C", max_temp);
    println!("Avg temp: {:.1}°C", avg_temp);
    println!("Throttled: {}", throttled);
    
    // Assert acceptable thermal behavior
    assert!(*max_temp < 85.0, "Max temperature exceeded 85°C");
    
    // With active cooling, should not throttle
    if has_active_cooling() {
        assert!(!throttled, "Throttled despite active cooling");
    }
}

#[test]
#[ignore = "requires hardware"]
fn test_performance_under_thermal_pressure() {
    let thermal = ThermalGovernor::new();
    let mut renderer = Renderer::new();
    
    // Heat up the system
    stress_gpu_for(Duration::from_secs(120));
    
    // Measure performance at different temperatures
    let temps = vec![60.0, 70.0, 75.0, 80.0, 82.0];
    
    for target_temp in temps {
        // Stabilize at target temperature
        thermal.stabilize(target_temp);
        
        // Measure FPS
        let fps = renderer.measure_fps(Duration::from_secs(5));
        
        println!("Temp {:.1}°C: {} FPS", target_temp, fps);
        
        // Performance should degrade gracefully
        if target_temp > 80.0 {
            assert!(fps > 45, "Performance degraded too much at high temp");
        }
    }
}
```

### 6.2 Power Consumption

```rust
// tests/thermal/power_consumption.rs

#[test]
#[ignore = "requires power meter"]
fn test_power_consumption() {
    let meter = PowerMeter::connect().unwrap();
    
    // Idle power
    let idle_power = meter.measure_average(Duration::from_secs(30));
    println!("Idle power: {:.2}W", idle_power);
    assert!(idle_power < 5.0, "Idle power too high");
    
    // Load power
    stress_system();
    let load_power = meter.measure_average(Duration::from_secs(30));
    println!("Load power: {:.2}W", load_power);
    assert!(load_power < 15.0, "Load power exceeds PSU rating");
}
```

---

## 7. End-to-End Testing

### 7.1 WebbOS Boot Test

```rust
// tests/e2e/boot_test.rs

#[test]
#[ignore = "requires full system"]
fn test_webbos_boot_sequence() {
    // 1. Boot system
    let system = SystemController::boot().unwrap();
    
    // 2. Wait for desktop
    system.wait_for_desktop(Duration::from_secs(30))
        .expect("Desktop did not appear");
    
    // 3. Verify display output
    let display = system.get_display_info();
    assert_eq!(display.width, 1920);
    assert_eq!(display.height, 1080);
    assert!(display.refresh_rate >= 60);
    
    // 4. Verify input working
    system.simulate_key_press(KEY_ENTER);
    
    // 5. Launch browser
    system.launch_app("browser").unwrap();
    thread::sleep(Duration::from_secs(5));
    
    // 6. Verify browser rendered
    assert!(system.is_app_running("browser"));
}
```

### 7.2 Longevity Test

```rust
// tests/e2e/longevity_test.rs

#[test]
#[ignore = "requires full system, 24h runtime"]
fn test_24_hour_stability() {
    let system = SystemController::boot().unwrap();
    let start = Instant::now();
    let duration = Duration::from_secs(86400); // 24 hours
    
    let mut stats = TestStats::new();
    
    while start.elapsed() < duration {
        // Cycle through apps
        for app in &["browser", "notepad", "paint"] {
            system.launch_app(app).unwrap();
            thread::sleep(Duration::from_secs(60));
            
            // Record metrics
            stats.record(MemorySnapshot::capture());
            stats.record(TemperatureSnapshot::capture());
            
            system.close_app(app).unwrap();
        }
    }
    
    // Analysis
    let memory_leak = stats.detect_memory_leak();
    let thermal_issues = stats.detect_thermal_issues();
    
    assert!(!memory_leak, "Memory leak detected over 24h");
    assert!(!thermal_issues, "Thermal issues over 24h");
}
```

---

## 8. Test Automation

### 8.1 CI/CD Integration

```yaml
# .github/workflows/gui-tests.yml
name: GUI & Input Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          target: aarch64-unknown-none
      
      - name: Run unit tests
        run: cargo test --lib --features rpi5
      
      - name: Generate coverage
        run: cargo tarpaulin --out Xml
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3

  hardware-tests:
    runs-on: [self-hosted, rpi5]
    steps:
      - uses: actions/checkout@v3
      
      - name: Build for Pi 5
        run: cargo build --target aarch64-unknown-none --features rpi5
      
      - name: Run hardware tests
        run: |
          cargo test --test integration --features rpi5 -- --ignored
      
      - name: Run performance benchmarks
        run: |
          cargo test --test performance --features rpi5 -- --ignored
      
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: target/test-results/
```

### 8.2 Test Harness

```rust
// tools/test_harness.rs

pub struct TestHarness {
    config: TestConfig,
    results: TestResults,
}

impl TestHarness {
    /// Run all tests
    pub fn run_all(&mut self) {
        self.run_unit_tests();
        self.run_integration_tests();
        self.run_performance_tests();
        self.run_thermal_tests();
    }
    
    /// Generate report
    pub fn generate_report(&self) -> String {
        format!(r#"
# Test Report

## Summary
- Unit Tests: {}/{} passed
- Integration Tests: {}/{} passed
- Performance: {} within spec
- Thermal: {} stable

## Coverage
- Lines: {:.1}%
- Functions: {:.1}%
- Branches: {:.1}%

## Recommendations
{}
"#,
            self.results.unit.passed, self.results.unit.total,
            self.results.integration.passed, self.results.integration.total,
            if self.results.performance.within_spec { "✅" } else { "❌" },
            if self.results.thermal.stable { "✅" } else { "❌" },
            self.results.coverage.lines * 100.0,
            self.results.coverage.functions * 100.0,
            self.results.coverage.branches * 100.0,
            self.generate_recommendations()
        )
    }
}
```

---

## 9. Test Data

### 9.1 Test Fixtures

```
test_fixtures/
├── display/
│   ├── edid/
│   │   ├── 1080p60.bin
│   │   ├── 4k60_hdr.bin
│   │   └── dsi_touch.bin
│   └── modes/
│       └── standard_modes.json
├── input/
│   ├── hid_reports/
│   │   ├── keyboard_a.bin
│   │   ├── mouse_move.bin
│   │   └── gamepad.bin
│   └── touch/
│       ├── single_touch.bin
│       ├── multi_touch_5.bin
│       └── gesture_swipe.bin
└── shaders/
    ├── test_vertex.glsl
    └── test_fragment.glsl
```

### 9.2 Golden References

```rust
// test_fixtures/golden/mod.rs

/// Expected rendering output for comparison
pub struct GoldenImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl GoldenImage {
    /// Compare with rendered output
    pub fn compare(&self, actual: &[u8]) -> ComparisonResult {
        let mut diff_pixels = 0;
        let threshold = 2; // Allow 2/256 difference per channel
        
        for (expected, actual) in self.pixels.iter().zip(actual.iter()) {
            if (*expected as i16 - *actual as i16).abs() > threshold {
                diff_pixels += 1;
            }
        }
        
        let diff_percent = diff_pixels as f32 / self.pixels.len() as f32;
        
        ComparisonResult {
            passed: diff_percent < 0.001, // 0.1% tolerance
            diff_percentage: diff_percent,
        }
    }
}
```

---

## 10. Test Coverage Requirements

| Component | Unit Test | Integration | E2E | Target |
|-----------|-----------|-------------|-----|--------|
| DRM Driver | 85% | 75% | 50% | 80% |
| GPU Driver | 80% | 70% | 50% | 75% |
| Input Drivers | 85% | 80% | 60% | 80% |
| EGL Backend | 75% | 70% | 50% | 70% |
| Wayland Client | 80% | 75% | 60% | 75% |
| Compositor | 70% | 70% | 60% | 70% |
| Overall | - | - | - | **75%** |

---

*"Dokumentera allt - including the test results."* - Ingrid L'Ingénieure
