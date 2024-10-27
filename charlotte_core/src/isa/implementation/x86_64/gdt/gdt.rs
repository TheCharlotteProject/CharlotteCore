use core::arch::global_asm;

global_asm! {
    include_str!("gdt.asm")
}
