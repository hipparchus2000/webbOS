# WebbOS ARM64 Build Buglist

## Build Status: FAILED

**Date:** 2026-02-16  
**Target:** aarch64-unknown-none  
**Rust Version:** nightly-2025-01-15  
**Build Scripts Used:** build-aarch64.sh, build-aarch64-drivers.sh

## Summary

The ARM64 build of webbOS has failed with **272 errors** and **106 warnings** in the kernel, and **1 critical error** in the bootloader. The primary issues are:

1. **Architecture Porting Issues**: x86_64-specific code being compiled for ARM64
2. **Missing Dependencies**: No `alloc` imports for collections and strings
3. **Invalid Assembly**: x86 assembly instructions in ARM64 context
4. **Configuration Issues**: Missing ARM64-specific implementations
5. **Syntax Errors**: Invalid identifiers and type mismatches

## Bug Categories

### Category 1: Architecture Porting Issues (BLOCKING)

#### Bug 1.1: x86_64-specific modules not available for ARM64
- **Severity:** BLOCKING
- **Files:** 
  - `kernel/src/arch/mod.rs`
  - Multiple files importing `arch::cpu`, `arch::interrupts`, `arch::paging`, `arch::gdt`
- **Errors:**
  - `E0432: unresolved import crate::arch::paging`
  - `E0432: unresolved import crate::arch::gdt`
  - `E0432: unresolved import crate::arch::interrupts`
- **Root Cause:** The `arch` module conditionally compiles x86_64 modules but ARM64 modules are in `arch::aarch64` submodule
- **Fix:** Update imports to use `crate::arch::aarch64::*` or create proper conditional compilation

#### Bug 1.2: x86 assembly instructions in ARM64 build
- **Severity:** BLOCKING
- **Files:**
  - `kernel/src/console/serial.rs` (lines 53-65, 126-136)
  - `kernel/src/syscall/mod.rs` (lines 177-212)
  - `kernel/src/drivers/timer.rs` (lines 37-133)
  - `kernel/src/drivers/pci.rs` (lines 47-248)
  - `kernel/src/drivers/input/mod.rs` (lines 22-56)
  - `kernel/src/storage/ata.rs` (lines 399-433)
- **Errors:** `invalid register 'dx'`, `invalid register 'al'`, `invalid register 'eax'`, etc.
- **Root Cause:** x86 `in`/`out` assembly instructions being compiled for ARM64
- **Fix:** Create ARM64-specific implementations or use conditional compilation

### Category 2: Missing Dependencies and Imports (BLOCKING)

#### Bug 2.1: Missing `alloc` crate imports
- **Severity:** BLOCKING
- **Files:** Multiple files throughout filesystem modules
- **Errors:**
  - `E0432: unresolved import crate::error`
  - `E0412: cannot find type Vec in this scope`
  - `E0412: cannot find type String in this scope`
  - `E0433: failed to resolve: use of undeclared crate or module std`
- **Root Cause:** Filesystem modules use `alloc` types (`Vec`, `String`) without proper imports
- **Fix:** Add `use alloc::vec::Vec;`, `use alloc::string::String;`, `use alloc::format;` etc.

#### Bug 2.2: Missing macro imports
- **Severity:** BLOCKING
- **Files:** Filesystem modules
- **Errors:**
  - `error: cannot find macro format in this scope`
  - `error: cannot find macro vec in this scope`
- **Root Cause:** `format!` and `vec!` macros from `alloc` not imported
- **Fix:** Add `use alloc::format;` and `use alloc::vec;`

### Category 3: Syntax and Type Errors (BLOCKING)

#### Bug 3.1: Invalid identifier starting with number
- **Severity:** BLOCKING
- **File:** `kernel/src/drivers/raspberrypi/ethernet/mod.rs:176`
- **Error:** `error: expected identifier, found '9346CR'`
- **Root Cause:** Rust identifiers cannot start with numbers
- **Fix:** Rename `9346CR` to `REG_9346CR` or similar

#### Bug 3.2: Duplicate constant definition
- **Severity:** BLOCKING
- **File:** `kernel/src/drivers/raspberrypi/ethernet/mod.rs:176`
- **Error:** `error[E0428]: the name 'CR' is defined multiple times`
- **Root Cause:** `CR` constant defined twice in same module
- **Fix:** Rename one of the `CR` constants

#### Bug 3.3: Pattern binding inconsistency
- **Severity:** BLOCKING
- **File:** `kernel/src/fs/partition/mod.rs:82-84`
- **Error:** `E0408: variable 'guid' is not bound in all patterns`
- **Root Cause:** `guid` variable only bound in `Gpt` pattern but not `Mbr` patterns
- **Fix:** Restructure match arms or use different pattern

#### Bug 3.4: Copy trait implementation failure
- **Severity:** BLOCKING
- **File:** `kernel/src/fs/partition/mod.rs:47`
- **Error:** `E0204: the trait core::marker::Copy cannot be implemented for this type`
- **Root Cause:** `Partition` struct contains `PartitionType` which doesn't implement `Copy`
- **Fix:** Remove `Copy` derive or implement `Copy` for `PartitionType`

### Category 4: Configuration and Feature Issues (HIGH)

#### Bug 4.1: Unstable feature usage
- **Severity:** HIGH
- **File:** `kernel/src/fs/block/sdhost.rs:800`
- **Error:** `E0658: use of unstable library feature 'stdarch_arm_hints'`
- **Root Cause:** Using `core::arch::aarch64::__nop()` without enabling feature
- **Fix:** Add `#![feature(stdarch_arm_hints)]` or use alternative implementation

#### Bug 4.2: Invalid enum discriminant
- **Severity:** HIGH
- **File:** `kernel/src/arch/aarch64/interrupts.rs:12-20`
- **Error:** `E0081: discriminant value '7' assigned more than once`
- **Root Cause:** `CP14DTTrap` and `AdvSIMDFPAccessTrap` both have value `0x07`
- **Fix:** Assign unique values to each enum variant

#### Bug 4.3: Missing enum variants
- **Severity:** HIGH
- **File:** `kernel/src/arch/aarch64/interrupts.rs:248`
- **Error:** `E0599: no variant or associated item named 'IRQSPx' found`
- **Root Cause:** Referencing non-existent enum variants
- **Fix:** Add missing variants or correct variant names

### Category 5: Type Mismatches and Logic Errors (MEDIUM)

#### Bug 5.1: Type mismatch in panic handler
- **Severity:** MEDIUM
- **File:** `kernel/src/arch/aarch64/mod.rs:35`
- **Error:** `E0308: mismatched types`
- **Root Cause:** `if let` expecting `PanicMessage` but `info.message()` returns `Option`
- **Fix:** Correct pattern matching

#### Bug 5.2: Missing `to_string()` method
- **Severity:** MEDIUM
- **Files:** Multiple files
- **Error:** `E0599: no method named 'to_string' found for reference '&str'`
- **Root Cause:** `ToString` trait not in scope
- **Fix:** Add `use alloc::string::ToString;`

#### Bug 5.3: Type inference failure
- **Severity:** MEDIUM
- **File:** `kernel/src/fs/fat32/mod.rs:691`
- **Error:** `E0282: type annotations needed`
- **Root Cause:** Cannot infer type for `eq_ignore_ascii_case` generic parameter
- **Fix:** Add type annotation: `lfn.eq_ignore_ascii_case::<&str>(name)`

#### Bug 5.4: Arithmetic type mismatch
- **Severity:** MEDIUM
- **File:** `kernel/src/drivers/raspberrypi/uart/mod.rs:400`
- **Error:** `E0277: cannot divide 'u64' by 'u32'`
- **Root Cause:** Mixing `u64` and `u32` in arithmetic
- **Fix:** Cast `config.baud_rate` to `u64` or use consistent types

### Category 6: Bootloader Issues (BLOCKING)

#### Bug 6.1: Unsupported calling convention
- **Severity:** BLOCKING
- **File:** `bootloader/src/main.rs:166`
- **Error:** `rustc-LLVM ERROR: Unsupported calling convention.`
- **Root Cause:** Using `extern "sysv64"` calling convention on ARM64
- **Fix:** Use appropriate ARM64 calling convention (`extern "C"` or `extern "aarch64"`)

#### Bug 6.2: UEFI dependency issues
- **Severity:** HIGH
- **Note:** Bootloader depends on UEFI which may not be appropriate for bare-metal ARM64
- **Fix:** Consider using different boot method for ARM64 (e.g., direct boot from RPi firmware)

### Category 7: Warnings and Code Quality Issues (LOW)

#### Bug 7.1: Unused imports and code
- **Severity:** LOW
- **Files:** Throughout codebase
- **Issues:** 106 warnings about unused imports, variables, functions
- **Fix:** Remove unused code or mark with `#[allow(unused)]`

#### Bug 7.2: Configuration warnings
- **Severity:** LOW
- **Files:** Various
- **Issues:** 
  - `warning: profiles for the non root package will be ignored`
  - `warning: unknown and unstable feature specified for '-Ctarget-feature': 'strict-align'`
- **Fix:** Fix Cargo.toml profiles and target features

## Priority Summary

### BLOCKING (Must fix for build to succeed):
1. Architecture porting issues (x86 assembly in ARM64)
2. Missing `alloc` imports
3. Syntax errors (invalid identifiers, duplicate definitions)
4. Bootloader calling convention
5. Type system errors

### HIGH (Critical for functionality):
1. Unstable feature usage
2. Enum definition errors
3. Missing ARM64 implementations

### MEDIUM (Functional issues):
1. Type mismatches
2. Missing trait implementations
3. Arithmetic errors

### LOW (Code quality):
1. Unused code warnings
2. Configuration warnings

## Suggested Fix Strategy

### Phase 1: Architecture Abstraction
1. Create proper conditional compilation for x86_64 vs ARM64
2. Move architecture-specific code to `#[cfg(target_arch)]` blocks
3. Create ARM64 implementations for missing modules

### Phase 2: Dependency Fixes
1. Add `alloc` imports to all filesystem modules
2. Fix missing trait imports (`ToString`, `Error`, etc.)
3. Update Cargo.toml for proper no_std support

### Phase 3: Syntax and Type Fixes
1. Fix invalid identifiers and duplicate definitions
2. Correct pattern matching and enum definitions
3. Fix type mismatches and arithmetic errors

### Phase 4: Bootloader Port
1. Fix calling convention for ARM64
2. Consider alternative boot method for Raspberry Pi
3. Update linker script for ARM64

### Phase 5: Code Cleanup
1. Remove unused code
2. Fix configuration warnings
3. Add proper error handling

## Build Logs
- Kernel build log: `build_logs/kernel_build.log`
- Bootloader build log: `build_logs/bootloader_build.log`
- Full build log: `build_logs/build_aarch64_full.log`

## Next Steps
1. Start with Phase 1 fixes to get basic compilation working
2. Focus on filesystem module imports (Category 2)
3. Fix architecture-specific assembly (Category 1)
4. Address syntax errors (Category 3)
5. Work on bootloader issues last

**Note:** This is a complex porting effort requiring significant changes to the codebase structure and architecture abstraction layer.