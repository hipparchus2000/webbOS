//! SHA-1 Hash Function
//!
//! Implementation of the SHA-1 cryptographic hash function (FIPS 180-4).
//! Used for WPA2-PSK key derivation (PBKDF2-HMAC-SHA1).

#![allow(dead_code)]

/// SHA-1 digest size in bytes
pub const DIGEST_SIZE: usize = 20;

/// SHA-1 block size in bytes
pub const BLOCK_SIZE: usize = 64;

/// SHA-1 state
pub struct Sha1 {
    state: [u32; 5],
    buffer: [u8; BLOCK_SIZE],
    buffer_len: usize,
    total_len: u64,
}

/// Initial hash values
const H: [u32; 5] = [
    0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0,
];

impl Sha1 {
    /// Create new SHA-1 hasher
    pub fn new() -> Self {
        Self {
            state: H,
            buffer: [0; BLOCK_SIZE],
            buffer_len: 0,
            total_len: 0,
        }
    }

    /// Update hash with data
    pub fn update(&mut self, data: &[u8]) {
        self.total_len += data.len() as u64;

        let mut data_offset = 0;

        // If there's data in the buffer, try to fill it
        if self.buffer_len > 0 {
            let to_copy = (BLOCK_SIZE - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + to_copy]
                .copy_from_slice(&data[..to_copy]);
            self.buffer_len += to_copy;
            data_offset += to_copy;

            // If buffer is full, process it
            if self.buffer_len == BLOCK_SIZE {
                let block = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
            }
        }

        // Process full blocks from remaining data
        while data_offset + BLOCK_SIZE <= data.len() {
            let mut block = [0u8; BLOCK_SIZE];
            block.copy_from_slice(&data[data_offset..data_offset + BLOCK_SIZE]);
            self.process_block(&block);
            data_offset += BLOCK_SIZE;
        }

        // Store remaining data in buffer
        if data_offset < data.len() {
            let remaining = data.len() - data_offset;
            self.buffer[..remaining].copy_from_slice(&data[data_offset..]);
            self.buffer_len = remaining;
        }
    }

    /// Finalize and return digest
    pub fn finalize(mut self) -> [u8; DIGEST_SIZE] {
        let bit_len = self.total_len * 8;
        
        // Append 0x80
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        // If there's not enough space for the length, process and reset
        if self.buffer_len > BLOCK_SIZE - 8 {
            self.buffer[self.buffer_len..].fill(0);
            let block = self.buffer;
            self.process_block(&block);
            self.buffer.fill(0);
            self.buffer_len = 0;
        } else {
            self.buffer[self.buffer_len..BLOCK_SIZE - 8].fill(0);
        }

        // Append length (big-endian)
        let len_bytes = bit_len.to_be_bytes();
        self.buffer[BLOCK_SIZE - 8..].copy_from_slice(&len_bytes);
        let block = self.buffer;
        self.process_block(&block);

        // Convert state to bytes
        let mut digest = [0u8; DIGEST_SIZE];
        for (i, &word) in self.state.iter().enumerate() {
            digest[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
        }

        digest
    }

    /// Process a single 64-byte block
    fn process_block(&mut self, block: &[u8]) {
        let mut w = [0u32; 80];
        
        // Copy block into first 16 words
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }

        // Extend to 80 words
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        // Initialize working variables
        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];

        // Main loop
        for i in 0..80 {
            let (f, k) = if i < 20 {
                ((b & c) | ((!b) & d), 0x5a827999)
            } else if i < 40 {
                (b ^ c ^ d, 0x6ed9eba1)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 0x8f1bbcdc)
            } else {
                (b ^ c ^ d, 0xca62c1d6)
            };

            let temp = a.rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        // Add to state
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute SHA-1 hash of data
pub fn hash(data: &[u8]) -> [u8; DIGEST_SIZE] {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finalize()
}

/// HMAC-SHA-1
pub fn hmac(key: &[u8], data: &[u8]) -> [u8; DIGEST_SIZE] {
    const BLOCK_SIZE: usize = 64;
    
    let mut k = [0u8; BLOCK_SIZE];
    if key.len() <= BLOCK_SIZE {
        k[..key.len()].copy_from_slice(key);
    } else {
        let key_hash = hash(key);
        k[..DIGEST_SIZE].copy_from_slice(&key_hash);
    }

    let mut inner = k;
    let mut outer = k;
    for i in 0..BLOCK_SIZE {
        inner[i] ^= 0x36;
        outer[i] ^= 0x5c;
    }

    let mut inner_hasher = Sha1::new();
    inner_hasher.update(&inner);
    inner_hasher.update(data);
    let inner_hash = inner_hasher.finalize();

    let mut outer_hasher = Sha1::new();
    outer_hasher.update(&outer);
    outer_hasher.update(&inner_hash);
    outer_hasher.finalize()
}

/// Initialize SHA-1 module
pub fn init() {
    // Self-test with known test vector
    let result = hash(b"abc");
    let expected = [
        0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a,
        0xba, 0x3e, 0x25, 0x71, 0x78, 0x50, 0xc2, 0x6c,
        0x9c, 0xd0, 0xd8, 0x9d,
    ];
    
    if result == expected {
        crate::println!("[sha1] Self-test passed");
    } else {
        crate::println!("[sha1] Self-test FAILED");
    }
}
