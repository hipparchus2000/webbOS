# WebbOS Test Suite

This directory contains tests for the WebbOS project.

## Test Categories

### Unit Tests

Located within each crate's `src/` directory, using `#[cfg(test)]` modules.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        assert_eq!(1 + 1, 2);
    }
}
```

### Integration Tests

Located in `tests/integration/`.

### Kernel Tests

Kernel-level tests run in QEMU. See `tests/kernel/`.

## Running Tests

```bash
# Run all tests
cargo test --all

# Run specific test
cargo test -p webbos-shared

# Run kernel tests in QEMU
cargo test -p kernel --target x86_64-unknown-none
```

## Test Coverage

Generate coverage reports with:

```bash
cargo tarpaulin --out Html
```
