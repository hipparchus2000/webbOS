//! Cryptographic primitives
//!
//! Implements cryptographic algorithms needed for TLS 1.3:
//! - SHA-256, SHA-384 hash functions
//! - AES-GCM AEAD cipher
//! - ChaCha20-Poly1305 AEAD cipher
//! - HKDF key derivation
//! - X25519 key exchange

pub mod sha256;
pub mod sha384;
pub mod aes;
pub mod chacha20;
pub mod hkdf;
pub mod x25519;

use crate::println;

/// Initialize cryptographic subsystem
pub fn init() {
    println!("[crypto] Initializing cryptographic subsystem...");
    
    sha256::init();
    sha384::init();
    aes::init();
    chacha20::init();
    hkdf::init();
    x25519::init();
    
    println!("[crypto] Cryptographic subsystem initialized");
}

/// Constant-time comparison of two byte slices
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0u8;
    for i in 0..a.len() {
        result |= a[i] ^ b[i];
    }
    
    result == 0
}

/// XOR two byte slices in place
pub fn xor_in_place(a: &mut [u8], b: &[u8]) {
    let len = a.len().min(b.len());
    for i in 0..len {
        a[i] ^= b[i];
    }
}

/// Securely clear memory
pub fn secure_clear(buf: &mut [u8]) {
    for byte in buf.iter_mut() {
        *byte = 0;
    }
    // Prevent optimization
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}
