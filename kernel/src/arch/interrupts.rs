//! Interrupt handling

use crate::println;

/// IDT Entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn new() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    fn set_handler(&mut self, handler: u64) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
        self.selector = super::gdt::KERNEL_CODE_SELECTOR;
        self.type_attr = 0x8E; // Present, Ring 0, Interrupt Gate
    }
}

/// Number of IDT entries
pub const IDT_ENTRIES: usize = 256;

/// IDT (256 entries)
static mut IDT: [IdtEntry; 256] = [IdtEntry::new(); 256];

/// IDT pointer for LIDT instruction
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

/// Interrupt stack frame
#[repr(C)]
#[derive(Debug)]
pub struct InterruptStackFrame {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

/// Initialize interrupt handling
pub fn init() {
    unsafe {
        // Set up exception handlers
        IDT[0].set_handler(divide_error as u64);
        IDT[1].set_handler(debug as u64);
        IDT[2].set_handler(nmi as u64);
        IDT[3].set_handler(breakpoint as u64);
        IDT[4].set_handler(overflow as u64);
        IDT[5].set_handler(bound_range_exceeded as u64);
        IDT[6].set_handler(invalid_opcode as u64);
        IDT[7].set_handler(device_not_available as u64);
        IDT[8].set_handler(double_fault as u64);
        IDT[10].set_handler(invalid_tss as u64);
        IDT[11].set_handler(segment_not_present as u64);
        IDT[12].set_handler(stack_segment_fault as u64);
        IDT[13].set_handler(general_protection_fault as u64);
        IDT[14].set_handler(page_fault as u64);
        IDT[16].set_handler(x87_floating_point as u64);
        IDT[17].set_handler(alignment_check as u64);
        IDT[18].set_handler(machine_check as u64);
        IDT[19].set_handler(simd_floating_point as u64);
        IDT[20].set_handler(virtualization as u64);
        IDT[30].set_handler(security_exception as u64);
        
        // Load IDT
        let idt_ptr = IdtPointer {
            limit: ((256 * core::mem::size_of::<IdtEntry>()) - 1) as u16,
            base: IDT.as_ptr() as u64,
        };
        
        core::arch::asm!(
            "lidt [{}]",
            in(reg) &idt_ptr,
            options(nostack)
        );
    }
    
    // Enable interrupts
    super::cpu::enable_interrupts();
}

/// Disable interrupts
pub fn disable() {
    super::cpu::disable_interrupts();
}

/// Enable interrupts
pub fn enable() {
    super::cpu::enable_interrupts();
}

/// Check if interrupts are enabled
pub fn are_enabled() -> bool {
    super::cpu::interrupts_enabled()
}

// Exception handlers

extern "x86-interrupt" fn divide_error(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Divide Error\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn debug(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Debug\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn nmi(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Non-Maskable Interrupt\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Breakpoint at {:#x}", stack_frame.instruction_pointer);
}

extern "x86-interrupt" fn overflow(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Overflow\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn bound_range_exceeded(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Bound Range Exceeded\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Invalid Opcode at {:#x}\n{:#?}", 
        stack_frame.instruction_pointer, stack_frame);
}

extern "x86-interrupt" fn device_not_available(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Device Not Available\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT (error code: {})\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn invalid_tss(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: Invalid TSS (error code: {})\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn segment_not_present(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: Segment Not Present (error code: {})\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn stack_segment_fault(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: Stack Segment Fault (error code: {})\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn general_protection_fault(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: General Protection Fault (error code: {})\n{:#?}", 
        error_code, stack_frame);
}

extern "x86-interrupt" fn page_fault(stack_frame: InterruptStackFrame, error_code: u64) {
    // Read CR2 for faulting address
    let cr2: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack));
    }
    
    panic!(
        "EXCEPTION: Page Fault\n  Accessed Address: {:#x}\n  Error Code: {:#b}\n  {:#?}",
        cr2, error_code, stack_frame
    );
}

extern "x86-interrupt" fn x87_floating_point(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: x87 Floating Point\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn alignment_check(stack_frame: InterruptStackFrame, _error_code: u64) {
    panic!("EXCEPTION: Alignment Check\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn machine_check(stack_frame: InterruptStackFrame) -> ! {
    panic!("EXCEPTION: Machine Check\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn simd_floating_point(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: SIMD Floating Point\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn virtualization(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: Virtualization\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn security_exception(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: Security Exception (error code: {})\n{:#?}", error_code, stack_frame);
}
