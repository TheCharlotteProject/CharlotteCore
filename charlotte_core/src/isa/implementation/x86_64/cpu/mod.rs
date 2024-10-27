use core::arch::asm;
use core::arch::x86_64::{__cpuid, __cpuid_count};

use spin::lazy::Lazy;

use crate::arch::x86_64::cpu::cpu_intrinsics::{asm_read_msr, asm_write_msr};

mod cpu_intrinsics;

/// The number of significant bits in a physical address on the current CPU.
pub static PADDR_SIG_BITS: Lazy<u8> = Lazy::new(|| {
    let cpuid = unsafe { __cpuid_count(0x80000008, 0) };
    // 0x80000008 is the highest cpuid leaf that returns the physical address width in EAX[7:0]
    let psig_bits = cpuid.eax & 0xFF;
    psig_bits as u8
});

/// The number of significant bits in a virtual address on the current CPU.
pub static VADDR_SIG_BITS: Lazy<u8> = Lazy::new(|| {
    let cpuid = unsafe { __cpuid_count(0x80000008, 0) };
    // 0x80000008 is the highest cpuid leaf that returns the virtual address width in EAX[15:8]
    let vsig_bits = (cpuid.eax >> 8) & 0xFF;
    vsig_bits as u8
});

pub static ARE_HUGE_PAGES_SUPPORTED: Lazy<bool> = Lazy::new(huge_pages_supported);
pub static CPU_HAS_MSR: Lazy<bool> = Lazy::new(|| {
    let res = unsafe { __cpuid_count(0, 0) };
    res.edx & 1 << 5 != 0
});

extern "C" {
    pub fn asm_halt() -> !;
    pub fn asm_get_vendor_string(dest: &mut [u8; 12]);
    pub fn asm_get_privilege_level() -> u8;
}

pub struct MSRValue {
    pub eax: u32,
    pub edx: u32,
}

pub fn get_privilege_level() -> u8 {
    unsafe { asm_get_privilege_level() }
}

pub fn read_msr(msr: u32) -> MSRValue {
    if !*CPU_HAS_MSR {
        panic!("Processor lacks msr support and read_msr was called!");
    }
    unsafe { asm_read_msr(msr) }
}

pub fn read_msr_u64(msr: u32) -> u64 {
    if !*CPU_HAS_MSR {
        panic!("Processor lacks msr support and read_msr was called!");
    }
    let regs = unsafe { asm_read_msr(msr) };
    let mut res = 0u64;
    res = (res | (regs.edx as u64) << 32) | regs.eax as u64;
    res
}

pub fn write_msr(msr: u32, value: MSRValue) {
    if !*CPU_HAS_MSR {
        panic!("Processor lacks msr support and write_msr was called!");
    }
    unsafe { asm_write_msr(msr, value) };
}

pub fn set_msr_bit(msr: u32, bit: u8) {
    let mut val = read_msr(msr);
    val.edx |= 1 << bit;
    write_msr(msr, val);
}

pub fn clear_msr_bit(msr: u32, bit: u8) {
    let mut val = read_msr(msr);
    val.edx &= !(1 << bit);
    write_msr(msr, val);
}

/// Test the flags of the processor to determine if the interrupts are enabled
pub fn asm_are_interrupts_enabled() -> bool {
    let mut flags: u64;
    unsafe { asm!("pushf\n\tpop {}", out(reg) flags) };
    (flags & 1 << 9) != 0
}

#[allow(unused)]
pub fn irq_disable() {
    unsafe {
        asm!("cli");
    };
}

#[allow(unused)]
pub fn irq_restore() {
    unsafe {
        asm!("sti");
    };
}

pub fn asm_outb(port: u16, val: u8) {
    unsafe {
        asm!(
        "
            out dx, al
        ",
        in("dx") port,
        in("al") val,
        );
    }
}

pub fn asm_inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        asm!(
        "
            in al, dx
        ",
        out("al") val,
        in("dx") port,
        );
    }
    val
}

/// outputs `word` to `port`
pub fn asm_outw(port: u16, word: u16) {
    unsafe {
        asm!(
        "
            out dx, ax
        ",
        in("dx") port,
        in("ax") word,
        );
    }
}

/// outputs `dword` to `port`
pub fn asm_outdw(port: u16, dword: u32) {
    unsafe {
        asm!(
        "
            out dx, eax
        ",
        in("dx") port,
        in("eax") dword,
        );
    }
}

pub fn asm_inw(port: u16) -> u16 {
    let word: u16;
    unsafe {
        asm!(
        "
            in ax, dx
        ",
        out("ax") word,
        in("dx") port,
        );
    }
    word
}

pub fn asm_indw(port: u16) -> u32 {
    let dword: u32;
    unsafe {
        asm!(
        "
            in eax, dx
        ",
        out("eax") dword,
        in("dx") port,
        );
    }
    dword
}

pub fn get_tsc_frequency() -> u32 {
    let cpuid_res = unsafe { __cpuid(0x15) };

    if cpuid_res.ebx == 0 {
        panic!("TSC clock ratio is not enumerated!");
    }

    cpuid_res.ebx / cpuid_res.eax
}

/// Determines whether the current LP supports huge pages.
/// Returns `true` if huge pages are supported, `false` otherwise.
fn huge_pages_supported() -> bool {
    let cpuid_result = unsafe { __cpuid_count(0x80000001, 0) };
    let edx = cpuid_result.edx;
    edx & (1 << 26) != 0
}
