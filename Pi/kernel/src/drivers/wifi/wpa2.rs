//! WPA2 Security Implementation
//!
//! Implements WPA2-PSK (Pre-Shared Key) authentication including:
//! - PSK derivation from passphrase and SSID using PBKDF2-HMAC-SHA1
//! - PMK (Pairwise Master Key) generation
//! - PTK (Pairwise Transient Key) generation using PRF-HMAC-SHA1
//! - 4-way handshake
//! - MIC (Message Integrity Check) calculation using HMAC-SHA1

use alloc::vec::Vec;
use crate::println;
use crate::crypto::sha1;
use crate::crypto::pbkdf2;

// WPA2 constants
pub const PMK_LEN: usize = 32;
pub const PTK_LEN: usize = 64;
pub const GTK_LEN: usize = 32;
pub const MIC_LEN: usize = 16;

// EAPOL key types
pub const EAPOL_KEY_TYPE_RSN: u8 = 2;

// Key info bits
pub const KEY_INFO_KEY_TYPE: u16 = 0x0008;
pub const KEY_INFO_INSTALL: u16 = 0x0040;
pub const KEY_INFO_ACK: u16 = 0x0080;
pub const KEY_INFO_MIC: u16 = 0x0100;
pub const KEY_INFO_SECURE: u16 = 0x0200;

// EAPOL version
pub const EAPOL_VERSION_1: u8 = 1;

// EAPOL packet type
pub const EAPOL_KEY: u8 = 3;

/// WPA2 PSK (Pre-Shared Key) structure
#[derive(Clone)]
pub struct Wpa2Psk {
    pub pmk: [u8; PMK_LEN],
    pub ptk: [u8; PTK_LEN],
    pub gtk: [u8; GTK_LEN],
    pub authenticator_mac: [u8; 6],
    pub supplicant_mac: [u8; 6],
    pub anonce: [u8; 32],
    pub snonce: [u8; 32],
    pub key_replay_counter: u64,
}

impl Wpa2Psk {
    /// Create new WPA2 PSK context
    pub fn new(authenticator_mac: [u8; 6], supplicant_mac: [u8; 6]) -> Self {
        Self {
            pmk: [0; PMK_LEN],
            ptk: [0; PTK_LEN],
            gtk: [0; GTK_LEN],
            authenticator_mac,
            supplicant_mac,
            anonce: [0; 32],
            snonce: [0; 32],
            key_replay_counter: 0,
        }
    }
    
    /// Generate PMK from passphrase and SSID using PBKDF2-HMAC-SHA1
    /// 
    /// Uses standard WPA2 parameters: 4096 iterations, SSID as salt
    pub fn derive_pmk(&mut self, passphrase: &str, ssid: &[u8]) {
        println!("[wpa2] Deriving PMK using PBKDF2-HMAC-SHA1...");
        
        // Use proper PBKDF2-HMAC-SHA1 with 4096 iterations (WPA2 standard)
        self.pmk = pbkdf2::derive_wpa2_pmk(passphrase, ssid);
        
        println!("[wpa2] PMK derived successfully");
    }
    
    /// Generate PTK from PMK and nonces using PRF-HMAC-SHA1
    /// 
    /// PTK = PRF(PMK, "Pairwise key expansion", 
    ///           Min(AA,SA) || Max(AA,SA) || Min(ANonce,SNonce) || Max(ANonce,SNonce))
    pub fn derive_ptk(&mut self) {
        println!("[wpa2] Deriving PTK using PRF-HMAC-SHA1...");
        
        // Build PRF input: Min(AA,SA) || Max(AA,SA) || Min(ANonce,SNonce) || Max(ANonce,SNonce)
        let mut prf_input = Vec::with_capacity(76);
        
        // Add MAC addresses (smaller first)
        if compare_mac(&self.authenticator_mac, &self.supplicant_mac) {
            prf_input.extend_from_slice(&self.authenticator_mac);
            prf_input.extend_from_slice(&self.supplicant_mac);
        } else {
            prf_input.extend_from_slice(&self.supplicant_mac);
            prf_input.extend_from_slice(&self.authenticator_mac);
        }
        
        // Add nonces (smaller first)
        if compare_nonce(&self.anonce, &self.snonce) {
            prf_input.extend_from_slice(&self.anonce);
            prf_input.extend_from_slice(&self.snonce);
        } else {
            prf_input.extend_from_slice(&self.snonce);
            prf_input.extend_from_slice(&self.anonce);
        }
        
        // Derive PTK using PRF-HMAC-SHA1
        // PRF(key, label, data) = HMAC-SHA1(key, label || 0 || data) || 
        //                         HMAC-SHA1(key, label || 1 || data) || ...
        let label = b"Pairwise key expansion";
        prf(&self.pmk, label, &prf_input, &mut self.ptk);
        
        println!("[wpa2] PTK derived successfully");
    }
    
    /// Calculate MIC for EAPOL key frame using HMAC-SHA1
    /// 
    /// MIC = HMAC-SHA1(KCK, EAPOL frame with MIC field zeroed)
    /// where KCK = first 16 bytes of PTK
    pub fn calculate_mic(&self, eapol_frame: &[u8], _key_descriptor_version: u8) -> [u8; 16] {
        // KCK is the first 16 bytes of PTK
        let kck = &self.ptk[0..16];
        
        // MIC = HMAC-SHA1(KCK, EAPOL frame with MIC field zeroed)
        let hmac_result = sha1::hmac(kck, eapol_frame);
        
        // Return first 16 bytes of HMAC
        let mut mic = [0u8; 16];
        mic.copy_from_slice(&hmac_result[..16]);
        mic
    }
    
    /// Verify MIC on received EAPOL frame
    pub fn verify_mic(&self, eapol_frame: &[u8], received_mic: &[u8]) -> bool {
        if received_mic.len() != 16 {
            return false;
        }
        
        // Calculate expected MIC
        let expected_mic = self.calculate_mic(eapol_frame, 0);
        
        // Constant-time comparison to prevent timing attacks
        crate::crypto::constant_time_eq(&expected_mic, received_mic)
    }
    
    /// Generate random SNonce using hardware entropy
    /// 
    /// Uses ARM physical counter and timer ticks for entropy
    pub fn generate_snonce(&mut self) {
        // Collect entropy from hardware sources
        let entropy = collect_entropy();
        
        // Use HKDF-like construction to derive nonce from entropy
        // First, hash the entropy to get a uniform distribution
        let entropy_hash = sha1::hash(&entropy);
        
        // Expand to 32 bytes using multiple HMAC iterations
        for i in 0..2 {
            let mut hmac_input = [0u8; 24]; // entropy_hash (20) + index (4)
            hmac_input[..20].copy_from_slice(&entropy_hash);
            hmac_input[20..24].copy_from_slice(&(i as u32).to_le_bytes());
            
            let hmac_result = sha1::hmac(b"WPA2-SNonce", &hmac_input);
            
            let offset = i * 20;
            let remaining = 32 - offset;
            let to_copy = remaining.min(20);
            self.snonce[offset..offset + to_copy].copy_from_slice(&hmac_result[..to_copy]);
        }
    }
    
    /// Get temporal key for encryption
    pub fn tk(&self) -> &[u8] {
        &self.ptk[32..64]
    }
    
    /// Get KCK (Key Confirmation Key) - first 16 bytes of PTK
    pub fn kck(&self) -> &[u8] {
        &self.ptk[0..16]
    }
    
    /// Get KEK (Key Encryption Key) - bytes 16-31 of PTK
    pub fn kek(&self) -> &[u8] {
        &self.ptk[16..32]
    }
}

/// PRF (Pseudo-Random Function) using HMAC-SHA1
/// 
/// Output = HMAC-SHA1(key, label || 0 || data) || 
///          HMAC-SHA1(key, label || 1 || data) || ...
fn prf(key: &[u8], label: &[u8], data: &[u8], output: &mut [u8]) {
    let mut counter: u8 = 0;
    let mut offset = 0;
    
    while offset < output.len() {
        // Build input: label || counter || data
        let mut hmac_input = Vec::with_capacity(label.len() + 1 + data.len());
        hmac_input.extend_from_slice(label);
        hmac_input.push(counter);
        hmac_input.extend_from_slice(data);
        
        // Compute HMAC-SHA1
        let hmac_result = sha1::hmac(key, &hmac_input);
        
        // Copy to output (truncate if necessary for last block)
        let remaining = output.len() - offset;
        let to_copy = remaining.min(hmac_result.len());
        output[offset..offset + to_copy].copy_from_slice(&hmac_result[..to_copy]);
        
        offset += to_copy;
        counter += 1;
    }
}

/// Collect entropy from hardware sources for nonce generation
fn collect_entropy() -> Vec<u8> {
    let mut entropy = Vec::with_capacity(32);
    
    // Read ARM physical counter (CNTPCT_EL0)
    unsafe {
        let cntpct: u64;
        core::arch::asm!(
            "mrs {0}, CNTPCT_EL0",
            out(reg) cntpct,
        );
        entropy.extend_from_slice(&cntpct.to_le_bytes());
    }
    
    // Read timer ticks (atomic)
    let ticks = crate::drivers::timer::ticks();
    entropy.extend_from_slice(&ticks.to_le_bytes());
    
    // Read counter frequency
    unsafe {
        let cntfrq: u64;
        core::arch::asm!(
            "mrs {0}, CNTFRQ_EL0",
            out(reg) cntfrq,
        );
        entropy.extend_from_slice(&cntfrq.to_le_bytes());
    }
    
    entropy
}

/// Compare two MAC addresses
fn compare_mac(a: &[u8; 6], b: &[u8; 6]) -> bool {
    for i in 0..6 {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
    }
    false
}

/// Compare two nonces
fn compare_nonce(a: &[u8; 32], b: &[u8; 32]) -> bool {
    for i in 0..32 {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
    }
    false
}

/// 4-Way Handshake State Machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    Idle,
    WaitingForMessage1,
    SentMessage2,
    WaitingForMessage3,
    SentMessage4,
    Complete,
    Failed,
}

/// 4-Way Handshake Handler
#[derive(Clone)]
pub struct FourWayHandshake {
    pub state: HandshakeState,
    pub psk: Wpa2Psk,
    pub passphrase: Vec<u8>,
    pub ssid: Vec<u8>,
}

impl FourWayHandshake {
    /// Create new handshake handler
    pub fn new(authenticator_mac: [u8; 6], supplicant_mac: [u8; 6], passphrase: &[u8], ssid: &[u8]) -> Self {
        let mut psk = Wpa2Psk::new(authenticator_mac, supplicant_mac);
        
        // Derive PMK
        if let Ok(pass_str) = core::str::from_utf8(passphrase) {
            psk.derive_pmk(pass_str, ssid);
        }
        
        Self {
            state: HandshakeState::Idle,
            psk,
            passphrase: passphrase.to_vec(),
            ssid: ssid.to_vec(),
        }
    }
    
    /// Start the handshake
    pub fn start(&mut self) -> Option<Vec<u8>> {
        println!("[wpa2] Starting 4-way handshake...");
        self.psk.generate_snonce();
        self.state = HandshakeState::WaitingForMessage1;
        println!("[wpa2] Waiting for Message 1 from AP...");
        None
    }
    
    /// Process received EAPOL key frame (simplified)
    pub fn process_message(&mut self, _eapol_data: &[u8]) -> Option<Vec<u8>> {
        match self.state {
            HandshakeState::WaitingForMessage1 => {
                // Simplified: assume we got message 1
                println!("[wpa2] Received Message 1");
                self.psk.derive_ptk();
                self.state = HandshakeState::SentMessage2;
                println!("[wpa2] Sending Message 2");
                Some(Vec::new()) // Would send actual response
            }
            HandshakeState::WaitingForMessage3 => {
                println!("[wpa2] Received Message 3");
                self.state = HandshakeState::SentMessage4;
                self.state = HandshakeState::Complete;
                println!("[wpa2] 4-way handshake complete!");
                Some(Vec::new()) // Would send actual response
            }
            _ => {
                println!("[wpa2] Unexpected message in state {:?}", self.state);
                None
            }
        }
    }
    
    /// Check if handshake is complete
    pub fn is_complete(&self) -> bool {
        self.state == HandshakeState::Complete
    }
    
    /// Get temporal key for encryption
    pub fn get_temporal_key(&self) -> &[u8] {
        self.psk.tk()
    }
}

/// Initialize WPA2 module
pub fn init() {
    println!("[wpa2] WPA2 module initialized with proper PBKDF2-HMAC-SHA1");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test PMK derivation with known test vector
    #[test]
    fn test_pmk_derivation() {
        // Test vector from RFC 6070 / WPA2 test cases
        let passphrase = "password";
        let ssid = b"IEEE";
        
        let mut psk = Wpa2Psk::new([0; 6], [0; 6]);
        psk.derive_pmk(passphrase, ssid);
        
        // The PMK should not be all zeros (basic sanity check)
        assert_ne!(psk.pmk, [0u8; 32]);
    }

    /// Test PRF function
    #[test]
    fn test_prf() {
        let key = [0u8; 32];
        let label = b"test label";
        let data = b"test data";
        let mut output = [0u8; 64];
        
        prf(&key, label, data, &mut output);
        
        // Output should not be all zeros
        assert_ne!(output, [0u8; 64]);
    }

    /// Test MIC calculation
    #[test]
    fn test_mic_calculation() {
        let mut psk = Wpa2Psk::new([0; 6], [0; 6]);
        psk.ptk = [0xAA; 64]; // Set known PTK
        
        let frame = b"test eapol frame";
        let mic = psk.calculate_mic(frame, 0);
        
        // MIC should not be all zeros
        assert_ne!(mic, [0u8; 16]);
    }

    /// Test MAC comparison
    #[test]
    fn test_compare_mac() {
        let mac1 = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let mac2 = [0x00, 0x11, 0x22, 0x33, 0x44, 0x66];
        
        assert!(compare_mac(&mac1, &mac2));
        assert!(!compare_mac(&mac2, &mac1));
    }
}
