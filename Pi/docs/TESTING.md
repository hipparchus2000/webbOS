# Testing WebbOS

## Test Strategy

WebbOS uses a multi-layered testing approach:

1. **Unit Tests** - Test individual functions
2. **Integration Tests** - Test component interactions
3. **Kernel Tests** - Test kernel in QEMU
4. **End-to-End Tests** - Full system testing

## Running Tests

### Unit Tests

```bash
# Test shared library
cargo test -p webbos-shared

# Test with release optimizations
cargo test -p webbos-shared --release
```

### Kernel Tests

Kernel tests run inside QEMU with a custom test runner.

```bash
# Build and run kernel tests
make test-kernel

# Run specific test
cargo test -p kernel --test kernel_test -- <test_name>
```

### Integration Tests

```bash
# Run all integration tests
make test-integration

# Run with verbose output
make test-integration VERBOSE=1
```

## Writing Tests

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_contains() {
        let region = MemoryRegion::new(
            PhysAddr::new(0x1000),
            ByteSize::new(0x1000),
            MemoryRegionType::Available,
        );

        assert!(region.contains(PhysAddr::new(0x1000)));
        assert!(region.contains(PhysAddr::new(0x1FFF)));
        assert!(!region.contains(PhysAddr::new(0x2000)));
    }
}
```

### Kernel Test Example

```rust
#[test_case]
fn test_paging() {
    let phys_offset = VirtAddr::new(0xFFFF800000000000);
    let mapper = unsafe { init(phys_offset) };
    
    // Test that we can create a mapping
    let page = Page::containing_address(VirtAddr::new(0x1000));
    let frame = allocate_frame().unwrap();
    
    unsafe {
        mapper.map_to(page, frame, PageTableFlags::PRESENT, &mut frame_allocator)
            .unwrap()
            .flush();
    }
}
```

## Test Organization

```
tests/
├── unit/               # Unit tests (run on host)
├── kernel/             # Kernel tests (run in QEMU)
├── integration/        # Integration tests
└── fixtures/           # Test data
```

## Coverage

### Generate Coverage Report

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage/

# View report
open coverage/tarpaulin-report.html
```

### Coverage Targets

| Component | Target |
|-----------|--------|
| shared | 80% |
| bootloader | 70% |
| kernel | 80% |
| browser | 75% |
| desktop | 70% |

## CI/CD Testing

Tests are run automatically on pull requests:

1. **Lint** - `cargo clippy`
2. **Format** - `cargo fmt --check`
3. **Unit** - `cargo test --lib`
4. **Build** - `cargo build --release`
5. **Kernel** - Run in QEMU
6. **Coverage** - Generate and upload report

## Debugging Test Failures

### QEMU Output

```bash
# Run with serial output to stdout
make run-debug

# Run with GDB
make debug
# In another terminal: gdb -ex "target remote :1234"
```

### Logs

Enable kernel logging:
```rust
// In kernel code
log::info!("Debug message: {:?}", value);
log::debug!("Verbose debug: {}", detail);
```

View logs:
```bash
# Serial output is logged to serial.log
make run-debug 2>&1 | tee serial.log
```

## Performance Testing

### Benchmarks

```bash
# Run benchmarks
cargo bench -p kernel

# Profile with perf (Linux)
perf record -g target/release/kernel
perf report
```

### Memory Testing

```bash
# Check for memory leaks in tests
cargo test -p kernel --features leak-detection
```

## Fuzzing

Fuzz testing for input parsing:

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Run fuzzer
cargo fuzz run parser
```
