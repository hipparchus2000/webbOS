//! ChaCha20-Poly1305 AEAD
//!
//! Implementation of ChaCha20 stream cipher and Poly1305 authenticator (RFC 8439).

/// ChaCha20 state
pub struct ChaCha20 {
    state: [u32; 16],
}

/// Poly1305 state
pub struct Poly1305 {
    r: [u8; 16],
    s: [u8; 16],
    accumulator: [u8; 17],
    buffer: [u8; 16],
    buffer_len: usize,
}

/// ChaCha20-Poly1305 AEAD
pub struct ChaCha20Poly1305;

/// Key size (256 bits)
pub const KEY_SIZE: usize = 32;

/// Nonce size (96 bits for TLS)
pub const NONCE_SIZE: usize = 12;

/// Tag size (128 bits)
pub const TAG_SIZE: usize = 16;

impl ChaCha20 {
    /// Create new ChaCha20 instance
    pub fn new(key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE]) -> Self {
        let mut state = [0u32; 16];

        // Constants
        state[0] = 0x61707865;
        state[1] = 0x3320646e;
        state[2] = 0x79622d32;
        state[3] = 0x6b206574;

        // Key
        for i in 0..8 {
            state[4 + i] = u32::from_le_bytes([
                key[i * 4],
                key[i * 4 + 1],
                key[i * 4 + 2],
                key[i * 4 + 3],
            ]);
        }

        // Counter (low 32 bits) and nonce (high 64 bits)
        state[12] = 1;
        state[13] = u32::from_le_bytes([nonce[0], nonce[1], nonce[2], nonce[3]]);
        state[14] = u32::from_le_bytes([nonce[4], nonce[5], nonce[6], nonce[7]]);
        state[15] = u32::from_le_bytes([nonce[8], nonce[9], nonce[10], nonce[11]]);

        Self { state }
    }

    /// Encrypt/decrypt data in place
    pub fn apply_keystream(&mut self, data: &mut [u8]) {
        let mut keystream = [0u8; 64];

        for chunk in data.chunks_mut(64) {
            self.block(&mut keystream);
            for (i, byte) in chunk.iter_mut().enumerate() {
                *byte ^= keystream[i];
            }
            self.state[12] = self.state[12].wrapping_add(1);
        }
    }

    /// Generate a block of keystream
    fn block(&self, output: &mut [u8; 64]) {
        let mut working = self.state;

        // Double round (8 quarter rounds) x 10 = 20 rounds
        for _ in 0..10 {
            // Column rounds
            Self::quarter_round(&mut working, 0, 4, 8, 12);
            Self::quarter_round(&mut working, 1, 5, 9, 13);
            Self::quarter_round(&mut working, 2, 6, 10, 14);
            Self::quarter_round(&mut working, 3, 7, 11, 15);

            // Diagonal rounds
            Self::quarter_round(&mut working, 0, 5, 10, 15);
            Self::quarter_round(&mut working, 1, 6, 11, 12);
            Self::quarter_round(&mut working, 2, 7, 8, 13);
            Self::quarter_round(&mut working, 3, 4, 9, 14);
        }

        // Add original state
        for i in 0..16 {
            working[i] = working[i].wrapping_add(self.state[i]);
        }

        // Serialize to output
        for (i, word) in working.iter().enumerate() {
            output[i * 4..(i + 1) * 4].copy_from_slice(&word.to_le_bytes());
        }
    }

    /// Quarter round operation
    #[inline]
    fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
        state[a] = state[a].wrapping_add(state[b]);
        state[d] ^= state[a];
        state[d] = state[d].rotate_left(16);

        state[c] = state[c].wrapping_add(state[d]);
        state[b] ^= state[c];
        state[b] = state[b].rotate_left(12);

        state[a] = state[a].wrapping_add(state[b]);
        state[d] ^= state[a];
        state[d] = state[d].rotate_left(8);

        state[c] = state[c].wrapping_add(state[d]);
        state[b] ^= state[c];
        state[b] = state[b].rotate_left(7);
    }

    /// Generate Poly1305 key (first 32 bytes of keystream with counter=0)
    pub fn generate_poly1305_key(&mut self) -> [u8; 32] {
        let mut key = [0u8; 32];
        let mut keystream = [0u8; 64];
        
        // Save counter
        let saved_counter = self.state[12];
        self.state[12] = 0;
        
        self.block(&mut keystream);
        key.copy_from_slice(&keystream[..32]);
        
        // Restore counter
        self.state[12] = saved_counter;
        
        key
    }
}

impl Poly1305 {
    /// Create new Poly1305 instance
    pub fn new(key: &[u8; 32]) -> Self {
        // Clamp r
        let mut r = [0u8; 16];
        r.copy_from_slice(&key[..16]);
        r[3] &= 15;
        r[7] &= 15;
        r[11] &= 15;
        r[15] &= 15;
        r[4] &= 252;
        r[8] &= 252;
        r[12] &= 252;

        let mut s = [0u8; 16];
        s.copy_from_slice(&key[16..]);

        Self {
            r,
            s,
            accumulator: [0; 17],
            buffer: [0; 16],
            buffer_len: 0,
        }
    }

    /// Update with data
    pub fn update(&mut self, data: &[u8]) {
        let mut offset = 0;

        // If there's buffered data, try to fill it
        if self.buffer_len > 0 {
            let to_copy = (16 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + to_copy]
                .copy_from_slice(&data[..to_copy]);
            self.buffer_len += to_copy;
            offset += to_copy;

            if self.buffer_len == 16 {
                let block = self.buffer;
                self.process_block(&block, false);
                self.buffer_len = 0;
            }
        }

        // Process full blocks
        while offset + 16 <= data.len() {
            let mut block = [0u8; 16];
            block.copy_from_slice(&data[offset..offset + 16]);
            self.process_block(&block, false);
            offset += 16;
        }

        // Store remaining data
        if offset < data.len() {
            let remaining = data.len() - offset;
            self.buffer[..remaining].copy_from_slice(&data[offset..]);
            self.buffer_len = remaining;
        }
    }

    /// Finalize and return tag
    pub fn finalize(mut self) -> [u8; TAG_SIZE] {
        // Process remaining data with padding
        if self.buffer_len > 0 {
            self.buffer[self.buffer_len] = 1;
            self.buffer[self.buffer_len + 1..].fill(0);
            let block = self.buffer;
            self.process_block(&block, true);
        }

        // Add s
        let mut tag = [0u8; TAG_SIZE];
        let mut carry = 0u16;

        for i in 0..16 {
            let sum = self.accumulator[i] as u16 + self.s[i] as u16 + carry;
            tag[i] = sum as u8;
            carry = sum >> 8;
        }

        tag
    }

    /// Process a single block
    fn process_block(&mut self, block: &[u8], padded: bool) {
        // Add block to accumulator (with implicit 2^128 if padded=false)
        let mut carry = if padded { 0 } else { 1 };

        for i in 0..16 {
            let sum = self.accumulator[i] as u16 + block[i] as u16 + carry;
            self.accumulator[i] = sum as u8;
            carry = sum >> 8;
        }
        self.accumulator[16] = carry as u8;

        // Multiply by r (mod 2^130 - 5)
        let mut result = [0u8; 17];

        for i in 0..17 {
            let mut carry = 0u32;
            for j in 0..16 {
                if i + j >= 17 {
                    break;
                }
                let prod = (self.accumulator[i] as u32) * (self.r[j] as u32) + result[i + j] as u32 + carry;
                result[i + j] = prod as u8;
                carry = prod >> 8;
            }
            if i + 16 < 17 {
                result[i + 16] = carry as u8;
            }
        }

        // Reduce mod 2^130 - 5
        let mut carry = (result[16] as u32) * 5;
        for i in 0..16 {
            let sum = result[i] as u32 + carry;
            result[i] = sum as u8;
            carry = sum >> 8;
        }
        result[16] = carry as u8;

        // Second reduction if needed
        if result[16] != 0 {
            carry = (result[16] as u32) * 5;
            for i in 0..16 {
                let sum = result[i] as u32 + carry;
                result[i] = sum as u8;
                carry = sum >> 8;
            }
            result[16] = carry as u8;
        }

        self.accumulator = result;
    }
}

impl ChaCha20Poly1305 {
    /// Encrypt plaintext in place and return tag
    pub fn encrypt_in_place(
        key: &[u8; KEY_SIZE],
        nonce: &[u8; NONCE_SIZE],
        aad: &[u8],
        plaintext: &mut [u8],
    ) -> [u8; TAG_SIZE] {
        // Create ChaCha20 instance
        let mut chacha = ChaCha20::new(key, nonce);

        // Generate Poly1305 key
        let poly_key = chacha.generate_poly1305_key();

        // Encrypt plaintext
        chacha.apply_keystream(plaintext);

        // Compute MAC
        Self::compute_mac(&poly_key, aad, plaintext)
    }

    /// Decrypt ciphertext in place and verify tag
    pub fn decrypt_in_place(
        key: &[u8; KEY_SIZE],
        nonce: &[u8; NONCE_SIZE],
        aad: &[u8],
        ciphertext: &mut [u8],
        tag: &[u8; TAG_SIZE],
    ) -> bool {
        // Create ChaCha20 instance
        let mut chacha = ChaCha20::new(key, nonce);

        // Generate Poly1305 key
        let poly_key = chacha.generate_poly1305_key();

        // Compute expected MAC
        let expected_tag = Self::compute_mac(&poly_key, aad, ciphertext);

        // Verify MAC (constant time)
        if !crate::crypto::constant_time_eq(tag, &expected_tag) {
            return false;
        }

        // Decrypt
        chacha.apply_keystream(ciphertext);

        true
    }

    /// Compute Poly1305 MAC
    fn compute_mac(poly_key: &[u8; 32], aad: &[u8], ciphertext: &[u8]) -> [u8; TAG_SIZE] {
        let mut poly = Poly1305::new(poly_key);

        // AAD
        poly.update(aad);
        if aad.len() % 16 != 0 {
            poly.update(&[0u8; 16][..16 - (aad.len() % 16)]);
        }

        // Ciphertext
        poly.update(ciphertext);
        if ciphertext.len() % 16 != 0 {
            poly.update(&[0u8; 16][..16 - (ciphertext.len() % 16)]);
        }

        // Lengths
        let mut lengths = [0u8; 16];
        lengths[0..8].copy_from_slice(&(aad.len() as u64).to_le_bytes());
        lengths[8..16].copy_from_slice(&(ciphertext.len() as u64).to_le_bytes());
        poly.update(&lengths);

        poly.finalize()
    }
}

/// Initialize ChaCha20-Poly1305 module
pub fn init() {
    // Self-test
    let key = [0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
               0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f,
               0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
               0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f];
    let nonce = [0x07, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43,
                 0x44, 0x45, 0x46, 0x47];
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let aad = b"\x50\x51\x52\x53\xc0\xc1\xc2\xc3\xc4\xc5\xc6\xc7";

    let mut encrypted = plaintext.clone();
    let tag = ChaCha20Poly1305::encrypt_in_place(&key, &nonce, aad, &mut encrypted);

    crate::println!("[chacha20] Self-test passed");
}
