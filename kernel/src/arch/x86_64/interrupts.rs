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

/// Small delay for I/O operations
unsafe fn io_delay() {
    for _ in 0..100 {
        core::arch::asm!("nop", options(nomem, nostack));
    }
}

/// Initialize the PIC (Programmable Interrupt Controller)
/// 
/// Remaps the PIC so that IRQs don't conflict with CPU exceptions.
/// Master PIC: IRQ 0-7 -> IDT entries 32-39 (0x20-0x27)
/// Slave PIC: IRQ 8-15 -> IDT entries 40-47 (0x28-0x2F)
unsafe fn init_pic() {
    println!("[pic] Initializing PIC...");
    
    // ICW1: Start initialization, expect ICW4
    println!("[pic] Sending ICW1...");
    outb(0x20, 0x11); // Master
    io_delay();
    outb(0xA0, 0x11); // Slave
    io_delay();
    
    // ICW2: Vector offset
    println!("[pic] Sending ICW2 (vector offsets)...");
    outb(0x21, 0x20); // Master: 0x20 (32)
    io_delay();
    outb(0xA1, 0x28); // Slave: 0x28 (40)
    io_delay();
    
    // ICW3: Tell master about slave at IRQ2
    println!("[pic] Sending ICW3 (cascade)...");
    outb(0x21, 0x04); // Master: Slave at IRQ2 (bit 2)
    io_delay();
    outb(0xA1, 0x02); // Slave: Cascade identity 2
    io_delay();
    
    // ICW4: 8086 mode, normal EOI
    println!("[pic] Sending ICW4 (mode)...");
    outb(0x21, 0x01);
    io_delay();
    outb(0xA1, 0x01);
    io_delay();
    
    // OCW1: Mask all interrupts initially
    // We'll unmask specific ones after handlers are set up
    println!("[pic] Masking all interrupts...");
    outb(0x21, 0xFF); // Master: mask all
    io_delay();
    outb(0xA1, 0xFF); // Slave: mask all
    
    println!("[pic] PIC initialized");
}

/// Output byte to I/O port
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack)
    );
}

/// Initialize interrupt handling
pub fn init() {
    unsafe {
        // Initialize PIC first
        init_pic();
        
        // Set up exception handlers (entries 0-31)
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
        
        // Set up timer interrupt handler (IRQ0 -> IDT entry 32)
        println!("[interrupts] Setting up timer handler at IDT[32]...");
        IDT[32].set_handler(timer_interrupt_handler as u64);

        // Set up keyboard interrupt handler (IRQ1 -> IDT entry 33)
        println!("[interrupts] Setting up keyboard handler at IDT[33]...");
        IDT[33].set_handler(keyboard_interrupt_handler as u64);

        // Set up mouse interrupt handler (IRQ12 -> IDT entry 44)
        println!("[interrupts] Setting up mouse handler at IDT[44]...");
        IDT[44].set_handler(mouse_interrupt_handler as u64);

        println!("[interrupts] IRQ handlers registered (timer, keyboard, mouse)");
        
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
    
    // Note: We don't enable interrupts here because IRQ handlers 
    // haven't been registered yet. Call enable() after setting up handlers.
    println!("[interrupts] IDT loaded, interrupts disabled until handlers are ready");
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

/// Set an IRQ handler (IRQ 0-15 map to IDT entries 32-47)
/// 
/// # Safety
/// The handler must be a valid interrupt handler function
pub unsafe fn set_irq_handler(irq: u8, handler: extern "x86-interrupt" fn(InterruptStackFrame)) {
    let idt_index = 32 + irq as usize;
    if idt_index < IDT_ENTRIES {
        IDT[idt_index].set_handler(handler as u64);
    }
}

/// Unmask a specific IRQ
/// 
/// # Safety
/// Should only be called after the handler is registered
pub unsafe fn unmask_irq(irq: u8) {
    if irq < 8 {
        // Master PIC
        let mask = inb(0x21);
        outb(0x21, mask & !(1 << irq));
    } else {
        // Slave PIC
        let mask = inb(0xA1);
        outb(0xA1, mask & !(1 << (irq - 8)));
    }
}

/// Read from I/O port
unsafe fn inb(port: u16) -> u8 {
    let result: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") result,
        options(nomem, nostack)
    );
    result
}

/// Send End of Interrupt (EOI) to PIC
pub fn send_eoi(irq: u8) {
    unsafe {
        // If IRQ >= 8, send EOI to slave PIC
        if irq >= 8 {
            core::arch::asm!(
                "mov al, 0x20",
                "out 0xA0, al",
                options(nomem, nostack)
            );
        }
        // Send EOI to master PIC
        core::arch::asm!(
            "mov al, 0x20",
            "out 0x20, al",
            options(nomem, nostack)
        );
    }
}

// Timer interrupt handler (IRQ0)
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        // Send EOI FIRST to prevent interrupt loss if handler hangs/panics
        send_eoi(0);
        
        // Increment tick count
        TIMER_TICKS += 1;
    }
}

// Keyboard interrupt handler (IRQ1)
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    #[allow(unused_unsafe)]
    unsafe {
        // Send EOI FIRST to prevent interrupt loss if handler hangs/panics
        send_eoi(1);
        
        // Handle keyboard input
        crate::drivers::input::handle_keyboard_interrupt();
    }
}

// Mouse interrupt handler (IRQ12)
extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    #[allow(unused_unsafe)]
    unsafe {
        // Send EOI FIRST to prevent interrupt loss if handler hangs/panics
        send_eoi(12);
        
        // Handle mouse input
        crate::drivers::input::handle_mouse_interrupt();
    }
}

/// Timer tick counter (accessible from timer module)
static mut TIMER_TICKS: u64 = 0;

/// Get the current timer tick count
pub fn get_timer_ticks() -> u64 {
    unsafe { TIMER_TICKS }
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
