#![no_std]
#![no_main]
#![warn(missing_copy_implementations)]
<<<<<<< Updated upstream
=======
#![feature(step_trait)]
#![feature(allocator_api)]
>>>>>>> Stashed changes

use core::fmt::Write;
use core::panic::PanicInfo;

use arch::{Api, ArchApi, HwTimerMode};

use crate::kmon::Kmon;

mod acpi;
mod arch;
mod bootinfo;
mod framebuffer;
mod kmon;
mod logger;
mod physical_memory;
#[cfg(target_arch = "x86_64")]
mod segmentation;
mod serial;
mod virtual_memory;

/// This is the kernel entrypoint function,
/// the first thing it does is call: [isa_init](ArchApi::isa_init)
/// you should check the documentation on that function for details,
/// since that contains all the ISA specific initialization code.
#[no_mangle]
unsafe extern "C" fn main() -> ! {
    let mut arch_api = ArchApi::isa_init();
    logln!("Bring up finished, starting kernel interactive prompt");

//This code currently causes a triple fault if allowed to run. A fix is needed!
/*     // Setup handle_timer function to handle interrupt vector 32 for x86_64
    #[cfg(target_arch = "x86_64")]
    arch_api.set_interrupt_handler(on_tick, 32);
    // Start the ISA specific timer(s) with a rate of about every 10us (1MHz)
    arch_api.setup_isa_timer(100_000, HwTimerMode::Recurrent, 0);
    arch_api.start_isa_timers(); */
    let port = arch_api.get_serial();
    let mut mon = Kmon::new(port);
    mon.repl_loop();

    #[allow(clippy::empty_loop)]
    loop {}
}

#[no_mangle]
/// System monotonic time, this is a global variable that is updated every time the timer ticks.
static mut SYSTEM_MONOTONIC: u64 = 0;

/// This function handles timer ticks, inside of it you can dispatch schedulers
/// or anything else that needs to be done on a timer tick.
fn on_tick(_: u64) {
    unsafe {
        SYSTEM_MONOTONIC += 10;
    }
    ArchApi::end_of_interrupt();
}

#[panic_handler]
fn rust_panic(_info: &PanicInfo) -> ! {
    logln!("A kernel panic has occurred due to a Rust runtime panic.");
    logln!("PanicInfo: {:?}", _info);
    ArchApi::panic()
}
