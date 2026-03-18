use core::mem::size_of;

use crate::arch::x86_64::gdt;

/// Interrupt stack frame pushed by the CPU on exception/interrupt entry.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// Gate types for IDT entries.
#[derive(Clone, Copy)]
#[repr(u8)]
enum GateType {
    Interrupt = 0xE, // Clears IF on entry
    Trap = 0xF,      // Preserves IF
}

/// A single IDT entry (gate descriptor).
#[derive(Clone, Copy)]
#[repr(C)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    _reserved: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    fn set_handler(&mut self, handler: u64, gate_type: GateType, ist: u8) {
        self.offset_low = handler as u16;
        self.offset_mid = (handler >> 16) as u16;
        self.offset_high = (handler >> 32) as u32;
        self.selector = gdt::KERNEL_CS;
        self.ist = ist;
        // Present | DPL 0 | gate type
        self.type_attr = 0x80 | (gate_type as u8);
        self._reserved = 0;
    }
}

/// IDTR pointer structure.
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

/// The IDT: 256 entries.
static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];

/// Cast a handler function pointer to u64 address.
macro_rules! handler_addr {
    ($f:ident) => {
        $f as *const () as u64
    };
}

/// Initialize the IDT with exception and interrupt handlers.
///
/// # Safety
/// Must be called once during single-threaded boot after GDT is loaded.
pub unsafe fn init() {
    // SAFETY: Single-threaded boot, writing to static IDT.
    unsafe {
        // CPU exceptions (vectors 0-31)
        IDT[0].set_handler(handler_addr!(isr_divide_error), GateType::Trap, 0);
        IDT[1].set_handler(handler_addr!(isr_debug), GateType::Trap, 0);
        IDT[2].set_handler(
            handler_addr!(isr_nmi),
            GateType::Interrupt,
            gdt::IST_NMI as u8,
        );
        IDT[3].set_handler(handler_addr!(isr_breakpoint), GateType::Trap, 0);
        IDT[4].set_handler(handler_addr!(isr_overflow), GateType::Trap, 0);
        IDT[5].set_handler(handler_addr!(isr_bound_range), GateType::Trap, 0);
        IDT[6].set_handler(handler_addr!(isr_invalid_opcode), GateType::Trap, 0);
        IDT[7].set_handler(handler_addr!(isr_device_not_available), GateType::Trap, 0);
        IDT[8].set_handler(
            handler_addr!(isr_double_fault),
            GateType::Interrupt,
            gdt::IST_DOUBLE_FAULT as u8,
        );
        // 9: coprocessor segment overrun (legacy, not used)
        IDT[10].set_handler(handler_addr!(isr_invalid_tss), GateType::Trap, 0);
        IDT[11].set_handler(handler_addr!(isr_segment_not_present), GateType::Trap, 0);
        IDT[12].set_handler(handler_addr!(isr_stack_segment), GateType::Trap, 0);
        IDT[13].set_handler(handler_addr!(isr_general_protection), GateType::Trap, 0);
        IDT[14].set_handler(handler_addr!(isr_page_fault), GateType::Interrupt, 0);
        // 15: reserved
        IDT[16].set_handler(handler_addr!(isr_x87_fpe), GateType::Trap, 0);
        IDT[17].set_handler(handler_addr!(isr_alignment_check), GateType::Trap, 0);
        IDT[18].set_handler(
            handler_addr!(isr_machine_check),
            GateType::Interrupt,
            gdt::IST_MCE as u8,
        );
        IDT[19].set_handler(handler_addr!(isr_simd_fpe), GateType::Trap, 0);
        IDT[20].set_handler(handler_addr!(isr_virtualization), GateType::Trap, 0);
    }

    // Load the IDT
    let idt_ptr = IdtPointer {
        limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
        base: core::ptr::addr_of!(IDT) as u64,
    };

    // SAFETY: IDT is properly initialized and the pointer is valid.
    unsafe {
        core::arch::asm!("lidt [{}]", in(reg) &idt_ptr, options(nostack));
    }
}

/// Set a handler for an arbitrary interrupt vector (for APIC timer, etc).
///
/// # Safety
/// Handler must be a valid ISR with correct calling convention.
pub unsafe fn set_handler(vector: u8, handler: u64, ist: u8) {
    // SAFETY: Caller guarantees handler validity.
    unsafe {
        IDT[vector as usize].set_handler(handler, GateType::Interrupt, ist);
    }
}

// Exception handlers — these are called from assembly stubs or directly.
// Each handler receives the interrupt frame and optional error code.

extern "x86-interrupt" fn isr_divide_error(frame: InterruptFrame) {
    panic!(
        "DIVIDE ERROR at {:#x}:{:#x}\n{:?}",
        frame.cs, frame.rip, frame
    );
}

extern "x86-interrupt" fn isr_debug(frame: InterruptFrame) {
    crate::serial_println!("DEBUG exception at {:#x}", frame.rip);
}

extern "x86-interrupt" fn isr_nmi(frame: InterruptFrame) {
    crate::serial_println!("NMI at {:#x}", frame.rip);
}

extern "x86-interrupt" fn isr_breakpoint(frame: InterruptFrame) {
    crate::serial_println!("BREAKPOINT at {:#x}", frame.rip);
}

extern "x86-interrupt" fn isr_overflow(frame: InterruptFrame) {
    panic!("OVERFLOW at {:#x}\n{:?}", frame.rip, frame);
}

extern "x86-interrupt" fn isr_bound_range(frame: InterruptFrame) {
    panic!("BOUND RANGE EXCEEDED at {:#x}\n{:?}", frame.rip, frame);
}

extern "x86-interrupt" fn isr_invalid_opcode(frame: InterruptFrame) {
    panic!("INVALID OPCODE at {:#x}\n{:?}", frame.rip, frame);
}

extern "x86-interrupt" fn isr_device_not_available(frame: InterruptFrame) {
    panic!("DEVICE NOT AVAILABLE at {:#x}\n{:?}", frame.rip, frame);
}

extern "x86-interrupt" fn isr_double_fault(frame: InterruptFrame, error_code: u64) -> ! {
    panic!(
        "DOUBLE FAULT (error={:#x}) at {:#x}\n{:?}",
        error_code, frame.rip, frame
    );
}

extern "x86-interrupt" fn isr_invalid_tss(frame: InterruptFrame, error_code: u64) {
    panic!(
        "INVALID TSS (error={:#x}) at {:#x}\n{:?}",
        error_code, frame.rip, frame
    );
}

extern "x86-interrupt" fn isr_segment_not_present(frame: InterruptFrame, error_code: u64) {
    panic!(
        "SEGMENT NOT PRESENT (error={:#x}) at {:#x}\n{:?}",
        error_code, frame.rip, frame
    );
}

extern "x86-interrupt" fn isr_stack_segment(frame: InterruptFrame, error_code: u64) {
    panic!(
        "STACK SEGMENT FAULT (error={:#x}) at {:#x}\n{:?}",
        error_code, frame.rip, frame
    );
}

extern "x86-interrupt" fn isr_general_protection(frame: InterruptFrame, error_code: u64) {
    panic!(
        "GENERAL PROTECTION FAULT (error={:#x}) at {:#x}\n{:?}",
        error_code, frame.rip, frame
    );
}

extern "x86-interrupt" fn isr_page_fault(frame: InterruptFrame, error_code: u64) {
    let cr2: u64;
    // SAFETY: Reading CR2 is always safe — it holds the faulting address.
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack));
    }
    panic!(
        "PAGE FAULT at {:#x} (error={:#x}, addr={:#x})\n{:?}",
        frame.rip, error_code, cr2, frame
    );
}

extern "x86-interrupt" fn isr_x87_fpe(frame: InterruptFrame) {
    panic!("x87 FPE at {:#x}\n{:?}", frame.rip, frame);
}

extern "x86-interrupt" fn isr_alignment_check(frame: InterruptFrame, error_code: u64) {
    panic!(
        "ALIGNMENT CHECK (error={:#x}) at {:#x}\n{:?}",
        error_code, frame.rip, frame
    );
}

extern "x86-interrupt" fn isr_machine_check(frame: InterruptFrame) -> ! {
    panic!("MACHINE CHECK at {:#x}\n{:?}", frame.rip, frame);
}

extern "x86-interrupt" fn isr_simd_fpe(frame: InterruptFrame) {
    panic!("SIMD FPE at {:#x}\n{:?}", frame.rip, frame);
}

extern "x86-interrupt" fn isr_virtualization(frame: InterruptFrame) {
    panic!("VIRTUALIZATION EXCEPTION at {:#x}\n{:?}", frame.rip, frame);
}
