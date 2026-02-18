//! Block device abstraction layer
//!
//! Provides a unified interface for block storage devices including
//! SD cards, virtual block devices for testing, and future storage backends.

use crate::error::VFatError;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

pub mod sdhost;

/// Block device trait - abstracts different storage backends
pub trait BlockDevice: Send + Sync {
    /// Read a single block (512 bytes)
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), VFatError>;
    
    /// Write a single block (512 bytes)
    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), VFatError>;
    
    /// Read multiple contiguous blocks
    fn read_blocks(&mut self, start_block: u64, count: usize, buffer: &mut [u8]) -> Result<(), VFatError>;
    
    /// Write multiple contiguous blocks
    fn write_blocks(&mut self, start_block: u64, count: usize, buffer: &[u8]) -> Result<(), VFatError>;
    
    /// Get device capacity in blocks
    fn capacity(&self) -> u64;
    
    /// Get block size in bytes (typically 512)
    fn block_size(&self) -> usize;
    
    /// Flush pending writes to device
    fn flush(&mut self) -> Result<(), VFatError>;
}

/// Block device implementation wrapping the SD card driver
pub struct SdCardBlockDevice {
    inner: sdhost::SdBlockDevice,
}

impl SdCardBlockDevice {
    /// Create a new SD card block device
    /// 
    /// # Safety
    /// Caller must ensure base_addr is a valid SDHCI register base
    pub unsafe fn new(base_addr: usize) -> Result<Self, VFatError> {
        let mut inner = sdhost::SdBlockDevice::new(base_addr);
        inner.init()?;
        Ok(Self { inner })
    }

    /// Get device capacity in blocks
    pub fn capacity(&self) -> u64 {
        self.inner.capacity()
    }

    /// Get block size in bytes
    pub fn block_size(&self) -> usize {
        self.inner.block_size()
    }
}

impl BlockDevice for SdCardBlockDevice {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), VFatError> {
        self.inner.read_sector(block, buffer)
    }

    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), VFatError> {
        self.inner.write_sector(block, buffer)
    }

    fn read_blocks(&mut self, start_block: u64, count: usize, buffer: &mut [u8]) -> Result<(), VFatError> {
        self.inner.read_sectors(start_block, count, buffer)
    }

    fn write_blocks(&mut self, start_block: u64, count: usize, buffer: &[u8]) -> Result<(), VFatError> {
        self.inner.write_sectors(start_block, count, buffer)
    }

    fn capacity(&self) -> u64 {
        self.inner.capacity()
    }

    fn block_size(&self) -> usize {
        self.inner.block_size()
    }

    fn flush(&mut self) -> Result<(), VFatError> {
        // SD card has internal flush mechanisms
        // For now, we assume writes are synchronous
        Ok(())
    }
}

/// Virtual block device for testing - backed by memory
pub struct VirtualBlockDevice {
    data: Vec<u8>,
    block_size: usize,
    capacity: u64,
}

impl VirtualBlockDevice {
    /// Create a new virtual block device with given capacity
    pub fn new(capacity_blocks: u64) -> Self {
        let block_size = 512;
        let capacity_bytes = capacity_blocks as usize * block_size;
        Self {
            data: vec![0u8; capacity_bytes],
            block_size,
            capacity: capacity_blocks,
        }
    }

    /// Create a virtual block device from existing data
    pub fn from_data(data: Vec<u8>) -> Self {
        let block_size = 512;
        let capacity = (data.len() / block_size) as u64;
        Self {
            data,
            block_size,
            capacity,
        }
    }

    /// Get reference to underlying data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get mutable reference to underlying data
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl BlockDevice for VirtualBlockDevice {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), VFatError> {
        if block >= self.capacity {
            return Err(VFatError::InvalidParameter(
                format!("Block {} out of range (capacity: {})", block, self.capacity)
            ));
        }
        if buffer.len() != self.block_size {
            return Err(VFatError::InvalidParameter(
                format!("Buffer size {} != block size {}", buffer.len(), self.block_size)
            ));
        }
        
        let offset = block as usize * self.block_size;
        buffer.copy_from_slice(&self.data[offset..offset + self.block_size]);
        Ok(())
    }

    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), VFatError> {
        if block >= self.capacity {
            return Err(VFatError::InvalidParameter(
                format!("Block {} out of range (capacity: {})", block, self.capacity)
            ));
        }
        if buffer.len() != self.block_size {
            return Err(VFatError::InvalidParameter(
                format!("Buffer size {} != block size {}", buffer.len(), self.block_size)
            ));
        }
        
        let offset = block as usize * self.block_size;
        self.data[offset..offset + self.block_size].copy_from_slice(buffer);
        Ok(())
    }

    fn read_blocks(&mut self, start_block: u64, count: usize, buffer: &mut [u8]) -> Result<(), VFatError> {
        if start_block + count as u64 > self.capacity {
            return Err(VFatError::InvalidParameter(
                format!("Block range {}-{} out of range", start_block, start_block + count as u64)
            ));
        }
        if buffer.len() != count * self.block_size {
            return Err(VFatError::InvalidParameter(
                "Buffer size mismatch".to_string()
            ));
        }

        for i in 0..count {
            self.read_block(start_block + i as u64, &mut buffer[i * self.block_size..(i + 1) * self.block_size])?;
        }
        Ok(())
    }

    fn write_blocks(&mut self, start_block: u64, count: usize, buffer: &[u8]) -> Result<(), VFatError> {
        if start_block + count as u64 > self.capacity {
            return Err(VFatError::InvalidParameter(
                format!("Block range {}-{} out of range", start_block, start_block + count as u64)
            ));
        }
        if buffer.len() != count * self.block_size {
            return Err(VFatError::InvalidParameter(
                "Buffer size mismatch".to_string()
            ));
        }

        for i in 0..count {
            self.write_block(start_block + i as u64, &buffer[i * self.block_size..(i + 1) * self.block_size])?;
        }
        Ok(())
    }

    fn capacity(&self) -> u64 {
        self.capacity
    }

    fn block_size(&self) -> usize {
        self.block_size
    }

    fn flush(&mut self) -> Result<(), VFatError> {
        // Memory-backed, nothing to flush
        Ok(())
    }
}

/// Block I/O statistics
#[derive(Debug, Default, Clone)]
pub struct BlockStats {
    /// Number of reads performed
    pub reads: u64,
    /// Number of writes performed
    pub writes: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Number of read errors
    pub read_errors: u64,
    /// Number of write errors
    pub write_errors: u64,
}

/// Block device wrapper that tracks I/O statistics
pub struct StatsBlockDevice<B: BlockDevice> {
    inner: B,
    stats: BlockStats,
}

impl<B: BlockDevice> StatsBlockDevice<B> {
    /// Create a new stats-tracking block device
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            stats: BlockStats::default(),
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> &BlockStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = BlockStats::default();
    }

    /// Get mutable reference to inner device
    pub fn inner_mut(&mut self) -> &mut B {
        &mut self.inner
    }
}

impl<B: BlockDevice> BlockDevice for StatsBlockDevice<B> {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), VFatError> {
        match self.inner.read_block(block, buffer) {
            Ok(()) => {
                self.stats.reads += 1;
                self.stats.bytes_read += buffer.len() as u64;
                Ok(())
            }
            Err(e) => {
                self.stats.read_errors += 1;
                Err(e)
            }
        }
    }

    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), VFatError> {
        match self.inner.write_block(block, buffer) {
            Ok(()) => {
                self.stats.writes += 1;
                self.stats.bytes_written += buffer.len() as u64;
                Ok(())
            }
            Err(e) => {
                self.stats.write_errors += 1;
                Err(e)
            }
        }
    }

    fn read_blocks(&mut self, start_block: u64, count: usize, buffer: &mut [u8]) -> Result<(), VFatError> {
        match self.inner.read_blocks(start_block, count, buffer) {
            Ok(()) => {
                self.stats.reads += 1;
                self.stats.bytes_read += buffer.len() as u64;
                Ok(())
            }
            Err(e) => {
                self.stats.read_errors += 1;
                Err(e)
            }
        }
    }

    fn write_blocks(&mut self, start_block: u64, count: usize, buffer: &[u8]) -> Result<(), VFatError> {
        match self.inner.write_blocks(start_block, count, buffer) {
            Ok(()) => {
                self.stats.writes += 1;
                self.stats.bytes_written += buffer.len() as u64;
                Ok(())
            }
            Err(e) => {
                self.stats.write_errors += 1;
                Err(e)
            }
        }
    }

    fn capacity(&self) -> u64 {
        self.inner.capacity()
    }

    fn block_size(&self) -> usize {
        self.inner.block_size()
    }

    fn flush(&mut self) -> Result<(), VFatError> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_block_device() {
        let mut dev = VirtualBlockDevice::new(100);
        assert_eq!(dev.capacity(), 100);
        assert_eq!(dev.block_size(), 512);

        // Write and read back
        let write_data = vec![0xABu8; 512];
        dev.write_block(0, &write_data).unwrap();

        let mut read_data = vec![0u8; 512];
        dev.read_block(0, &mut read_data).unwrap();
        assert_eq!(write_data, read_data);
    }

    #[test]
    fn test_stats_block_device() {
        let inner = VirtualBlockDevice::new(100);
        let mut dev = StatsBlockDevice::new(inner);

        let write_data = vec![0xCDu8; 512];
        dev.write_block(0, &write_data).unwrap();
        dev.write_block(1, &write_data).unwrap();

        let mut read_data = vec![0u8; 512];
        dev.read_block(0, &mut read_data).unwrap();

        let stats = dev.stats();
        assert_eq!(stats.writes, 2);
        assert_eq!(stats.reads, 1);
        assert_eq!(stats.bytes_written, 1024);
        assert_eq!(stats.bytes_read, 512);
    }
}
