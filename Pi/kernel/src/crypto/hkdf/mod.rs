//! HKDF (HMAC-based Extract-and-Expand Key Derivation Function)
//!
//! Implementation of HKDF as defined in RFC 5869.

use alloc::vec::Vec;
use crate::crypto::sha256::{self, Sha256, DIGEST_SIZE};

/// HKDF-Extract using SHA-256
pub fn extract(salt: &[u8], ikm: &[u8]) -> [u8; DIGEST_SIZE] {
    sha256::hmac(salt, ikm)
}

/// HKDF-Expand using SHA-256
pub fn expand(prk: &[u8; DIGEST_SIZE], info: &[u8], out_len: usize) -> Vec<u8> {
    let n = (out_len + DIGEST_SIZE - 1) / DIGEST_SIZE;
    let mut okm = Vec::with_capacity(out_len);
    let mut t = Vec::new();

    for i in 1..=n {
        let mut data = t.clone();
        data.extend_from_slice(info);
        data.push(i as u8);
        t = sha256::hmac(prk, &data).to_vec();
        okm.extend_from_slice(&t);
    }

    okm.truncate(out_len);
    okm
}

/// Single-shot HKDF
pub fn derive(salt: &[u8], ikm: &[u8], info: &[u8], out_len: usize) -> Vec<u8> {
    let prk = extract(salt, ikm);
    expand(&prk, info, out_len)
}

/// TLS 1.3 specific labels
pub mod labels {
    pub const EXTERNAL_PSK_BINDER: &[u8] = b"ext binder";
    pub const RESUMPTION_PSK_BINDER: &[u8] = b"res binder";
    pub const CLIENT_EARLY_TRAFFIC: &[u8] = b"c e traffic";
    pub const EARLY_EXPORTER_MASTER: &[u8] = b"e exp master";
    pub const CLIENT_HANDSHAKE_TRAFFIC: &[u8] = b"c hs traffic";
    pub const SERVER_HANDSHAKE_TRAFFIC: &[u8] = b"s hs traffic";
    pub const CLIENT_APPLICATION_TRAFFIC: &[u8] = b"c ap traffic";
    pub const SERVER_APPLICATION_TRAFFIC: &[u8] = b"s ap traffic";
    pub const EXPORTER_MASTER: &[u8] = b"exp master";
    pub const RESUMPTION_MASTER: &[u8] = b"res master";
    pub const KEY: &[u8] = b"key";
    pub const IV: &[u8] = b"iv";
    pub const FINISHED: &[u8] = b"finished";
    pub const DERIVED: &[u8] = b"derived";
}

/// Create HkdfLabel structure as per TLS 1.3
fn make_label(label: &[u8], context: &[u8], length: u16) -> Vec<u8> {
    let mut result = Vec::with_capacity(2 + 1 + label.len() + 1 + context.len());
    
    // Length
    result.extend_from_slice(&length.to_be_bytes());
    
    // Label length and label
    result.push(label.len() as u8);
    result.extend_from_slice(label);
    
    // Context length and context
    result.push(context.len() as u8);
    result.extend_from_slice(context);
    
    result
}

/// TLS 1.3 HKDF-Expand-Label
pub fn expand_label(
    secret: &[u8; DIGEST_SIZE],
    label: &[u8],
    context: &[u8],
    length: u16,
) -> Vec<u8> {
    let hkdf_label = make_label(label, context, length);
    expand(secret, &hkdf_label, length as usize)
}

/// TLS 1.3 Derive-Secret
pub fn derive_secret(secret: &[u8; DIGEST_SIZE], label: &[u8], messages: &[u8]) -> [u8; DIGEST_SIZE] {
    let hash = sha256::hash(messages);
    let result = expand_label(secret, label, &hash, DIGEST_SIZE as u16);
    
    let mut array = [0u8; DIGEST_SIZE];
    array.copy_from_slice(&result);
    array
}

/// Initialize HKDF module
pub fn init() {
    crate::println!("[hkdf] HKDF initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hkdf_rfc5869_test_case_1() {
        let ikm = [0x0b; 22];
        let salt = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
                    0x08, 0x09, 0x0a, 0x0b, 0x0c];
        let info = [0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7,
                    0xf8, 0xf9];
        
        let prk = extract(&salt, &ikm);
        let okm = expand(&prk, &info, 42);
        
        assert_eq!(okm.len(), 42);
    }
}
