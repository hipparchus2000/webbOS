//! Global Descriptor Table (GDT) setup

use core::mem::size_of;

/// GDT Entry
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    const fn new() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }

    fn set(&mut self, base: u32, limit: u32, access: u8, granularity: u8) {
        self.limit_low = (limit & 0xFFFF) as u16;
        self.base_low = (base & 0xFFFF) as u16;
        self.base_middle = ((base >> 16) & 0xFF) as u8;
        self.access = access;
        self.granularity = ((limit >> 16) & 0x0F) as u8 | (granularity & 0xF0);
        self.base_high = ((base >> 24) & 0xFF) as u8;
    }
}

/// TSS Entry (64-bit)
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
struct TssEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
    base_upper: u32,
    reserved: u32,
}

impl TssEntry {
    const fn new() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
            base_upper: 0,
            reserved: 0,
        }
    }

    fn set(&mut self, base: u64, limit: u32) {
        self.limit_low = (limit & 0xFFFF) as u16;
        self.base_low = (base & 0xFFFF) as u16;
        self.base_middle = ((base >> 16) & 0xFF) as u8;
        self.access = 0x89; // Present, TSS, Accessible
        self.granularity = ((limit >> 16) & 0x0F) as u8;
        self.base_high = ((base >> 24) & 0xFF) as u8;
        self.base_upper = (base >> 32) as u32;
        self.reserved = 0;
    }
}

/// Task State Segment (64-bit)
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
struct Tss {
    _reserved1: u32,
    rsp0_low: u32,
    rsp0_high: u32,
    rsp1_low: u32,
    rsp1_high: u32,
    rsp2_low: u32,
    rsp2_high: u32,
    _reserved2: u64,
    ist1_low: u32,
    ist1_high: u32,
    ist2_low: u32,
    ist2_high: u32,
    ist3_low: u32,
    ist3_high: u32,
    ist4_low: u32,
    ist4_high: u32,
    ist5_low: u32,
    ist5_high: u32,
    ist6_low: u32,
    ist6_high: u32,
    ist7_low: u32,
    ist7_high: u32,
    _reserved3: u64,
    _reserved4: u16,
    io_map_base: u16,
}

impl Tss {
    const fn new() -> Self {
        Self {
            _reserved1: 0,
            rsp0_low: 0,
            rsp0_high: 0,
            rsp1_low: 0,
            rsp1_high: 0,
            rsp2_low: 0,
            rsp2_high: 0,
            _reserved2: 0,
            ist1_low: 0,
            ist1_high: 0,
            ist2_low: 0,
            ist2_high: 0,
            ist3_low: 0,
            ist3_high: 0,
            ist4_low: 0,
            ist4_high: 0,
            ist5_low: 0,
            ist5_high: 0,
            ist6_low: 0,
            ist6_high: 0,
            ist7_low: 0,
            ist7_high: 0,
            _reserved3: 0,
            _reserved4: 0,
            io_map_base: size_of::<Tss>() as u16,
        }
    }

    fn set_rsp0(&mut self, rsp: u64) {
        self.rsp0_low = rsp as u32;
        self.rsp0_high = (rsp >> 32) as u32;
    }
}

/// GDT with 6 entries (null, kernel code, kernel data, user code32, user data, user code64)
static mut GDT: [GdtEntry; 6] = [GdtEntry::new(); 6];
/// Number of GDT entries
const GDT_ENTRIES: usize = 6;
static mut TSS: Tss = Tss::new();
static mut TSS_ENTRY: TssEntry = TssEntry::new();

/// GDT pointer for LGDT instruction
#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

/// Kernel code segment selector
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
/// Kernel data segment selector
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
/// User code segment selector (32-bit)
pub const USER_CODE32_SELECTOR: u16 = 0x18;
/// User data segment selector
pub const USER_DATA_SELECTOR: u16 = 0x20;
/// User code segment selector (64-bit)
pub const USER_CODE64_SELECTOR: u16 = 0x28;
/// TSS segment selector
pub const TSS_SELECTOR: u16 = 0x30;

/// Initialize GDT
pub fn init() {
    unsafe {
        // Null descriptor (index 0)
        GDT[0].set(0, 0, 0, 0);
        
        // Kernel code segment (index 1)
        // Base: 0, Limit: 4GB, Access: Present, Ring 0, Code, Execute/Read
        GDT[1].set(0, 0xFFFFFFFF, 0x9A, 0xAF);
        
        // Kernel data segment (index 2)
        // Base: 0, Limit: 4GB, Access: Present, Ring 0, Data, Read/Write
        GDT[2].set(0, 0xFFFFFFFF, 0x92, 0xCF);
        
        // User code segment 32-bit (index 3)
        GDT[3].set(0, 0xFFFFFFFF, 0xFA, 0xCF);
        
        // User data segment (index 4)
        GDT[4].set(0, 0xFFFFFFFF, 0xF2, 0xCF);
        
        // User code segment 64-bit (index 5)
        GDT[5].set(0, 0xFFFFFFFF, 0xFA, 0xAF);
        
        // Set up TSS entry
        let tss_addr = &TSS as *const _ as u64;
        TSS_ENTRY.set(tss_addr, size_of::<Tss>() as u32 - 1);
        
        // Load GDT
        let gdt_ptr = GdtPointer {
            limit: ((GDT_ENTRIES * size_of::<GdtEntry>()) - 1) as u16,
            base: GDT.as_ptr() as u64,
        };
        
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &gdt_ptr,
            options(nostack)
        );
        
        // Reload segment registers
        core::arch::asm!(
            "mov ax, {0:x}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            "push {1:r}",
            "lea rax, [2f]",
            "push rax",
            "retfq",
            "2:",
            in(reg) KERNEL_DATA_SELECTOR,
            in(reg) KERNEL_CODE_SELECTOR,
            options(nostack)
        );
        
        // Load TSS
        core::arch::asm!(
            "ltr {0:x}",
            in(reg) TSS_SELECTOR,
            options(nostack)
        );
    }
}

/// Set kernel stack in TSS
pub fn set_kernel_stack(stack_top: u64) {
    unsafe {
        TSS.set_rsp0(stack_top);
    }
}
