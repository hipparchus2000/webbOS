# WebbOS Development Environment

## System Information

- **OS**: Windows 11
- **Shell**: PowerShell (primary), GitBash available
- **Git**: Installed via GitBash
- **Rust**: Nightly toolchain (nightly-2025-01-15)
- **QEMU**: Available but has limitations with ARM64 on Windows

## Git Configuration

Git is installed via GitBash. To use git commands, use GitBash terminal or add git to PATH:

```powershell
# Git is typically located at:
# C:\Program Files\Git\bin\git.exe
# or
# C:\Users\<username>\AppData\Local\Programs\Git\bin\git.exe
```

## Build Commands

### Pi Port (ARM64)
```powershell
cd Pi
cargo +nightly-2025-01-15 build -p bootloader --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release
cargo +nightly-2025-01-15 build -p kernel --target aarch64-unknown-none -Z build-std=core,compiler_builtins,alloc --release
python make-raw-image.py target/aarch64-unknown-none/release/bootloader target/aarch64-unknown-none/release/kernel webbos-pi-raw.img
```

### PC Port (x86_64)
```powershell
cd PC
./build.bat
./run.bat
```

## Project Structure

```
webbOs/
├── PC/          # x86_64 UEFI version
├── Pi/          # ARM64 Raspberry Pi 3/4 (most complete)
├── Pi5/         # ARM64 Raspberry Pi 5 (in progress)
├── PORT_COMPARISON.md
├── SECURITY_AUDIT_REPORT.md
└── TASKS.md     # Development tasks and status
```

## Recent Major Changes (2025-02-25)

### Security Fixes (Pi Port)
- ✅ WPA2: Proper PBKDF2-HMAC-SHA1 implementation
- ✅ TCP ISN: RFC 6528 compliant generation
- ✅ Filesystem: Bounds checking on all parsers
- ✅ USB/DTB: Descriptor/DTB validation
- ✅ Scheduler: Race condition fixes with proper synchronization

### Features Added
- ✅ Framebuffer debug output (bootloader shows text on HDMI)
- ✅ UART debug output (serial console)
- ✅ Process scheduler with context switching
- ✅ WiFi SDIO data channel integration

## Testing Notes

- **PC Port**: Works in QEMU on Windows
- **Pi Port**: Requires real Raspberry Pi hardware (QEMU on Windows has memory issues)
- **Framebuffer**: Shows boot progress on HDMI monitor
- **UART**: Requires USB-to-TTL adapter for serial output

## Coding Standards

- Use `checked_add`, `checked_sub` for arithmetic that could overflow
- Replace `static mut` with `AtomicU32`/`Mutex<T>`/`lazy_static!`
- Use `&raw mut` and `&raw const` instead of `&mut` on statics
- All `unsafe` blocks must have safety comments
