use crate::arch::x86_64::cpu::MSRValue;
use core::arch::{asm, global_asm};

pub unsafe fn asm_read_msr(selector: u32) -> MSRValue {
    let mut eax: u32;
    let mut edx: u32;

    asm!(
        "rdmsr",
        in("ecx") selector,
        out("eax") eax,
        out("edx") edx,
    );

    MSRValue { eax, edx }
}

pub unsafe fn asm_write_msr(selector: u32, value: MSRValue) {
    asm!(
        "wrmsr",
        in("ecx") selector,
        in("eax") value.eax,
        in("edx") value.edx,
    );
}

global_asm! {
    include_str!("cpu.asm")
}
