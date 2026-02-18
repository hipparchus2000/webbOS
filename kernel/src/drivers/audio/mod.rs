//! Audio subsystem
//!
//! Provides audio playback support through various audio devices.
//! Currently supports Intel HD Audio (HDA) controllers.

pub mod hda;

use crate::println;
use alloc::boxed::Box;

use lazy_static::lazy_static;
use spin::Mutex;

/// Audio format specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    /// Sample rate in Hz (e.g., 44100, 48000)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u8,
    /// Bits per sample (8, 16, 24, or 32)
    pub bits_per_sample: u8,
}

impl AudioFormat {
    /// Create a standard CD-quality format (44.1kHz, stereo, 16-bit)
    pub const fn cd_quality() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
        }
    }
    
    /// Create a standard 48kHz format (48kHz, stereo, 16-bit)
    pub const fn standard_48k() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            bits_per_sample: 16,
        }
    }
    
    /// Calculate bytes per sample
    pub fn bytes_per_sample(&self) -> usize {
        (self.bits_per_sample as usize / 8) * self.channels as usize
    }
    
    /// Calculate bytes per second (byte rate)
    pub fn bytes_per_second(&self) -> usize {
        self.sample_rate as usize * self.bytes_per_sample()
    }
    
    /// Validate the format
    pub fn is_valid(&self) -> bool {
        match self.bits_per_sample {
            8 | 16 | 24 | 32 => self.channels >= 1 && self.channels <= 8,
            _ => false,
        }
    }
}

/// Audio device trait
/// 
/// Implement this trait for audio output devices.
pub trait AudioDevice: Send + Sync {
    /// Get device name
    fn name(&self) -> &str;
    
    /// Play audio buffer
    /// 
    /// # Arguments
    /// * `buffer` - Audio sample data
    /// * `format` - Audio format specification
    /// 
    /// # Returns
    /// * `Ok(())` - Playback started successfully
    /// * `Err(AudioError)` - Error starting playback
    fn play(&mut self, buffer: &[u8], format: &AudioFormat) -> Result<(), AudioError>;
    
    /// Stop playback
    fn stop(&mut self);
    
    /// Set volume (0-100)
    fn set_volume(&mut self, volume: u8);
    
    /// Check if device is currently playing
    fn is_playing(&self) -> bool;
    
    /// Get current volume
    fn volume(&self) -> u8;
}

/// Audio error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioError {
    /// Device not found
    DeviceNotFound,
    /// Invalid format
    InvalidFormat,
    /// Buffer too large
    BufferTooLarge,
    /// Hardware error
    HardwareError,
    /// DMA allocation failed
    DmaAllocationFailed,
    /// Device busy
    DeviceBusy,
    /// Unsupported operation
    Unsupported,
    /// Timeout
    Timeout,
}

impl core::fmt::Display for AudioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AudioError::DeviceNotFound => write!(f, "Audio device not found"),
            AudioError::InvalidFormat => write!(f, "Invalid audio format"),
            AudioError::BufferTooLarge => write!(f, "Audio buffer too large"),
            AudioError::HardwareError => write!(f, "Hardware error"),
            AudioError::DmaAllocationFailed => write!(f, "DMA buffer allocation failed"),
            AudioError::DeviceBusy => write!(f, "Device busy"),
            AudioError::Unsupported => write!(f, "Unsupported operation"),
            AudioError::Timeout => write!(f, "Operation timed out"),
        }
    }
}

lazy_static! {
    static ref AUDIO_DEVICE: Mutex<Option<Box<dyn AudioDevice>>> = Mutex::new(None);
}

/// Initialize the audio subsystem
pub fn init() {
    println!("[audio] Initializing audio subsystem...");
    
    // Try to initialize Intel HD Audio
    match hda::HdaController::init() {
        Ok(controller) => {
            println!("[audio] Intel HD Audio initialized successfully");
            *AUDIO_DEVICE.lock() = Some(Box::new(controller));
            
            // Play startup sound
            play_startup_sound();
        }
        Err(e) => {
            println!("[audio] Failed to initialize HDA: {}", e);
            println!("[audio] No audio device available");
        }
    }
}

/// Get the global audio device
pub fn get_device() -> Option<spin::MutexGuard<'static, Option<Box<dyn AudioDevice>>>> {
    Some(AUDIO_DEVICE.lock())
}

/// Play audio buffer with the default device
pub fn play(buffer: &[u8], format: &AudioFormat) -> Result<(), AudioError> {
    let mut device = AUDIO_DEVICE.lock();
    
    if let Some(dev) = device.as_mut() {
        dev.play(buffer, format)
    } else {
        Err(AudioError::DeviceNotFound)
    }
}

/// Stop playback on the default device
pub fn stop() {
    let mut device = AUDIO_DEVICE.lock();
    
    if let Some(dev) = device.as_mut() {
        dev.stop();
    }
}

/// Set volume on the default device
pub fn set_volume(volume: u8) {
    let mut device = AUDIO_DEVICE.lock();
    
    if let Some(dev) = device.as_mut() {
        dev.set_volume(volume);
    }
}

/// Get current volume from the default device
pub fn get_volume() -> u8 {
    let device = AUDIO_DEVICE.lock();
    
    if let Some(dev) = device.as_ref() {
        dev.volume()
    } else {
        0
    }
}

/// Check if audio is playing
pub fn is_playing() -> bool {
    let device = AUDIO_DEVICE.lock();
    
    if let Some(dev) = device.as_ref() {
        dev.is_playing()
    } else {
        false
    }
}

/// Generate a square wave beep
/// 
/// # Arguments
/// * `frequency` - Frequency in Hz
/// * `duration_ms` - Duration in milliseconds
/// * `sample_rate` - Sample rate in Hz
pub fn generate_beep(frequency: u32, duration_ms: u32, sample_rate: u32) -> alloc::vec::Vec<u8> {
    let samples = (sample_rate as u32 * duration_ms / 1000) as usize;
    let mut buffer = alloc::vec::Vec::with_capacity(samples * 2); // 16-bit mono
    
    let period_samples = sample_rate / frequency;
    
    for i in 0..samples {
        let value = if (i as u32 / period_samples) % 2 == 0 {
            0x4000i16 // Positive half
        } else {
            -0x4000i16 // Negative half
        };
        
        buffer.push((value & 0xFF) as u8);
        buffer.push(((value >> 8) & 0xFF) as u8);
    }
    
    buffer
}

/// Generate a simple startup sound (three-tone beep sequence)
pub fn generate_startup_sound() -> alloc::vec::Vec<u8> {
    let sample_rate = 44100u32;
    let mut result = alloc::vec::Vec::new();
    
    // Three tones: 440Hz (A4), 554Hz (C#5), 659Hz (E5)
    result.extend_from_slice(&generate_beep(440, 150, sample_rate));
    result.extend_from_slice(&generate_beep(554, 150, sample_rate));
    result.extend_from_slice(&generate_beep(659, 300, sample_rate));
    
    // Convert mono to stereo by duplicating samples
    let mut stereo = alloc::vec::Vec::with_capacity(result.len() * 2);
    for i in (0..result.len()).step_by(2) {
        let sample = &result[i..i+2];
        stereo.push(sample[0]);
        stereo.push(sample[1]);
        stereo.push(sample[0]); // Duplicate for right channel
        stereo.push(sample[1]);
    }
    
    stereo
}

/// Play the startup sound
fn play_startup_sound() {
    let sound = generate_startup_sound();
    let format = AudioFormat {
        sample_rate: 44100,
        channels: 2,
        bits_per_sample: 16,
    };
    
    // Try to play, but don't fail if it doesn't work
    let _ = play(&sound, &format);
}

/// Generate a test tone (sine wave approximation)
pub fn generate_test_tone(frequency: u32, duration_ms: u32, sample_rate: u32) -> alloc::vec::Vec<u8> {
    let samples = (sample_rate as u32 * duration_ms / 1000) as usize;
    let mut buffer = alloc::vec::Vec::with_capacity(samples * 4); // 16-bit stereo
    
    // Simple square wave for now (can be improved to sine wave)
    let period_samples = sample_rate / frequency;
    
    for i in 0..samples {
        let value = if (i as u32 / period_samples) % 2 == 0 {
            0x2000i16
        } else {
            -0x2000i16
        };
        
        // Left channel
        buffer.push((value & 0xFF) as u8);
        buffer.push(((value >> 8) & 0xFF) as u8);
        // Right channel
        buffer.push((value & 0xFF) as u8);
        buffer.push(((value >> 8) & 0xFF) as u8);
    }
    
    buffer
}

/// Run audio self-tests
pub fn run_tests() {
    println!("[audio] Running audio tests...");
    
    // Test format validation
    let format = AudioFormat::cd_quality();
    assert!(format.is_valid());
    assert_eq!(format.bytes_per_sample(), 4); // 2 bytes * 2 channels
    assert_eq!(format.bytes_per_second(), 44100 * 4);
    
    // Test beep generation
    let beep = generate_beep(1000, 100, 44100);
    assert!(!beep.is_empty());
    
    // Test startup sound generation
    let startup = generate_startup_sound();
    assert!(!startup.is_empty());
    
    println!("[audio] Audio tests passed");
}

/// Print audio subsystem status
pub fn print_status() {
    let device = AUDIO_DEVICE.lock();
    
    println!("Audio Subsystem Status:");
    if let Some(dev) = device.as_ref() {
        println!("  Device: {}", dev.name());
        println!("  Playing: {}", dev.is_playing());
        println!("  Volume: {}%", dev.volume());
    } else {
        println!("  No audio device available");
    }
}
