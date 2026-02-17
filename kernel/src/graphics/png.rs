//! PNG Decoder for WebbOS
//!
//! A minimal PNG decoder supporting:
//! - RGB (color type 2) and RGBA (color type 6)
//! - 8-bit depth
//! - Non-interlaced images
//! - Basic zlib decompression

use alloc::vec::Vec;
use alloc::vec;
use alloc::string::{String, ToString};
use alloc::format;

/// PNG decoding error
#[derive(Debug, Clone, PartialEq)]
pub enum PngError {
    InvalidSignature,
    InvalidHeader,
    UnsupportedFormat(&'static str),
    DecompressionError,
    InvalidData,
    IoError(String),
}

/// Decoded PNG image
#[derive(Debug, Clone)]
pub struct PngImage {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>, // Raw RGBA pixels
}

impl PngImage {
    /// Create empty image
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba_data: vec![0; (width * height * 4) as usize],
        }
    }

    /// Get pixel at (x, y) as RGBA tuple
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        if idx + 3 >= self.rgba_data.len() {
            return None;
        }
        Some((
            self.rgba_data[idx],
            self.rgba_data[idx + 1],
            self.rgba_data[idx + 2],
            self.rgba_data[idx + 3],
        ))
    }
}

/// PNG signature
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Chunk type constants
const IHDR: [u8; 4] = *b"IHDR";
const IDAT: [u8; 4] = *b"IDAT";
const IEND: [u8; 4] = *b"IEND";

/// Decode a PNG image from bytes
pub fn decode_png(data: &[u8]) -> Result<PngImage, PngError> {
    // Check signature
    if data.len() < 8 || &data[0..8] != &PNG_SIGNATURE[..] {
        return Err(PngError::InvalidSignature);
    }

    let mut pos = 8;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bit_depth = 0u8;
    let mut color_type = 0u8;
    let mut compression = 0u8;
    let mut filter_method = 0u8;
    let mut interlace = 0u8;
    let mut compressed_data = Vec::new();

    // Parse chunks
    while pos + 12 <= data.len() {
        // Read chunk length (big-endian)
        let chunk_len = u32::from_be_bytes([
            data[pos], data[pos + 1], data[pos + 2], data[pos + 3]
        ]) as usize;
        
        if pos + 12 + chunk_len > data.len() {
            return Err(PngError::InvalidData);
        }

        let chunk_type = &data[pos + 4..pos + 8];
        let chunk_data = &data[pos + 8..pos + 8 + chunk_len];
        // CRC is at pos + 8 + chunk_len (4 bytes), skip for now

        if chunk_type == &IHDR[..] {
            if chunk_len != 13 {
                return Err(PngError::InvalidHeader);
            }
            width = u32::from_be_bytes([chunk_data[0], chunk_data[1], chunk_data[2], chunk_data[3]]);
            height = u32::from_be_bytes([chunk_data[4], chunk_data[5], chunk_data[6], chunk_data[7]]);
            bit_depth = chunk_data[8];
            color_type = chunk_data[9];
            compression = chunk_data[10];
            filter_method = chunk_data[11];
            interlace = chunk_data[12];

            // Validate supported formats
            if bit_depth != 8 {
                return Err(PngError::UnsupportedFormat("only 8-bit depth supported"));
            }
            if color_type != 2 && color_type != 6 {
                return Err(PngError::UnsupportedFormat("only RGB/RGBA color types supported"));
            }
            if compression != 0 {
                return Err(PngError::UnsupportedFormat("only deflate compression supported"));
            }
            if filter_method != 0 {
                return Err(PngError::UnsupportedFormat("only adaptive filtering supported"));
            }
            if interlace != 0 {
                return Err(PngError::UnsupportedFormat("interlaced PNGs not supported"));
            }
        } else if chunk_type == &IDAT[..] {
            compressed_data.extend_from_slice(chunk_data);
        } else if chunk_type == &IEND[..] {
            break;
        }

        pos += 12 + chunk_len;
    }

    if width == 0 || height == 0 {
        return Err(PngError::InvalidHeader);
    }

    if compressed_data.is_empty() {
        return Err(PngError::InvalidData);
    }

    // Decompress image data
    let raw_data = decompress_zlib(&compressed_data)?;

    // Unfilter and convert to RGBA
    let rgba_data = unfilter_and_convert(&raw_data, width, height, color_type)?;

    Ok(PngImage {
        width,
        height,
        rgba_data,
    })
}

/// Minimal zlib decompressor for PNG
/// This is a simplified implementation that handles basic cases
fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, PngError> {
    if data.len() < 2 {
        return Err(PngError::DecompressionError);
    }

    // Check zlib header
    let cmf = data[0];
    let flg = data[1];
    
    // Check header validity
    if (cmf & 0x0F) != 8 {
        // Not deflate compression
        return Err(PngError::DecompressionError);
    }
    
    // Check CMF+FLG checksum
    if ((cmf as u16) * 256 + (flg as u16)) % 31 != 0 {
        return Err(PngError::DecompressionError);
    }

    // Skip header and decompress
    let compressed = &data[2..data.len() - 4]; // Skip header and adler32 trailer
    
    // Simple inflate implementation for uncompressed blocks and fixed huffman
    inflate(compressed)
}

/// Minimal inflate implementation
fn inflate(data: &[u8]) -> Result<Vec<u8>, PngError> {
    let mut output = Vec::new();
    let mut pos = 0;
    let mut bit_pos = 0;

    loop {
        if pos >= data.len() {
            break;
        }

        // Read block header
        let bfinal = read_bits(data, &mut pos, &mut bit_pos, 1)?;
        let btype = read_bits(data, &mut pos, &mut bit_pos, 2)?;

        match btype {
            0 => {
                // Stored block (uncompressed)
                // Align to byte boundary
                bit_pos = 0;
                pos += 1;
                
                if pos + 4 > data.len() {
                    return Err(PngError::DecompressionError);
                }
                
                let len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
                let nlen = u16::from_le_bytes([data[pos + 2], data[pos + 3]]) as usize;
                
                if len != (!nlen & 0xFFFF) {
                    return Err(PngError::DecompressionError);
                }
                
                pos += 4;
                
                if pos + len > data.len() {
                    return Err(PngError::DecompressionError);
                }
                
                output.extend_from_slice(&data[pos..pos + len]);
                pos += len;
            }
            1 => {
                // Fixed Huffman - not implemented in this minimal version
                return Err(PngError::UnsupportedFormat("fixed huffman blocks not implemented"));
            }
            2 => {
                // Dynamic Huffman - not implemented in this minimal version
                return Err(PngError::UnsupportedFormat("dynamic huffman blocks not implemented"));
            }
            _ => {
                return Err(PngError::DecompressionError);
            }
        }

        if bfinal == 1 {
            break;
        }
    }

    Ok(output)
}

/// Read bits from data stream
fn read_bits(data: &[u8], pos: &mut usize, bit_pos: &mut usize, count: usize) -> Result<u32, PngError> {
    let mut result = 0u32;
    
    for _ in 0..count {
        if *pos >= data.len() {
            return Err(PngError::DecompressionError);
        }
        
        let bit = (data[*pos] >> *bit_pos) & 1;
        result = (result << 1) | bit as u32;
        
        *bit_pos += 1;
        if *bit_pos >= 8 {
            *bit_pos = 0;
            *pos += 1;
        }
    }
    
    Ok(result)
}

/// Unfilter scanlines and convert to RGBA
fn unfilter_and_convert(raw_data: &[u8], width: u32, height: u32, color_type: u8) -> Result<Vec<u8>, PngError> {
    let bytes_per_pixel = if color_type == 6 { 4 } else { 3 };
    let stride = (width as usize * bytes_per_pixel) + 1; // +1 for filter byte
    
    if raw_data.len() < stride * height as usize {
        return Err(PngError::InvalidData);
    }

    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let mut prev_line = vec![0u8; width as usize * bytes_per_pixel];

    for y in 0..height as usize {
        let line_start = y * stride;
        let filter = raw_data[line_start];
        let line_data = &raw_data[line_start + 1..line_start + stride];

        // Unfilter
        let mut unfiltered = vec![0u8; width as usize * bytes_per_pixel];
        for x in 0..width as usize * bytes_per_pixel {
            let filtered = line_data[x];
            let unfilter_val = match filter {
                0 => filtered, // None
                1 => {
                    // Sub
                    let left = if x >= bytes_per_pixel { unfiltered[x - bytes_per_pixel] } else { 0 };
                    filtered.wrapping_add(left)
                }
                2 => {
                    // Up
                    filtered.wrapping_add(prev_line[x])
                }
                3 => {
                    // Average
                    let left = if x >= bytes_per_pixel { unfiltered[x - bytes_per_pixel] } else { 0 };
                    let up = prev_line[x];
                    filtered.wrapping_add(((left as u16 + up as u16) / 2) as u8)
                }
                4 => {
                    // Paeth
                    let left = if x >= bytes_per_pixel { unfiltered[x - bytes_per_pixel] } else { 0 };
                    let up = prev_line[x];
                    let left_up = if x >= bytes_per_pixel { prev_line[x - bytes_per_pixel] } else { 0 };
                    filtered.wrapping_add(paeth_predictor(left, up, left_up))
                }
                _ => return Err(PngError::InvalidData),
            };
            unfiltered[x] = unfilter_val;
        }

        // Convert to RGBA
        for x in 0..width as usize {
            let src_idx = x * bytes_per_pixel;
            let dst_idx = (y * width as usize + x) * 4;
            
            rgba[dst_idx] = unfiltered[src_idx];       // R
            rgba[dst_idx + 1] = unfiltered[src_idx + 1]; // G
            rgba[dst_idx + 2] = unfiltered[src_idx + 2]; // B
            rgba[dst_idx + 3] = if bytes_per_pixel == 4 { 
                unfiltered[src_idx + 3] // A
            } else { 
                255 // Fully opaque
            };
        }

        prev_line.copy_from_slice(&unfiltered);
    }

    Ok(rgba)
}

/// Paeth predictor for PNG filter type 4
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i16;
    let b = b as i16;
    let c = c as i16;
    
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

/// Check if data is a valid PNG
pub fn is_png(data: &[u8]) -> bool {
    data.len() >= 8 && &data[0..8] == &PNG_SIGNATURE[..]
}

/// Load PNG from filesystem (stub - returns error until VFS is fully integrated)
pub fn load_png_from_file(path: &str) -> Result<PngImage, PngError> {
    // For now, try to use the boot_disk module if available
    if let Some(data) = crate::fs::boot_disk::read_file(path) {
        decode_png(&data)
    } else {
        Err(PngError::IoError(format!("File not found: {}", path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_png() {
        let png_sig = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(is_png(&png_sig));
        
        let not_png = vec![0x00, 0x00, 0x00, 0x00];
        assert!(!is_png(&not_png));
    }
}
