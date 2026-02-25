//! Audio Subsystem
//!
//! Provides audio output capabilities for WebbOS.
//! Currently supports PWM audio on Raspberry Pi 3.5mm jack.

#![allow(dead_code)]

use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

pub mod pwm;

/// Audio sample rate (Hz)
pub const SAMPLE_RATE: u32 = 44100;

/// Audio buffer size (samples)
pub const BUFFER_SIZE: usize = 1024;

/// Maximum volume (0-100)
pub const MAX_VOLUME: u8 = 100;

/// Global volume level (0-100)
static VOLUME: Mutex<u8> = Mutex::new(80);

/// Audio output state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioState {
    /// Audio is stopped
    Stopped,
    /// Audio is playing
    Playing,
    /// Audio is paused
    Paused,
}

/// Audio format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// 8-bit unsigned
    U8,
    /// 16-bit signed little-endian
    S16LE,
    /// 24-bit signed little-endian
    S24LE,
}

/// Audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels (1=mono, 2=stereo)
    pub channels: u8,
    /// Sample format
    pub format: SampleFormat,
}

impl AudioConfig {
    /// Create default configuration (44.1kHz, stereo, 16-bit)
    pub fn default() -> Self {
        Self {
            sample_rate: SAMPLE_RATE,
            channels: 2,
            format: SampleFormat::S16LE,
        }
    }
}

/// Audio buffer for queued playback
pub struct AudioBuffer {
    /// Raw sample data
    data: Vec<u8>,
    /// Current read position
    position: usize,
    /// Audio format
    config: AudioConfig,
}

impl AudioBuffer {
    /// Create new audio buffer
    pub fn new(data: Vec<u8>, config: AudioConfig) -> Self {
        Self {
            data,
            position: 0,
            config,
        }
    }
    
    /// Get next sample (16-bit signed)
    pub fn next_sample(&mut self) -> Option<i16> {
        if self.position + 2 > self.data.len() {
            return None;
        }
        
        let sample = match self.config.format {
            SampleFormat::S16LE => {
                let lo = self.data[self.position] as i16;
                let hi = self.data[self.position + 1] as i16;
                (hi << 8) | (lo & 0xFF)
            }
            SampleFormat::U8 => {
                ((self.data[self.position] as i16) - 128) * 256
            }
            SampleFormat::S24LE => {
                // Skip 24-bit for now, treat as 0
                self.position += 1;
                0
            }
        };
        
        self.position += 2;
        Some(sample)
    }
    
    /// Check if buffer is exhausted
    pub fn is_empty(&self) -> bool {
        self.position >= self.data.len()
    }
    
    /// Reset to beginning
    pub fn reset(&mut self) {
        self.position = 0;
    }
}

/// Audio subsystem
pub struct AudioSubsystem {
    /// Current state
    state: AudioState,
    /// Current configuration
    config: AudioConfig,
    /// Playback buffer
    buffer: Option<AudioBuffer>,
}

lazy_static! {
    static ref AUDIO: Mutex<AudioSubsystem> = Mutex::new(AudioSubsystem::new());
}

impl AudioSubsystem {
    /// Create new audio subsystem
    pub fn new() -> Self {
        Self {
            state: AudioState::Stopped,
            config: AudioConfig::default(),
            buffer: None,
        }
    }
    
    /// Initialize audio hardware
    pub fn init(&mut self) -> Result<(), AudioError> {
        crate::println!("[audio] Initializing audio subsystem...");
        
        // Initialize PWM audio
        pwm::init()?;
        
        crate::println!("[audio] Audio subsystem initialized");
        Ok(())
    }
    
    /// Set volume (0-100)
    pub fn set_volume(&mut self, volume: u8) {
        let vol = volume.min(MAX_VOLUME);
        *VOLUME.lock() = vol;
        pwm::set_volume(vol);
        crate::println!("[audio] Volume set to {}%", vol);
    }
    
    /// Get current volume (0-100)
    pub fn get_volume(&self) -> u8 {
        *VOLUME.lock()
    }
    
    /// Play audio buffer
    pub fn play(&mut self, data: Vec<u8>, config: AudioConfig) -> Result<(), AudioError> {
        self.buffer = Some(AudioBuffer::new(data, config));
        self.state = AudioState::Playing;
        pwm::start()?;
        Ok(())
    }
    
    /// Play a simple beep/tone
    pub fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), AudioError> {
        // Generate square wave samples
        let samples = generate_square_wave(frequency, duration_ms, self.config.sample_rate);
        self.play(samples, self.config.clone())
    }
    
    /// Stop playback
    pub fn stop(&mut self) {
        self.state = AudioState::Stopped;
        pwm::stop();
        self.buffer = None;
    }
    
    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == AudioState::Playing {
            self.state = AudioState::Paused;
            pwm::stop();
        }
    }
    
    /// Resume playback
    pub fn resume(&mut self) -> Result<(), AudioError> {
        if self.state == AudioState::Paused {
            self.state = AudioState::Playing;
            pwm::start()?;
        }
        Ok(())
    }
    
    /// Get current state
    pub fn state(&self) -> AudioState {
        self.state
    }
    
    /// Process audio (call from timer interrupt)
    /// Returns true if more data is needed
    pub fn process(&mut self) -> bool {
        if self.state != AudioState::Playing {
            return false;
        }
        
        if let Some(ref mut buffer) = self.buffer {
            if buffer.is_empty() {
                self.stop();
                return true; // Need more data
            }
            
            // Feed samples to PWM
            while let Some(sample) = buffer.next_sample() {
                // Apply volume
                let vol = *VOLUME.lock() as i32;
                let scaled = (sample as i32 * vol / 100) as i16;
                
                if pwm::write_sample(scaled).is_err() {
                    break;
                }
            }
        }
        
        false
    }
}

/// Audio error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioError {
    /// Hardware not found
    NotFound,
    /// Initialization failed
    InitFailed,
    /// Invalid configuration
    InvalidConfig,
    /// Buffer underrun
    Underrun,
    /// Hardware error
    HardwareError,
}

/// Generate square wave samples
fn generate_square_wave(frequency: u32, duration_ms: u32, sample_rate: u32) -> Vec<u8> {
    let num_samples = (sample_rate as u32 * duration_ms / 1000) as usize;
    let period_samples = sample_rate / frequency;
    let half_period = period_samples / 2;
    
    let mut samples = Vec::with_capacity(num_samples * 2); // 16-bit stereo
    
    for i in 0..num_samples {
        // Square wave: full positive or full negative
        let sample: i16 = if (i as u32 % period_samples) < half_period {
            16000  // Positive half
        } else {
            -16000 // Negative half
        };
        
        // Left channel
        samples.push((sample & 0xFF) as u8);
        samples.push(((sample >> 8) & 0xFF) as u8);
        // Right channel (same)
        samples.push((sample & 0xFF) as u8);
        samples.push(((sample >> 8) & 0xFF) as u8);
    }
    
    samples
}

/// Initialize audio subsystem
pub fn init() -> Result<(), AudioError> {
    AUDIO.lock().init()
}

/// Set volume (0-100)
pub fn set_volume(volume: u8) {
    AUDIO.lock().set_volume(volume);
}

/// Get current volume (0-100)
pub fn get_volume() -> u8 {
    AUDIO.lock().get_volume()
}

/// Play tone (frequency in Hz, duration in ms)
pub fn play_tone(frequency: u32, duration_ms: u32) -> Result<(), AudioError> {
    AUDIO.lock().play_tone(frequency, duration_ms)
}

/// Play beep sound
pub fn beep() -> Result<(), AudioError> {
    play_tone(880, 100) // A5, 100ms
}

/// Stop playback
pub fn stop() {
    AUDIO.lock().stop();
}

/// Process audio (call from timer)
pub fn process() -> bool {
    AUDIO.lock().process()
}

/// Get audio state
pub fn state() -> AudioState {
    AUDIO.lock().state()
}
