#![no_std]
#![no_main]

#[macro_use]
mod drivers;
mod arch;
mod panic;
pub mod test_framework;

use limine::request::{
    BootloaderInfoRequest, ExecutableAddressRequest, FramebufferRequest, HhdmRequest,
    MemoryMapRequest, MpRequest, RequestsEndMarker, RequestsStartMarker, RsdpRequest,
    StackSizeRequest,
};
use limine::BaseRevision;

// Limine protocol: BaseRevision must be present for the bootloader to
// recognize our kernel. It lives outside the requests section.
#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

// Request section markers and all requests must be in a contiguous section.
// The start marker must come first and end marker last.
#[used]
#[link_section = ".requests_start_marker"]
static _REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[link_section = ".requests"]
static _STACK_SIZE: StackSizeRequest = StackSizeRequest::new().with_size(0x10_0000);

#[used]
#[link_section = ".requests"]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
#[link_section = ".requests"]
static MEMORY_MAP: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[link_section = ".requests"]
static HHDM: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
static KERNEL_ADDRESS: ExecutableAddressRequest = ExecutableAddressRequest::new();

#[used]
#[link_section = ".requests"]
static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".requests"]
static RSDP: RsdpRequest = RsdpRequest::new();

#[used]
#[link_section = ".requests"]
static SMP: MpRequest = MpRequest::new();

#[used]
#[link_section = ".requests_end_marker"]
static _REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

fn kernel_main() -> ! {
    // Phase 1: Early initialization
    // Serial port is the first thing we initialize — all debug output depends on it
    drivers::serial::init();

    serial_println!("LogOS v0.1.0 booting...");

    // Verify the bootloader supports our required revision
    if !BASE_REVISION.is_supported() {
        serial_println!("FATAL: Limine base revision not supported");
        halt_loop();
    }

    serial_println!("Bootloader revision: OK");

    // Validate CPU features
    let features = arch::x86_64::cpu::CpuFeatures::detect();
    if let Some(missing) = features.validate() {
        panic!("CPU lacks required feature: {}", missing);
    }

    serial_println!(
        "CPU: vendor={} family={} model={} stepping={}",
        core::str::from_utf8(&features.vendor).unwrap_or("unknown"),
        features.family,
        features.model,
        features.stepping,
    );
    serial_println!("CPU features: OK");
    serial_println!(
        "  XSAVE: {} (area size: {} bytes)",
        features.has_xsave,
        features.xsave_area_size
    );
    serial_println!("  RDRAND: {}", features.has_rdrand);
    serial_println!("  1 GiB pages: {}", features.has_1gib_pages);

    // Log memory map
    if let Some(response) = MEMORY_MAP.get_response() {
        let entries = response.entries();
        let mut total_usable: u64 = 0;
        for entry in entries {
            if entry.entry_type == limine::memory_map::EntryType::USABLE {
                total_usable += entry.length;
            }
        }
        serial_println!(
            "Memory: {} MiB usable ({} entries in map)",
            total_usable / (1024 * 1024),
            entries.len(),
        );
    } else {
        serial_println!("WARNING: No memory map from bootloader");
    }

    if let Some(response) = HHDM.get_response() {
        serial_println!("HHDM offset: {:#x}", response.offset());
    } else {
        serial_println!("WARNING: HHDM not provided by bootloader");
    }

    if let Some(response) = KERNEL_ADDRESS.get_response() {
        serial_println!(
            "Kernel: phys={:#x} virt={:#x}",
            response.physical_base(),
            response.virtual_base()
        );
    }

    serial_println!("Boot complete. Halting.");

    halt_loop()
}

/// Enter an infinite halt loop.
pub fn halt_loop() -> ! {
    loop {
        arch::x86_64::cpu::hlt();
    }
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    panic::panic_handler(info)
}

#[no_mangle]
extern "C" fn _start() -> ! {
    kernel_main()
}
