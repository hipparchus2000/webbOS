# WebbOS Desktop Wallpaper Support

This document describes the desktop wallpaper feature in WebbOS.

## Overview

WebbOS now supports custom desktop wallpapers that can be loaded from the filesystem or set programmatically. The wallpaper system supports BMP and PPM image formats with automatic fallback to a gradient background if no wallpaper is found.

## Features

- **Image Format Support**: BMP (24-bit and 32-bit) and PPM (P6 binary format)
- **Automatic Scaling**: Wallpapers are automatically resized to fit the screen using cover mode (maintains aspect ratio)
- **Fallback Gradient**: If no wallpaper file is found, a blue gradient is used
- **Runtime Changes**: Wallpapers can be changed at runtime without restarting

## File Locations

The system searches for wallpapers in the following order:

1. `/system/wallpapers/default.bmp`
2. `/system/wallpapers/default.ppm`
3. `/System/Wallpaper/default.bmp`
4. `/System/Wallpaper/default.ppm`

## API Usage

### Setting Wallpaper from Image Data

```rust
// From raw bytes (BMP or PPM)
let image_data = include_bytes!("/path/to/wallpaper.bmp");
desktop::ui::set_wallpaper_from_bytes(image_data).ok();
```

### Setting Gradient Wallpaper

```rust
// Set a custom gradient (two colors)
let color1 = 0x1a1a2e; // Dark blue
desktop::ui::set_wallpaper_gradient(color1, 0x16213e);
```

### Reloading from Filesystem

```rust
// Re-scan filesystem for wallpaper
desktop::ui::reload_wallpaper();
```

### Direct DesktopUI Access

```rust
use desktop::ui::DESKTOP_UI;
use desktop::wallpaper::Wallpaper;

let mut desktop = DESKTOP_UI.lock();

// Set from image data
desktop.set_wallpaper_from_bytes(image_data).ok();

// Set gradient
desktop.set_gradient_wallpaper(0x1a1a2e, 0x16213e);

// Clear wallpaper (use solid color)
desktop.clear_wallpaper();
```

## Creating Wallpapers

### Using the Generator Tool

A sample wallpaper generator is provided:

```bash
cd tools
rustc generate_wallpaper.rs -o generate_wallpaper
./generate_wallpaper
mv default.bmp ../fs/system/wallpapers/
```

### Manual Creation

You can use any image editor to create BMP files (24-bit or 32-bit uncompressed) or use ImageMagick:

```bash
# Convert any image to BMP format
convert input.jpg -resize 1280x800^ -gravity center -extent 1280x800 output.bmp

# Create a gradient
convert -size 1280x800 gradient:#1a1a2e-#16213e gradient.bmp

# Create PPM format
convert input.jpg output.ppm
```

## Technical Details

### Wallpaper Structure

```rust
pub struct Wallpaper {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>, // ARGB format
}
```

### Resizing Modes

- `resize()`: Simple nearest-neighbor scaling
- `resize_cover()`: Maintains aspect ratio, crops to fill screen
- `create_gradient()`: Generates a vertical gradient

### Performance Considerations

- Wallpapers are loaded once at startup and cached
- The wallpaper is drawn pixel-by-pixel; for better performance with large images, consider pre-scaling
- Screen resolution is assumed to be 1280x800 by default

## Future Enhancements

- PNG support (requires deflate decompression)
- JPEG support (requires JPEG decoder)
- Multiple wallpaper support (slideshow)
- Live wallpapers (animated backgrounds)
- Per-user wallpaper settings
- Wallpaper positioning modes (center, tile, stretch, fit)
