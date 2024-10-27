mod exceptions;

use core::fmt::Write;

use ignore_result::Ignore;

use super::serial::{ComPort::COM1, SerialPort};
use crate::arch::x86_64::idt::*;

use crate::arch::*;

pub fn load_exceptions(idt: &mut Idt) {
    idt.set_gate(0, isr_divide_by_zero, 1 << 3, true, true);
    idt.set_gate(1, isr_debug, 1 << 3, true, false);
    idt.set_gate(2, isr_non_maskable_interrupt, 1 << 3, true, false);
    idt.set_gate(3, isr_breakpoint, 1 << 3, true, false);
    idt.set_gate(4, isr_overflow, 1 << 3, true, false);
    idt.set_gate(5, isr_bound_range_exceeded, 1 << 3, true, false);
    idt.set_gate(6, isr_invalid_opcode, 1 << 3, true, false);
    idt.set_gate(7, isr_device_not_available, 1 << 3, true, false);
    idt.set_gate(8, isr_double_fault, 1 << 3, true, true);
    idt.set_gate(10, isr_invalid_tss, 1 << 3, true, false);
    idt.set_gate(11, isr_segment_not_present, 1 << 3, true, true);
    idt.set_gate(12, isr_stack_segment_fault, 1 << 3, true, false);
    idt.set_gate(13, isr_general_protection_fault, 1 << 3, true, true);
    idt.set_gate(14, isr_page_fault, 1 << 3, true, true);
    idt.set_gate(15, isr_reserved, 1 << 3, true, false);
    idt.set_gate(16, isr_x87_floating_point, 1 << 3, true, false);
    idt.set_gate(17, isr_alignment_check, 1 << 3, true, false);
    idt.set_gate(18, isr_machine_check, 1 << 3, true, false);
    idt.set_gate(19, isr_simd_floating_point, 1 << 3, true, false);
    idt.set_gate(20, isr_virtualization, 1 << 3, true, false);
    idt.set_gate(21, isr_control_protection, 1 << 3, true, false);
    idt.set_gate(28, isr_hypervisor_injection, 1 << 3, true, false);
    idt.set_gate(29, isr_vmm_communication, 1 << 3, true, false);
    idt.set_gate(30, isr_security_exception, 1 << 3, true, false);
}

extern "C" {
    fn isr_divide_by_zero();
    fn isr_debug();
    fn isr_non_maskable_interrupt();
    fn isr_breakpoint();
    fn isr_overflow();
    fn isr_bound_range_exceeded();
    fn isr_invalid_opcode();
    fn isr_device_not_available();
    fn isr_double_fault();
    fn isr_invalid_tss();
    fn isr_stack_segment_fault();
    fn isr_general_protection_fault();
    fn isr_segment_not_present();
    fn isr_page_fault();
    fn isr_reserved();
    fn isr_x87_floating_point();
    fn isr_alignment_check();
    fn isr_machine_check();
    fn isr_simd_floating_point();
    fn isr_virtualization();
    fn isr_control_protection();
    fn isr_hypervisor_injection();
    fn isr_vmm_communication();
    fn isr_security_exception();
}

#[no_mangle]
extern "C" fn ih_double_fault(_error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "A double fault has occurred! Panicking!").ignore();
    ArchApi::panic();
}

#[no_mangle]
extern "C" fn ih_divide_by_zero() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "A divide by zero has occurred in kernelspace! Panicking!"
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_general_protection_fault(error_code: u64, rip: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();
    let rip_adjusted = 0xffffffff80000000 - rip;
    if error_code != 0 {
        writeln!(
            &mut logger,
            "A general protection fault has occurred in kernel space with error code {:X}! Panicking!
            this is usually the segment selector that caused the fault. RIP = {:X}",
            error_code, rip_adjusted
        )
            .ignore();
    } else {
        writeln!(
            &mut logger,
            "A general protection fault has occurred in kernelspace! Panicking!
            RIP = {:X}",
            rip_adjusted,
        )
        .ignore();
    }
}

#[no_mangle]
extern "C" fn ih_page_fault(error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "A page fault has occurred with error code {:32b}",
        error_code
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_segment_not_present(error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "Segment Not Present Fault: error code {:x}",
        error_code
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_debug() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Debug Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_non_maskable_interrupt() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Non-maskable Interrupt Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_breakpoint() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Breakpoint Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_overflow() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Overflow Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_bound_range_exceeded() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Bound Range Exceeded Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_invalid_opcode() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Invalid Opcode Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_device_not_available() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Device Not Available Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_invalid_tss(error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "Invalid TSS Exception Occurred! Error code: {:x}",
        error_code
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_stack_segment_fault(error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "Stack-Segment Fault Occurred! Error code: {:x}",
        error_code
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_reserved() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "Unexpected Reserved Vector 15 Exception Occurred!"
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_x87_floating_point() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "x87 Floating-Point Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_alignment_check(error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "Alignment Check Exception Occurred! Error code: {:x}",
        error_code
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_machine_check() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "Machine Check Exception Occurred! System is halted for safety."
    )
    .ignore();
    // After logging, the system might be halted by the assembly handler (ih_machine_check does not return).
}

#[no_mangle]
extern "C" fn ih_simd_floating_point() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "SIMD Floating-Point Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_virtualization() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Virtualization Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_control_protection(error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "Control Protection Exception Occurred! Error code: {:x}",
        error_code
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_hypervisor_injection() {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(&mut logger, "Hypervisor Injection Exception Occurred!").ignore();
}

#[no_mangle]
extern "C" fn ih_vmm_communication(error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "VMM Communication Exception Occurred! Error code: {:x}",
        error_code
    )
    .ignore();
}

#[no_mangle]
extern "C" fn ih_security_exception(error_code: u64) {
    let mut logger = SerialPort::try_new(COM1).unwrap();

    writeln!(
        &mut logger,
        "Security Exception Occurred! Error code: {:x}",
        error_code
    )
    .ignore();
}
