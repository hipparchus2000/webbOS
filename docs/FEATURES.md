# WebbOS Features

## Overview
WebbOS is a web browser operating system that provides a complete desktop environment with applications, user management, and networking capabilities.

## Core Features

### 🖥️ Desktop Environment
- **Modern UI**: Beautiful HTML/CSS-based desktop with gradient wallpapers
- **Window Manager**: Multi-window support with z-index layering, minimize/maximize/close
- **Taskbar**: Shows running applications, system clock, start menu
- **Desktop Icons**: Files, folders, Home, Documents, Trash
- **Start Menu**: Launch applications, system settings, logout

### 👤 User Management
- **Multi-user support**: Create, delete, and manage user accounts
- **Authentication**: SHA-256 password hashing with session management
- **User types**: Administrator and standard users
- **Default accounts**:
  - `admin` / `admin` (full privileges)
  - `user` / `user` (standard user)

### 🎨 Built-in Applications

| Application | Icon | Description |
|------------|------|-------------|
| **File Manager** | 📁 | Browse and manage files and folders |
| **Notepad** | 📝 | Text editor with open/save functionality |
| **Paint** | 🎨 | Drawing application with pen, eraser, colors |
| **Task Manager** | 📊 | View processes, CPU/memory usage, kill tasks |
| **User Manager** | 👥 | Manage user accounts and permissions |
| **Terminal** | 💻 | Command-line interface |
| **Web Browser** | 🌐 | Browse the web with HTTP/HTTPS support |

### 🌐 Networking
- **TCP/IP Stack**: Full IPv4 implementation
- **HTTP/HTTPS Client**: HTTP/1.1 and HTTP/2 support
- **DNS Resolver**: Domain name resolution
- **TLS 1.3**: Modern encryption with ChaCha20-Poly1305
- **Socket API**: BSD-style sockets

### 💾 File Systems
- **VFS Layer**: Virtual filesystem abstraction
- **EXT2**: Second Extended Filesystem support
- **FAT32**: File Allocation Table support
- **Storage Drivers**: ATA/IDE, AHCI/SATA, NVMe

### 🔒 Security
- **Password hashing**: SHA-256 with salt
- **TLS 1.3**: Modern cryptographic protocols
- **Memory protection**: Higher-half kernel, NX bit
- **User isolation**: Separate user sessions

## System Commands

```
help              - Show available commands
info              - System information
memory            - Memory statistics
processes         - Show running processes
network           - Network status
storage           - Storage devices
users             - List user accounts
login             - Login to desktop
desktop           - Show desktop info
launch <app>      - Launch application
vesa              - Graphics info
input             - Input device status
test              - Run test suite
reboot            - Reboot system
shutdown          - Shutdown system
```

## Technical Specifications

### Architecture
- **Target**: x86_64 (64-bit)
- **Boot**: UEFI
- **Kernel**: Monolithic, higher-half loaded
- **Memory**: 4-level paging, 1MB initial heap

### Graphics
- **Resolution**: 1024x768 default (configurable)
- **Color depth**: 32-bit (ARGB)
- **Driver**: VESA/VBE framebuffer
- **Primitives**: Lines, rectangles, circles, text

### Input
- **Keyboard**: PS/2 with scancode translation
- **Mouse**: PS/2 with 3 buttons and scroll
- **Event system**: Poll-based with queue

### Network Stack
```
Application (Browser, HTTP Client)
          ↓
    Socket API
          ↓
TCP / UDP
          ↓
    IPv4 / ICMP
          ↓
    ARP
          ↓
VirtIO Network Driver
```

### Build Information
- **Language**: Rust (nightly-2025-01-15)
- **Kernel Size**: ~6.7 MB
- **Lines of Code**: ~15,000
- **License**: MIT

## File Structure

```
webbOs/
├── bootloader/          # UEFI bootloader
├── kernel/              # Kernel source
│   ├── src/
│   │   ├── arch/        # x86_64 architecture code
│   │   ├── drivers/     # Device drivers
│   │   ├── fs/          # Filesystems (EXT2, FAT32)
│   │   ├── net/         # Network stack
│   │   ├── crypto/      # Cryptographic primitives
│   │   ├── tls/         # TLS 1.3 implementation
│   │   ├── browser/     # Web browser engine
│   │   ├── graphics/    # Graphics subsystem
│   │   ├── desktop/     # Desktop environment
│   │   ├── users/       # User management
│   │   ├── mm/          # Memory management
│   │   └── main.rs      # Kernel entry
│   └── Cargo.toml
├── shared/              # Shared types between bootloader and kernel
├── docs/                # Documentation
└── Cargo.toml           # Workspace manifest
```

## Future Enhancements

### Planned Features
- [ ] USB HID support (keyboards, mice)
- [ ] Audio subsystem
- [ ] Multi-monitor support
- [ ] Software package manager
- [ ] WebRTC support
- [ ] IPv6 networking
- [ ] ACPI power management
- [ ] SMP (multi-core) support

### Known Limitations
- Single display resolution (1024x768)
- No hardware acceleration
- Limited USB device support
- No suspend/resume

## Credits

Built with ❤️ using Rust and the following amazing projects:
- Rust programming language
- UEFI specification
- Various open-source references

---

**WebbOS** - A web browser operating system for the modern era.
