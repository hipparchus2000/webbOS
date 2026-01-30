# 🌐 WebbOS

A web browser operating system that boots directly into a desktop environment with a full web browser, applications, and user management.

![WebbOS](docs/assets/webbos-logo.png)

## ✨ Features

- **🖥️ Desktop Environment** - Modern HTML/CSS-based desktop with windows, taskbar, and start menu
- **🎨 Built-in Apps** - Notepad, Paint, File Manager, Task Manager, User Manager, Terminal, Web Browser
- **👤 User Management** - Multi-user support with authentication and sessions
- **🌐 Full Networking** - TCP/IP, HTTP/HTTPS, TLS 1.3, DNS resolver
- **💾 File Systems** - EXT2, FAT32 with storage drivers (ATA, NVMe)
- **🔒 Security** - SHA-256 password hashing, modern cryptography

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly-2025-01-15
rustup component add rust-src --toolchain nightly-2025-01-15

# Install QEMU
# Windows: choco install qemu
# macOS: brew install qemu
# Ubuntu: sudo apt install qemu-system-x86
```

### Build and Run

```powershell
# Clone the repository
git clone https://github.com/yourusername/webbos.git
cd webbos

# Build and run (Windows PowerShell)
.\scripts\run-qemu.ps1

# Or manually:
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc
.\scripts\create-image.ps1
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=webbos.img -vga std -m 512M
```

### Default Login

When WebbOS boots, use these credentials:

| Username | Password | Type |
|----------|----------|------|
| `admin` | `admin` | Administrator |
| `user` | `user` | Standard User |

## 📸 Screenshots

### Login Screen
```
╔══════════════════════════════════════════╗
║                                          ║
║              🌐 WebbOS                   ║
║                                          ║
║         Welcome to WebbOS                ║
║    Web Browser Operating System          ║
║                                          ║
║    ┌─────────────────────────┐          ║
║    │ Username                │          ║
║    └─────────────────────────┘          ║
║    ┌─────────────────────────┐          ║
║    │ Password                │          ║
║    └─────────────────────────┘          ║
║                                          ║
║         [ Sign In ]                      ║
║                                          ║
║    Default: admin/admin or user/user     ║
║                                          ║
╚══════════════════════════════════════════╝
```

### Desktop
```
╔══════════════════════════════════════════════════════════╗
║  🏠 Home     📄 Documents                    12:45  👤  ║
╠══════════════════════════════════════════════════════════╣
║                                                          ║
║   📝 Notepad          ┌────────────────────────┐        ║
║   📊 Task Manager     │  Welcome to WebbOS!    │        ║
║   🎨 Paint            │                        │        ║
║   📁 File Manager     │  This is a fully       │        ║
║   💻 Terminal         │  functional desktop    │        ║
║                       │  environment.          │        ║
║   🗑 Trash            │                        │        ║
║                       └────────────────────────┘        ║
╠══════════════════════════════════════════════════════════╣
║  🌐 Start │ 📝 Notepad │ 📊 Task Manager      12:45 PM  ║
╚══════════════════════════════════════════════════════════╝
```

## 🎮 Using WebbOS

### Desktop Navigation

- **Click Start** (🌐) to open the application menu
- **Click windows** to focus them
- **Drag windows** by their title bar
- **Use window controls** (minimize, maximize, close)

### Available Commands

From the shell, type:

```
help          - Show all commands
info          - System information
memory        - Memory statistics
processes     - Show running processes
network       - Network status
users         - List user accounts
launch notepad     - Open Notepad
launch paint       - Open Paint
launch filemanager - Open File Manager
vesa          - Graphics info
input         - Input device status
test          - Run test suite
reboot        - Reboot system
shutdown      - Shutdown system
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Desktop Environment (HTML/CSS/JS)                      │
│  ├── Login Screen                                       │
│  ├── Window Manager                                     │
│  └── 7 Applications                                     │
├─────────────────────────────────────────────────────────┤
│  System Services                                        │
│  ├── User Management (SHA-256, Sessions)               │
│  ├── Graphics (VESA Framebuffer)                       │
│  └── Input (Keyboard, Mouse)                           │
├─────────────────────────────────────────────────────────┤
│  Network Stack                                          │
│  ├── HTTP/HTTPS Client                                 │
│  ├── TLS 1.3 (ChaCha20-Poly1305)                      │
│  └── TCP/IP + Socket API                               │
├─────────────────────────────────────────────────────────┤
│  Kernel Core                                            │
│  ├── Memory Management                                 │
│  ├── Process Scheduler                                 │
│  ├── VFS (EXT2, FAT32)                                │
│  └── Interrupt Handling                                │
└─────────────────────────────────────────────────────────┘
```

## 📚 Documentation

- [Architecture](docs/ARCHITECTURE.md) - System design and components
- [Features](docs/FEATURES.md) - Complete feature list
- [Running](docs/RUNNING.md) - Detailed running instructions
- [Build](docs/BUILD.md) - Build system documentation

## 🛠️ Development

```bash
# Build kernel
cargo +nightly-2025-01-15 build -p kernel --target x86_64-unknown-none -Z build-std=core,compiler_builtins,alloc

# Build bootloader  
cargo +nightly-2025-01-15 build -p bootloader --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc

# Run with network
.\scripts\run-qemu.ps1 -Network

# Debug mode (with GDB)
.\scripts\run-qemu.ps1 -Debug
```

## 📊 Specifications

| Component | Specification |
|-----------|---------------|
| **Architecture** | x86_64 |
| **Boot** | UEFI |
| **Resolution** | 1024x768 (32-bit color) |
| **Memory** | 512MB recommended |
| **Storage** | 64MB disk image |
| **Network** | VirtIO networking |

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details.

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Rust programming language
- QEMU for virtualization
- Various open-source references and specifications

---

**WebbOS** - A web browser operating system for the modern era. 🌐✨

Built with ❤️ and Rust.
