use core::arch::asm;
use core::{arch::x86_64::__cpuid_count, fmt::Write};
use spin::lazy::Lazy;

use crate::logln;

extern "C" {
    pub fn asm_halt() -> !;
    pub fn asm_inb(port: u16) -> u8;
    pub fn asm_outb(port: u16, val: u8);
    pub fn asm_read_msr(msr: u32, lo: &mut u32, hi: &mut u32);
    pub fn asm_write_msr(msr: u32, eax: u32, edx: u32);
    pub fn asm_get_privilege_level() -> u8;
}

pub struct MSRResponse {
    pub eax: u32,
    pub edx: u32,
}

pub fn get_privilege_level() -> u8 {
    unsafe { asm_get_privilege_level() }
}

pub fn assert_msr_presence() -> bool {
    let cpuidresult = unsafe { __cpuid_count(0, 0) };
    return cpuidresult.edx & 1 << 5 != 0;
}

pub fn read_msr(msr: u32) -> MSRResponse {
    if !assert_msr_presence() {
        panic!("Processor lacks msr support and read_msr was called!");
    }
    let mut lo = 0;
    let mut hi = 0;
    unsafe { asm_read_msr(msr, &mut lo, &mut hi) }

    MSRResponse { eax: lo, edx: hi }
}

pub fn write_msr(msr: u32, eax: u32, edx: u32) {
    if get_privilege_level() != 0 {
        logln!("Privilege level is not 0, is {}", get_privilege_level());
        return;
    }
    if !assert_msr_presence() {
        panic!("Processor lacks msr support and write_msr was called!");
    }
    unsafe { asm_write_msr(msr, eax, edx) };
}

/// Test the flags of the processor to determine if the interrupts are enabled
pub fn asm_are_interrupts_enabled() -> bool {
    let mut flags: u64;
    unsafe { asm!("pushf\n\tpop {}", out(reg) flags) };
    flags & 1 << 9 != 0
}

pub fn asm_sti() {
    unsafe { asm!("sti") };
}

pub fn asm_irq_enable() {
    // Get the status flags of the processor
    let mut flags: u64;
    unsafe { asm!("pushf\n\tpop {}", out(reg) flags) };
    flags |= 1 << 9;
    unsafe { asm!("push {}\n\tpopf", in(reg) flags) };
}

#[allow(unused)]
pub fn asm_irq_disable() -> u64 {
    let mut flags: u64;
    unsafe { asm!("pushf\n\tcli\n\tpop {}", out(reg) flags) };
    flags
}

#[allow(unused)]
pub fn asm_irq_restore(flags: u64) {
    unsafe { asm!("push {}\n\tpopf", in(reg) flags) };
}

pub fn get_vendor_string(dest: &mut [u8; 12]) {
    let cpuid_result = unsafe { __cpuid_count(0, 0) };
    dest[0..4].copy_from_slice(&cpuid_result.ebx.to_le_bytes());
    dest[4..8].copy_from_slice(&cpuid_result.edx.to_le_bytes());
    dest[8..12].copy_from_slice(&cpuid_result.ecx.to_le_bytes());
}

/// Determines whether the current LP supports huge pages.
/// Returns `true` if huge pages are supported, `false` otherwise.
pub fn huge_pages_supported() -> bool {
    let cpuid_result = unsafe { __cpuid_count(0x80000001, 0) };
    let edx = cpuid_result.edx;
    edx & (1 << 26) != 0
}
