# WebbOS Specification Document

> **Version:** 0.1.0  
> **Date:** 2026-01-29  
> **Status:** Draft - Pending Review

---

## 1. Overview

WebbOS is a minimal, high-performance operating system written in Rust, designed around a web-first architecture. The entire desktop environment is implemented as a single HTML file with an integrated web browser engine. Applications are web-based (HTML/JS/WASM) and distributed through a built-in app store.

### 1.1 Key Design Principles

1. **Rust-First:** Maximize Rust usage for memory safety and performance
2. **Minimal Footprint:** Keep the OS lean and efficient
3. **Web-Native:** The desktop IS a browser; apps ARE web pages
4. **Modular:** Clean separation between kernel, browser engine, and desktop shell
5. **x64 Primary:** Initial target is x86_64 architecture

### 1.2 Project Structure

```
webbos/
├── bootloader/          # Bootloader implementation
├── kernel/              # OS kernel (Rust)
│   ├── arch/            # Architecture-specific code
│   ├── drivers/         # Hardware drivers
│   ├── fs/              # File system
│   ├── mm/              # Memory management
│   ├── net/             # Network stack
│   ├── process/         # Process/task management
│   └── syscalls/        # System call interface
├── browser/             # Web browser engine
│   ├── layout/          # Layout engine
│   ├── rendering/       # Rendering/painting
│   ├── javascript/      # JS engine (or integration)
│   ├── wasm/            # WebAssembly runtime
│   ├── network/         # HTTP/HTTPS/TLS client
│   └── dom/             # DOM implementation
├── desktop/             # Desktop environment (HTML/JS)
├── appstore/            # App store server specs
└── shared/              # Shared libraries/types
```

---

## 2. Bootloader Specification

### 2.1 Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| BL-001 | Support x86_64 UEFI boot | Must | Primary boot method |
| BL-002 | Support legacy BIOS boot | Should | For older hardware |
| BL-003 | Multi-boot compatible (Linux chainloading) | Should | GRUB compatibility |
| BL-004 | Load kernel from FAT32/EXT2 | Must | Simple filesystems |
| BL-005 | Set up basic page tables | Must | For 64-bit transition |
| BL-006 | Enable long mode (x64) | Must | Required for kernel |
| BL-007 | Pass memory map to kernel | Must | Via multiboot2 or custom protocol |
| BL-008 | Pass framebuffer info to kernel | Should | For early graphics |
| BL-009 | Size < 64KB | Should | Keep it minimal |

### 2.2 Architecture

```
┌─────────────────────────────────────┐
│           Bootloader                │
├─────────────────────────────────────┤
│  Stage 1: MBR/UEFI (1-4KB)         │
│  - Load Stage 2 from disk          │
├─────────────────────────────────────┤
│  Stage 2: Main Loader (20-50KB)    │
│  - Initialize hardware             │
│  - Parse filesystem                │
│  - Load kernel to memory           │
│  - Setup paging                    │
│  - Enter long mode                 │
│  - Jump to kernel                  │
└─────────────────────────────────────┘
```

### 2.3 Boot Protocol

The bootloader passes information to the kernel via a structured memory block:

```rust
#[repr(C)]
pub struct BootInfo {
    pub magic: u64,              // 0x1BADB002_WEBBOS
    pub version: u32,            // Boot protocol version
    pub memory_map: *const MemoryMap,
    pub memory_map_size: usize,
    pub framebuffer: FramebufferInfo,
    pub rsdp: *const c_void,     // ACPI RSDP pointer
    pub cmdline: *const u8,      // Kernel command line
    pub bootloader_name: *const u8,
}

#[repr(C)]
pub struct MemoryMap {
    pub entries: *const MemoryRegion,
    pub count: usize,
}

#[repr(C)]
pub struct MemoryRegion {
    pub base: u64,
    pub length: u64,
    pub region_type: u32,        // 1=available, 2=reserved, etc.
}
```

### 2.4 Implementation Approach

**Option A: Custom Implementation** (Recommended)
- Write a small UEFI bootloader in Rust using `uefi-rs`
- Small, controlled, WebbOS-specific

**Option B: Use limine**
- Battle-tested bootloader with good protocol
- Less code to maintain

**Decision:** Start with Option A for learning/control, can switch to limine if needed.

---

## 3. Kernel Specification

### 3.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    User Space                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │   Browser   │  │   Desktop   │  │  User Apps      │ │
│  │   Process   │  │   Process   │  │  (WASM/JS/HTML) │ │
│  └──────┬──────┘  └──────┬──────┘  └─────────────────┘ │
└─────────┼────────────────┼─────────────────────────────┘
          │                │
┌─────────┼────────────────┼─────────────────────────────┐
│         │   Kernel Space │                             │
│  ┌──────▼──────┐  ┌──────▼──────┐  ┌─────────────────┐ │
│  │   VFS       │  │   Process   │  │   Network Stack │ │
│  │   Layer     │  │   Manager   │  │   (TCP/IP/TLS)  │ │
│  └──────┬──────┘  └──────┬──────┘  └─────────────────┘ │
│         │                │                              │
│  ┌──────▼────────────────▼──────┐  ┌─────────────────┐ │
│  │      System Call Interface   │  │   Device Drivers│ │
│  └──────────────────────────────┘  └─────────────────┘ │
│                                                        │
│  ┌─────────────────────────────────────────────────┐   │
│  │        Hardware Abstraction Layer (HAL)         │   │
│  │  - Memory Management Unit                       │   │
│  │  - Interrupt Controller (APIC/IO-APIC)          │   │
│  │  - Timer (HPET/ACPI/Local APIC)                 │   │
│  │  - PCI/PCIe Bus                                 │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Memory Management

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| KERN-001 | Physical memory allocator | Must | Bitmap or buddy allocator |
| KERN-002 | Virtual memory management | Must | x86_64 page tables (4-level) |
| KERN-003 | Kernel heap allocator | Must | Linked list or slab allocator |
| KERN-004 | User-space memory isolation | Must | Separate page tables per process |
| KERN-005 | Copy-on-write support | Should | For fork/clone optimization |
| KERN-006 | Demand paging | Should | Load pages on first access |

**Page Table Layout (x86_64):**
- 4KB pages
- 4-level paging (PML4, PDPT, PD, PT)
- Kernel mapped at higher half (0xFFFF800000000000+)
- User space: 0x0000000000000000 - 0x00007FFFFFFFFFFF

### 3.3 Process/Task Management

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| KERN-007 | Preemptive multitasking | Must | Timer-based preemption |
| KERN-008 | Process creation/termination | Must | fork/exec/exit equivalents |
| KERN-009 | Thread support | Must | Within process address space |
| KERN-010 | Scheduling algorithm | Must | CFS-like or simple round-robin |
| KERN-011 | Inter-process communication | Must | Pipes, shared memory, signals |
| KERN-012 | Process isolation | Must | Separate address spaces |

**Process Structure:**
```rust
pub struct Process {
    pub pid: Pid,
    pub state: ProcessState,
    pub address_space: Arc<AddressSpace>,
    pub threads: Vec<Thread>,
    pub file_descriptors: Vec<Option<FileHandle>>,
    pub parent: Option<Pid>,
    pub children: Vec<Pid>,
    pub name: String,
}

pub struct Thread {
    pub tid: Tid,
    pub state: ThreadState,
    pub context: Context,           // CPU registers
    pub kernel_stack: KernelStack,
    pub priority: Priority,
}
```

### 3.4 File System

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| KERN-013 | Virtual File System (VFS) | Must | Abstraction layer |
| KERN-014 | Initial RAM disk (initrd) | Must | Early boot filesystem |
| KERN-015 | Custom filesystem (WebbFS) | Should | Optimized for web assets |
| KERN-016 | FAT32 support | Should | For USB/external media |
| KERN-017 | EXT2/EXT4 support | Could | Linux compatibility |

**VFS Structure:**
```rust
pub trait FileSystem: Send + Sync {
    fn root(&self) -> Arc<dyn INode>;
    fn name(&self) -> &str;
}

pub trait INode: Send + Sync {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, offset: u64, buf: &[u8]) -> Result<usize>;
    fn metadata(&self) -> Metadata;
    fn lookup(&self, name: &str) -> Result<Arc<dyn INode>>;
    fn readdir(&self) -> Result<Vec<DirEntry>>;
}
```

### 3.5 Device Drivers

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| KERN-018 | Serial port (UART 16550) | Must | Debug output |
| KERN-019 | VGA/text mode | Must | Early console |
| KERN-020 | Keyboard (PS/2) | Must | Basic input |
| KERN-021 | Framebuffer (VESA/VBE) | Must | Graphics output |
| KERN-022 | PCI bus enumeration | Must | Device discovery |
| KERN-023 | Storage (AHCI/NVMe) | Must | Disk access |
| KERN-024 | Network (Intel E1000/VirtIO) | Must | Ethernet |
| KERN-025 | USB (EHCI/XHCI) | Should | USB devices |
| KERN-026 | Mouse (PS2/USB) | Should | Pointer input |
| KERN-027 | Audio (Intel HDA/AC97) | Could | Sound output |
| KERN-028 | GPU (basic framebuffer) | Should | Hardware acceleration later |

### 3.6 Network Stack

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| KERN-029 | Ethernet driver interface | Must | Abstract NICs |
| KERN-030 | ARP protocol | Must | Address resolution |
| KERN-031 | IPv4 | Must | Primary IP protocol |
| KERN-032 | IPv6 | Could | Future support |
| KERN-033 | ICMP | Must | Ping, errors |
| KERN-034 | TCP | Must | Reliable streams |
| KERN-035 | UDP | Must | Datagrams |
| KERN-036 | DHCP client | Should | Auto-configuration |
| KERN-037 | DNS resolver | Should | Name resolution |
| KERN-038 | TLS 1.3 | Must | For browser (rustls) |

### 3.7 System Calls

**Core syscalls (minimal set):**

| Syscall | Number | Description |
|---------|--------|-------------|
| exit | 0 | Terminate process |
| write | 1 | Write to file descriptor |
| read | 2 | Read from file descriptor |
| open | 3 | Open file |
| close | 4 | Close file descriptor |
| mmap | 5 | Map memory |
| munmap | 6 | Unmap memory |
| fork | 7 | Create child process |
| exec | 8 | Execute program |
| wait | 9 | Wait for child |
| getpid | 10 | Get process ID |
| gettime | 11 | Get time |
| yield | 12 | Yield CPU |
| sleep | 13 | Sleep for duration |
| socket | 14 | Create socket |
| connect | 15 | Connect to address |
| bind | 16 | Bind socket |
| listen | 17 | Listen for connections |
| accept | 18 | Accept connection |
| send | 19 | Send data |
| recv | 20 | Receive data |
| ioctl | 21 | Device control |
| fcntl | 22 | File control |
| poll | 23 | Poll file descriptors |
| sigaction | 24 | Set signal handler |
| kill | 25 | Send signal |
| getcwd | 26 | Get current directory |
| chdir | 27 | Change directory |
| mkdir | 28 | Create directory |
| unlink | 29 | Delete file |
| stat | 30 | Get file stats |

---

## 4. Web Browser Specification

### 4.1 Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| BRWS-001 | HTML5 parsing | Must | Spec-compliant parser |
| BRWS-002 | CSS3 support | Must | Style computation |
| BRWS-003 | Layout engine (Flow/Grid/Flex) | Must | Modern layouts |
| BRWS-004 | JavaScript execution | Must | Via integration |
| BRWS-005 | WebAssembly runtime | Must | WASM modules |
| BRWS-006 | TLS 1.3 | Must | Secure connections |
| BRWS-007 | HTTP/1.1 and HTTP/2 | Must | Web protocols |
| BRWS-008 | Cookie support | Must | Session management |
| BRWS-009 | Local Storage | Must | Key-value persistence |
| BRWS-010 | Internationalization (i18n) | Must | UTF-8, RTL, Unicode |
| BRWS-011 | Font rendering | Must | TrueType/OpenType |
| BRWS-012 | Image formats (PNG,JPG,SVG) | Must | Basic images |
| BRWS-013 | Video/Audio (basic) | Could | Media support |
| BRWS-014 | GPU acceleration | Could | WebGL later |

### 4.2 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Browser Engine                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   HTML      │  │    CSS      │  │   JavaScript        │ │
│  │   Parser    │→ │   Parser    │→ │   Engine            │ │
│  └──────┬──────┘  └──────┬──────┘  │   (quickjs/deno)    │ │
│         │                │         └─────────────────────┘ │
│         ▼                ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    DOM Tree                              ││
│  └────────────────────┬────────────────────────────────────┘│
│                       │                                      │
│  ┌────────────────────▼────────────────────────────────────┐│
│  │              Style Computation                           ││
│  │         (CSSOM + DOM → Render Tree)                      ││
│  └────────────────────┬────────────────────────────────────┘│
│                       │                                      │
│  ┌────────────────────▼────────────────────────────────────┐│
│  │                 Layout Engine                            ││
│  │        (Calculate positions and sizes)                   ││
│  └────────────────────┬────────────────────────────────────┘│
│                       │                                      │
│  ┌────────────────────▼────────────────────────────────────┐│
│  │               Rendering/Painting                         ││
│  │       (Rasterization, GPU commands)                      ││
│  └────────────────────┬────────────────────────────────────┘│
│                       │                                      │
│  ┌────────────────────▼────────────────────────────────────┐│
│  │               Compositor/Display                         ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  Network Layer (HTTP/HTTPS/TLS/DNS)                         │
├─────────────────────────────────────────────────────────────┤
│  Storage (Cookies, LocalStorage, Cache)                     │
└─────────────────────────────────────────────────────────────┘
```

### 4.3 JavaScript Engine Integration

**Options:**

1. **QuickJS** - Small, embeddable, good performance
2. **Deno Core** - V8-based, full ES support
3. **SpiderMonkey** - Firefox engine
4. **Custom implementation** - Too much work

**Decision:** Use QuickJS for initial implementation due to:
- Small binary size (~1MB)
- Easy embedding
- Good ES2020 support
- Written in C (can bind via FFI)

### 4.4 WebAssembly Runtime

**Requirements:**
- WASM MVP (minimum viable product) support
- WASI (WebAssembly System Interface) for sandboxed I/O
- Linear memory management
- Table/Function imports

**Options:**
1. **wasmtime** - Bytecode Alliance, fast, spec-compliant
2. **wasmer** - Good performance, multiple backends
3. **v8** - If using Deno

**Decision:** Use `wasmtime` as it's the most mature and spec-compliant.

---

## 5. Desktop Environment Specification

### 5.1 Overview

The desktop environment is a single HTML file (`desktop.html`) that runs inside the WebbOS browser. It provides:
- User login/authentication
- Application launcher
- Window management
- System settings
- File manager
- User administration

### 5.2 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Desktop Environment                      │
│                  (Single HTML File)                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    System API Layer                      ││
│  │  (JavaScript bindings to kernel syscalls)                ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │                   Desktop Shell                          ││
│  │  - Taskbar/Dock                                          ││
│  │  - Start Menu/App Launcher                               ││
│  │  - Window Management                                     ││
│  │  - Desktop Background/Widgets                            ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  ││
│  │   Login     │  │   Settings  │  │   File Manager      │  ││
│  │   Screen    │  │   Panel     │  │                     │  ││
│  └─────────────┘  └─────────────┘  └─────────────────────┘  ││
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  ││
│  │  User Admin │  │   App Store │  │   System Monitor    │  ││
│  │             │  │   Client    │  │                     │  ││
│  └─────────────┘  └─────────────┘  └─────────────────────┘  ││
└─────────────────────────────────────────────────────────────┘
```

### 5.3 System API (JavaScript Bindings)

The desktop environment needs access to system functions. These are exposed via a `webbos` global object:

```javascript
// Process Management
webbos.process.exec(command, args) -> Promise<Process>
webbos.process.list() -> Process[]
webbos.process.kill(pid)

// File System
webbos.fs.readFile(path) -> Promise<Uint8Array>
webbos.fs.writeFile(path, data)
webbos.fs.readdir(path) -> Promise<DirEntry[]>
webbos.fs.mkdir(path)
webbos.fs.unlink(path)
webbos.fs.stat(path) -> FileStat

// Network
webbos.net.fetch(url, options) -> Promise<Response>
webbos.net.socketConnect(host, port) -> Promise<Socket>

// System Info
webbos.system.getInfo() -> SystemInfo
webbos.system.getMemoryUsage() -> MemoryInfo
webbos.system.shutdown()
webbos.system.reboot()

// User Management
webbos.users.getCurrent() -> User
webbos.users.list() -> User[]
webbos.users.create(userInfo)
webbos.users.delete(userId)
webbos.users.authenticate(username, password) -> Session

// Apps
webbos.apps.install(appPackage)
webbos.apps.uninstall(appId)
webbos.apps.list() -> App[]
webbos.apps.launch(appId)

// Storage
webbos.storage.local.get(key) -> any
webbos.storage.local.set(key, value)
webbos.storage.local.remove(key)
```

### 5.4 Login Screen

- Simple username/password authentication
- User database stored in `/etc/passwd` (or similar)
- Session token generated on successful login
- Option for auto-login (single user mode)

### 5.5 Window Manager

Since the desktop IS a browser:
- Apps run in `<iframe>` or separate browser contexts
- Desktop manages "windows" as floating divs/iframes
- Z-index management for focus
- Minimize/maximize/close buttons
- Drag to move, resize handles

```javascript
// Window management API (desktop internal)
interface Window {
  id: string;
  title: string;
  appId: string;
  url: string;
  x: number;
  y: number;
  width: number;
  height: number;
  minimized: boolean;
  maximized: boolean;
  focused: boolean;
  
  focus(): void;
  minimize(): void;
  maximize(): void;
  close(): void;
  move(x, y): void;
  resize(w, h): void;
}
```

---

## 6. App Store Specification

### 6.1 Overview

The app store allows users to discover, download, and install applications. For this initial implementation, payment processing is excluded.

### 6.2 App Format

WebbOS apps are packaged as `.webapp` files (ZIP archives):

```
app.webapp/
├── manifest.json      # App metadata
├── icon.png           # App icon (128x128)
├── main.html          # Entry point
├── assets/            # Static resources
│   ├── styles.css
│   ├── scripts.js
│   └── images/
└── wasm/              # Optional WASM modules
    └── module.wasm
```

**manifest.json:**
```json
{
  "id": "com.example.myapp",
  "name": "My Application",
  "version": "1.0.0",
  "description": "A great app for WebbOS",
  "author": "Example Developer",
  "entry": "main.html",
  "icon": "icon.png",
  "permissions": [
    "storage",
    "network",
    "filesystem"
  ],
  "window": {
    "width": 800,
    "height": 600,
    "resizable": true
  }
}
```

### 6.3 App Store Server

**API Endpoints:**

```
GET  /api/apps              # List all apps
GET  /api/apps/{id}         # Get app details
GET  /api/apps/{id}/download # Download app package
POST /api/apps/{id}/purchase # (Future: purchase app)
```

**App Store Client (in Desktop):**
- Browse/search apps
- View app details, screenshots
- Download and install apps
- List installed apps
- Uninstall apps

### 6.4 App Storage

- Apps installed to `/apps/{app-id}/`
- User data stored in `/home/{user}/.apps/{app-id}/`
- App registry in `/var/lib/webbos/apps.json`

### 6.5 Demo Apps

Create 2-3 demo apps for testing:

1. **Calculator** - Simple arithmetic calculator
2. **Text Editor** - Basic text editing with save/load
3. **Weather** - Fetch and display weather data

---

## 7. Test Plan

### 7.1 Testing Strategy

**Test-Driven Development (TDD) Approach:**
1. Write test before implementation
2. Run test (should fail)
3. Implement feature
4. Run test (should pass)
5. Refactor if needed

### 7.2 Test Categories

| Category | Tools | Scope |
|----------|-------|-------|
| Unit Tests | Rust `cargo test` | Individual functions/modules |
| Integration Tests | Rust test + scripts | Component interactions |
| Kernel Tests | Custom test harness | Kernel functions in QEMU |
| Browser Tests | Playwright (via MCP) | UI automation, rendering |
| End-to-End | Playwright | Full user workflows |
| Performance | Custom benchmarks | Speed, memory usage |

### 7.3 Coverage Requirements

| Component | Target Coverage |
|-----------|-----------------|
| Bootloader | 70% |
| Kernel | 80% |
| Browser | 75% |
| Desktop | 70% |
| App Store | 75% |

### 7.4 CI/CD Pipeline

```yaml
stages:
  - lint
  - unit-test
  - build
  - integration-test
  - e2e-test
  - coverage-report
  - release
```

### 7.5 Key Test Scenarios

**Bootloader:**
- Loads correctly in QEMU (UEFI and BIOS)
- Can locate and load kernel
- Passes correct boot info structure
- Memory map is valid

**Kernel:**
- Memory allocation/deallocation
- Process creation and termination
- Context switching
- System call interface
- File system operations
- Network packet handling

**Browser:**
- HTML parsing (various test pages)
- CSS styling computation
- Layout calculations
- JavaScript execution
- WASM module loading
- TLS handshake

**Desktop:**
- Login flow
- App launch
- Window operations
- File manager operations
- Settings changes

**App Store:**
- Browse apps
- Download and install
- Launch installed app
- Uninstall app

---

## 8. Orchestrator Project Plan

### 8.1 Phase 1: Foundation (Weeks 1-4)

| Task | Owner | Est. | Dependencies |
|------|-------|------|--------------|
| 1.1 Setup build system and toolchain | Agent-1 | 2d | - |
| 1.2 Implement bootloader (UEFI) | Agent-2 | 5d | - |
| 1.3 Kernel skeleton + boot | Agent-3 | 3d | 1.2 |
| 1.4 Physical memory manager | Agent-4 | 4d | 1.3 |
| 1.5 Virtual memory + paging | Agent-5 | 5d | 1.4 |
| 1.6 Heap allocator | Agent-6 | 3d | 1.5 |
| 1.7 Interrupt handling (IDT) | Agent-7 | 3d | 1.3 |
| 1.8 Serial output (debug) | Agent-8 | 1d | 1.3 |

**Milestone 1:** Kernel boots, prints "Hello WebbOS", memory management works

### 8.2 Phase 2: Kernel Core (Weeks 5-8)

| Task | Owner | Est. | Dependencies |
|------|-------|------|--------------|
| 2.1 Process/Thread management | Agent-1 | 5d | 1.5, 1.7 |
| 2.2 Scheduler | Agent-2 | 4d | 2.1 |
| 2.3 System call interface | Agent-3 | 3d | 2.1 |
| 2.4 VFS + initrd | Agent-4 | 4d | 1.3 |
| 2.5 Keyboard driver | Agent-5 | 2d | 1.7 |
| 2.6 Timer/RTC | Agent-6 | 2d | 1.7 |
| 2.7 PCI bus driver | Agent-7 | 3d | 1.3 |
| 2.8 Storage driver (AHCI) | Agent-8 | 4d | 2.7 |

**Milestone 2:** Multi-process kernel with basic I/O

### 8.3 Phase 3: Network & Storage (Weeks 9-11)

| Task | Owner | Est. | Dependencies |
|------|-------|------|--------------|
| 3.1 Network driver (E1000/VirtIO) | Agent-1 | 3d | 2.7 |
| 3.2 TCP/IP stack | Agent-2 | 5d | 3.1 |
| 3.3 TLS 1.3 (rustls) | Agent-3 | 4d | 3.2 |
| 3.4 DNS client | Agent-4 | 2d | 3.2 |
| 3.5 Filesystem (WebbFS) | Agent-5 | 4d | 2.4, 2.8 |

**Milestone 3:** Network connectivity, can fetch HTTPS URLs

### 8.4 Phase 4: Browser Engine (Weeks 12-18)

| Task | Owner | Est. | Dependencies |
|------|-------|------|--------------|
| 4.1 HTML parser | Agent-1 | 5d | - |
| 4.2 DOM implementation | Agent-2 | 4d | 4.1 |
| 4.3 CSS parser | Agent-3 | 3d | - |
| 4.4 Style computation | Agent-4 | 5d | 4.2, 4.3 |
| 4.5 Layout engine | Agent-5 | 7d | 4.4 |
| 4.6 Rendering (CPU) | Agent-6 | 5d | 4.5 |
| 4.7 JavaScript integration | Agent-7 | 5d | 4.2 |
| 4.8 WASM runtime | Agent-8 | 4d | - |
| 4.9 Image decoding | Agent-9 | 3d | - |
| 4.10 Font rendering | Agent-10 | 4d | - |
| 4.11 HTTP client | Agent-11 | 3d | 3.2 |
| 4.12 Cookie/Storage | Agent-12 | 3d | 4.11 |

**Milestone 4:** Browser can render basic HTML/CSS/JS pages

### 8.5 Phase 5: Desktop Environment (Weeks 19-22)

| Task | Owner | Est. | Dependencies |
|------|-------|------|--------------|
| 5.1 System API bindings (JS→Syscalls) | Agent-1 | 5d | 2.3 |
| 5.2 Desktop shell UI | Agent-2 | 5d | 4.6 |
| 5.3 Login screen | Agent-3 | 3d | 5.2 |
| 5.4 Window management | Agent-4 | 4d | 5.2 |
| 5.5 File manager app | Agent-5 | 4d | 5.1 |
| 5.6 Settings panel | Agent-6 | 3d | 5.2 |
| 5.7 User admin tool | Agent-7 | 3d | 5.1 |

**Milestone 5:** Full desktop environment functional

### 8.6 Phase 6: App Store (Weeks 23-25)

| Task | Owner | Est. | Dependencies |
|------|-------|------|--------------|
| 6.1 App package format | Agent-1 | 2d | - |
| 6.2 App installation system | Agent-2 | 3d | 6.1 |
| 6.3 App store server API | Agent-3 | 3d | - |
| 6.4 App store client UI | Agent-4 | 4d | 5.2, 6.3 |
| 6.5 Demo app 1 (Calculator) | Agent-5 | 2d | 6.1 |
| 6.6 Demo app 2 (Text Editor) | Agent-6 | 3d | 6.1 |
| 6.7 Demo app 3 (Weather) | Agent-7 | 2d | 6.1, 3.2 |

**Milestone 6:** Can browse, install, and run apps from store

### 8.7 Phase 7: Integration & Polish (Weeks 26-28)

| Task | Owner | Est. | Dependencies |
|------|-------|------|--------------|
| 7.1 End-to-end testing | All | 5d | All |
| 7.2 Performance optimization | Agent-1 | 4d | All |
| 7.3 Documentation | Agent-2 | 5d | All |
| 7.4 Bug fixes | All | 5d | All |
| 7.5 Release packaging | Agent-3 | 2d | All |

**Milestone 7:** WebbOS v1.0 Release

### 8.8 Agent Roles

| Agent | Specialization | Primary Tasks |
|-------|----------------|---------------|
| Agent-1 | Build/Boot | Bootloader, Build System |
| Agent-2 | Kernel Core | Memory, Scheduling |
| Agent-3 | Kernel I/O | Drivers, Syscalls |
| Agent-4 | Filesystem | VFS, WebbFS |
| Agent-5 | Network | Stack, Protocols |
| Agent-6 | Browser Core | HTML, DOM |
| Agent-7 | Browser Style | CSS, Layout |
| Agent-8 | Browser Runtime | JS, WASM |
| Agent-9 | Browser Render | Painting, GPU |
| Agent-10 | Desktop UI | Shell, Components |
| Agent-11 | System Tools | File Manager, Settings |
| Agent-12 | App Store | Package, Store |

---

## 9. Open Questions

1. **Graphics Stack:** Software rendering initially, GPU acceleration later?
2. **Audio:** Include basic audio support in v1?
3. **Multi-user:** Full multi-user or single-user with profiles?
4. **Security:** Sandbox level for apps (WASM provides some)?
5. **Updates:** OTA update mechanism for OS?
6. **Hardware Targets:** Primary target is QEMU, specific real hardware?

---

## 10. Glossary

| Term | Definition |
|------|------------|
| VFS | Virtual File System |
| HAL | Hardware Abstraction Layer |
| IDT | Interrupt Descriptor Table |
| APIC | Advanced Programmable Interrupt Controller |
| ACPI | Advanced Configuration and Power Interface |
| PCI | Peripheral Component Interconnect |
| AHCI | Advanced Host Controller Interface (SATA) |
| NVMe | Non-Volatile Memory Express |
| UEFI | Unified Extensible Firmware Interface |
| COW | Copy-on-Write |
| CFS | Completely Fair Scheduler |
| TLS | Transport Layer Security |
| WASM | WebAssembly |
| WASI | WebAssembly System Interface |

---

**END OF SPECIFICATION**

> **Next Steps:**
> 1. Review this specification with stakeholders
> 2. Confirm/modify requirements
> 3. Approve architecture decisions
> 4. Begin Phase 1 implementation
