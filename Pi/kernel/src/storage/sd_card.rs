//! SD Card Block Device Driver
//!
//! Uses the SDIO driver to provide block device interface for SD cards.

#![allow(dead_code)]

use alloc::string::String;
use alloc::boxed::Box;
use crate::drivers::sdio::SdhciController;
use crate::storage::{BlockDevice, StorageError};
use crate::println;

/// SD Card block device
pub struct SdCard {
    name: String,
    controller: SdhciController,
    block_size: usize,
    block_count: u64,
}

impl SdCard {
    /// Create new SD card device
    pub fn new(pi4: bool) -> Result<Self, StorageError> {
        println!("[sd_card] Creating SD card device (Pi 4: {})", pi4);
        
        let mut controller = unsafe { SdhciController::new(pi4) };
        
        // Initialize the controller
        controller.init().map_err(|_| StorageError::IoError)?;
        
        // TODO: Get actual card capacity
        // For now, assume 32GB card
        let block_size = 512;
        let block_count = 62_500_000; // ~32GB
        
        Ok(Self {
            name: String::from("sd_card"),
            controller,
            block_size,
            block_count,
        })
    }
    
    /// Initialize SD card subsystem
    pub fn init() {
        println!("[sd_card] Initializing SD card subsystem...");
        
        // Detect Pi version and initialize appropriate controller
        let pi4 = false; // TODO: Detect from device tree
        
        match Self::new(pi4) {
            Ok(card) => {
                println!("[sd_card] SD card initialized: {} blocks", card.block_count);
                crate::storage::register_device(Box::new(card));
            }
            Err(e) => {
                println!("[sd_card] Failed to initialize SD card: {:?}", e);
            }
        }
    }
}

impl BlockDevice for SdCard {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn block_size(&self) -> usize {
        self.block_size
    }
    
    fn block_count(&self) -> u64 {
        self.block_count
    }
    
    fn read_blocks(&self, start: u64, count: usize, _buf: &mut [u8]) -> Result<(), StorageError> {
        // Use SDIO to read blocks
        // SDIO works with function 0 for SD card
        let _address = (start * self.block_size as u64) as u32;
        
        // Use CMD53 to read data
        // This is a simplified version - real implementation needs proper SDIO commands
        println!("[sd_card] Reading {} blocks from offset {}", count, start);
        
        // TODO: Implement actual SDIO block read
        // For now, return error
        Err(StorageError::IoError)
    }
    
    fn write_blocks(&self, start: u64, count: usize, _buf: &[u8]) -> Result<(), StorageError> {
        println!("[sd_card] Writing {} blocks to offset {}", count, start);
        
        // TODO: Implement actual SDIO block write
        Err(StorageError::IoError)
    }
    
    fn flush(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

/// Initialize SD card storage
pub fn init() {
    SdCard::init();
}

/// Read blocks from SD card (public API)
pub fn read_blocks(start: u64, count: usize, buf: &mut [u8]) -> Result<(), StorageError> {
    println!("[sd_card] read_blocks called: start={}, count={}", start, count);
    
    // Use the SDHCI controller
    match crate::drivers::sdio::with_controller(|controller| -> Result<(), StorageError> {
        // Set block length to 512 bytes
        controller.set_blocklen(512)
            .map_err(|_| StorageError::IoError)?;
        
        // Read blocks
        if count == 1 {
            // Single block read
            controller.read_single_block(start as u32, buf)
                .map_err(|_| StorageError::IoError)?;
        } else {
            // Multiple block read
            controller.read_multiple_blocks(start as u32, count as u16, buf)
                .map_err(|_| StorageError::IoError)?;
        }
        
        println!("[sd_card] Read {} blocks from offset {}", count, start);
        Ok(())
    }) {
        Some(result) => result,
        None => Err(StorageError::NotFound),
    }
}
