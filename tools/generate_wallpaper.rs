//! Generate a sample BMP wallpaper for WebbOS
//! 
//! Usage: rustc generate_wallpaper.rs -o generate_wallpaper && ./generate_wallpaper

use std::fs::File;
use std::io::Write;

fn main() {
    let width: u32 = 1280;
    let height: u32 = 800;
    
    // Create a nice blue-purple gradient
    let color1 = (0x1a, 0x1a, 0x2e); // Dark blue (top)
    let color2 = (0x16, 0x21, 0x3e); // Slightly lighter blue (bottom)
    
    let mut pixels: Vec<u8> = Vec::with_capacity((width * height * 3) as usize);
    
    for y in 0..height {
        let t = y as f32 / height as f32;
        let r = (color1.0 as f32 + (color2.0 as f32 - color1.0 as f32) * t) as u8;
        let g = (color1.1 as f32 + (color2.1 as f32 - color1.1 as f32) * t) as u8;
        let b = (color1.2 as f32 + (color2.2 as f32 - color1.2 as f32) * t) as u8;
        
        for _ in 0..width {
            pixels.push(b); // B
            pixels.push(g); // G
            pixels.push(r); // R
        }
    }
    
    // BMP is stored bottom-up, so we need to reverse the rows
    let mut flipped_pixels: Vec<u8> = Vec::with_capacity(pixels.len());
    let row_size = (width * 3 + 3) & !3; // Pad to 4-byte boundary
    
    for y in (0..height).rev() {
        let start = (y * width * 3) as usize;
        let end = start + (width * 3) as usize;
        flipped_pixels.extend_from_slice(&pixels[start..end]);
        // Add padding
        for _ in (width * 3)..row_size {
            flipped_pixels.push(0);
        }
    }
    
    // BMP Header (14 bytes)
    let file_header: Vec<u8> = vec![
        b'B', b'M', // Signature
        0, 0, 0, 0, // File size (will fill later)
        0, 0, // Reserved
        0, 0, // Reserved
        54, 0, 0, 0, // Data offset (14 + 40 bytes)
    ];
    
    // DIB Header (BITMAPINFOHEADER - 40 bytes)
    let dib_header: Vec<u8> = vec![
        40, 0, 0, 0, // Header size
        (width & 0xFF) as u8, ((width >> 8) & 0xFF) as u8, ((width >> 16) & 0xFF) as u8, ((width >> 24) & 0xFF) as u8, // Width
        (height & 0xFF) as u8, ((height >> 8) & 0xFF) as u8, ((height >> 16) & 0xFF) as u8, ((height >> 24) & 0xFF) as u8, // Height
        1, 0, // Color planes
        24, 0, // Bits per pixel
        0, 0, 0, 0, // Compression (none)
        0, 0, 0, 0, // Image size (can be 0 for uncompressed)
        0, 0, 0, 0, // X pixels per meter
        0, 0, 0, 0, // Y pixels per meter
        0, 0, 0, 0, // Colors in color table
        0, 0, 0, 0, // Important colors
    ];
    
    let file_size = 14 + 40 + flipped_pixels.len();
    
    let mut file = File::create("default.bmp").unwrap();
    file.write_all(&file_header).unwrap();
    
    // Write file size
    file.write_all(&[
        (file_size & 0xFF) as u8,
        ((file_size >> 8) & 0xFF) as u8,
        ((file_size >> 16) & 0xFF) as u8,
        ((file_size >> 24) & 0xFF) as u8,
    ]).unwrap();
    
    // Write reserved bytes
    file.write_all(&[0, 0, 0, 0]).unwrap();
    
    // Write data offset
    file.write_all(&[54, 0, 0, 0]).unwrap();
    
    file.write_all(&dib_header).unwrap();
    file.write_all(&flipped_pixels).unwrap();
    
    println!("Generated default.bmp ({}x{})", width, height);
}
