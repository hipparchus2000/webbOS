//! Common type definitions for WebbOS

/// Physical memory address
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(u64);

/// Virtual memory address
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(u64);

impl PhysAddr {
    /// Create a new physical address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the underlying address value
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Get the address as a pointer
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Get the address as a mutable pointer
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Align address up to page boundary (4KB)
    pub const fn align_up(self) -> Self {
        Self((self.0 + 0xFFF) & !0xFFF)
    }

    /// Align address down to page boundary (4KB)
    pub const fn align_down(self) -> Self {
        Self(self.0 & !0xFFF)
    }
}

impl VirtAddr {
    /// Create a new virtual address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the underlying address value
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Get the address as a pointer
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Get the address as a mutable pointer
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Align address up to page boundary (4KB)
    pub const fn align_up(self) -> Self {
        Self((self.0 + 0xFFF) & !0xFFF)
    }

    /// Align address down to page boundary (4KB)
    pub const fn align_down(self) -> Self {
        Self(self.0 & !0xFFF)
    }

    /// Convert to physical address (identity mapping)
    pub const fn to_phys(self) -> PhysAddr {
        PhysAddr(self.0)
    }
}

/// Size in bytes
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ByteSize(u64);

impl ByteSize {
    pub const fn new(size: u64) -> Self {
        Self(size)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub const fn to_kb(self) -> u64 {
        self.0 / 1024
    }

    pub const fn to_mb(self) -> u64 {
        self.0 / (1024 * 1024)
    }

    pub const fn to_gb(self) -> u64 {
        self.0 / (1024 * 1024 * 1024)
    }
}

/// Memory region type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryRegionType {
    /// Available RAM (can be used freely)
    Available = 1,
    /// Reserved by hardware or firmware
    Reserved = 2,
    /// ACPI reclaimable memory
    AcpiReclaimable = 3,
    /// ACPI NVS memory
    AcpiNvs = 4,
    /// Bad/unusable memory
    Bad = 5,
    /// Kernel code/data
    Kernel = 0x10,
    /// Bootloader code/data
    Bootloader = 0x11,
    /// Page tables
    PageTables = 0x12,
    /// Framebuffer
    Framebuffer = 0x13,
}

/// Memory region descriptor
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryRegion {
    /// Physical base address
    pub base: PhysAddr,
    /// Size in bytes
    pub size: ByteSize,
    /// Type of memory region
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    /// Create a new memory region
    pub const fn new(base: PhysAddr, size: ByteSize, region_type: MemoryRegionType) -> Self {
        Self {
            base,
            size,
            region_type,
        }
    }

    /// Check if this region contains a physical address
    pub fn contains(&self, addr: PhysAddr) -> bool {
        let base = self.base.as_u64();
        let end = base + self.size.as_u64();
        let addr_val = addr.as_u64();
        addr_val >= base && addr_val < end
    }

    /// Get end address (exclusive)
    pub fn end(&self) -> PhysAddr {
        PhysAddr::new(self.base.as_u64() + self.size.as_u64())
    }
}

/// Process ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Pid(u64);

impl Pid {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Thread ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Tid(u64);

impl Tid {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Result type for WebbOS
pub type Result<T> = core::result::Result<T, Error>;

/// Common error types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Error {
    Success = 0,
    InvalidArgument = 1,
    OutOfMemory = 2,
    NotFound = 3,
    AlreadyExists = 4,
    PermissionDenied = 5,
    InvalidOperation = 6,
    NotSupported = 7,
    IoError = 8,
    Timeout = 9,
    Busy = 10,
    BufferTooSmall = 11,
    InvalidPointer = 12,
    Unknown = 0xFFFFFFFF,
}

impl Error {
    pub const fn is_ok(self) -> bool {
        matches!(self, Error::Success)
    }

    pub const fn is_err(self) -> bool {
        !self.is_ok()
    }
}

/// Common constants
pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = 12;

/// Kernel virtual memory base (higher half)
pub const KERNEL_BASE: u64 = 0xFFFF_8000_0000_0000;

/// User space limit
pub const USER_SPACE_LIMIT: u64 = 0x0000_7FFF_FFFF_FFFF;

/// Kernel stack size (per CPU)
pub const KERNEL_STACK_SIZE: usize = 128 * 1024; // 128KB
