//! # x86_64 Architecture Module
//! This module implements the Arch interface for the x86_64 instruction set architecture (ISA).

use core::convert::From;
use core::fmt::Write;
use core::str;
use core::{
    borrow::{Borrow, BorrowMut},
    ptr::addr_of,
};

use memory::page_map::{get_cr3, PageMap};
use spin::lazy::Lazy;
use spin::mutex::spin::SpinMutex;

use cpu::*;
use gdt::{tss::Tss, Gdt};
use idt::*;
use serial::{ComPort, SerialPort};

use crate::acpi::{parse, AcpiInfo};
use crate::arch::x86_64::interrupts::apic::Apic;
use crate::arch::x86_64::interrupts::isa_handler::register_iv_handler;
use crate::arch::x86_64::memory::page_map::page_table::page_table_entry::PteFlags;
use crate::arch::{HwTimerMode, IsaParams, MemoryMap, PagingParams};
use crate::framebuffer::colors::Color;
use crate::framebuffer::framebuffer::FRAMEBUFFER;
use crate::logln;
use crate::memory::address::{/*PhysicalAddress,*/ VirtualAddress};
use crate::memory::pmm::PHYSICAL_FRAME_ALLOCATOR;

mod cpu;
mod exceptions;
mod gdt;
mod global;
mod idt;
mod interrupts;
pub mod memory;
mod serial;

/// The Api struct is used to provide an implementation of the ArchApi trait for the x86_64 architecture.
pub struct Api {
    acpi_info: AcpiInfo,
    bsp_apic: Apic,
    irq_flags: u64,
}

static BSP_RING0_INT_STACK: [u8; 4096] = [0u8; 4096];
static BSP_TSS: Lazy<Tss> = Lazy::new(|| Tss::new(addr_of!(BSP_RING0_INT_STACK) as u64));
static BSP_GDT: Lazy<Gdt> = Lazy::new(|| Gdt::new(&BSP_TSS));
static BSP_IDT: SpinMutex<Idt> = SpinMutex::new(Idt::new());
pub const ISA_PARAMS: IsaParams = IsaParams {
    paging: PagingParams {
        page_size: 0x1000,
        page_shift: 0xC,
        page_mask: !0xfff,
    },
};

/// Provide the implementation of the Api trait for the Api struct
impl crate::arch::Api for Api {
    type Api = Api;
    /// Define the logger type
    type DebugLogger = SerialPort;
    /// Define the memory map type
    type MemoryMap = memory::page_map::PageMap;
    type Serial = SerialPort;

    fn isa_init() -> Self {
        FRAMEBUFFER.lock().clear_screen(Color::BLACK);

        logln!("Initializing the bootstrap processor");
        Api::init_bsp();
        logln!("============================================================\n");
        logln!("Parsing ACPI information");
        let tbls = parse();
        logln!("============================================================\n");
        let mut api = Api {
            acpi_info: tbls,
            bsp_apic: Apic::new(tbls.madt()),
            irq_flags: 0,
        };
        logln!("============================================================\n");

        logln!("Enable interrupts");
        api.init_interrupts();
        logln!("Bus frequency is: {}MHz", api.bsp_apic.tps / 10000000);
        logln!("============================================================\n");

        logln!("Memory self tests");
        Self::pmm_self_test();
        logln!("============================================================\n");
        Self::vmm_self_test();
        logln!("============================================================\n");
        logln!("All x86_64 sanity checks passed, kernel main has control now");
        logln!("============================================================\n");

        api
    }

    #[inline]
    fn get_memory_map() -> Self::MemoryMap {
        memory::page_map::PageMap::from_cr3(memory::page_map::get_cr3())
            .expect("unable to create a page map structure from the address in the current CR3 value")
    }

    /// Get a new logger instance
    fn get_logger() -> Self::DebugLogger {
        SerialPort::try_new(ComPort::COM1).unwrap()
    }

    fn get_serial(&self) -> Self::Serial {
        SerialPort::try_new(ComPort::COM1).unwrap()
    }

    /// Get the number of significant physical address bits supported by the current CPU
    fn get_paddr_width() -> u8 {
        *PADDR_SIG_BITS
    }
    /// Get the number of significant virtual address bits supported by the current CPU
    fn get_vaddr_width() -> u8 {
        *VADDR_SIG_BITS
    }

    /// Validates a physical address in accordance with the x86_64 architecture
    #[inline]
    fn validate_paddr(raw: usize) -> bool {
        // Non-significant bits must be zero
        let unused_bitmask = !((1 << Self::get_paddr_width()) - 1);
        raw & unused_bitmask == 0
    }

    /// Validates a virtual address in accordance with the x86_64 architecture
    fn validate_vaddr(raw: u64) -> bool {
        // Canonical form check
        match Self::get_vaddr_width() {
            48 => {
                let masked = raw & 0xFFFF800000000000;
                masked == 0 || masked == 0xFFFF800000000000
            }
            57 => {
                let masked = raw & 0xFFFE000000000000;
                masked == 0 || masked == 0xFFFE000000000000
            }
            _ => false,
        }
    }

    /// Halt the calling LP
    fn halt() -> ! {
        unsafe { asm_halt() }
    }

    /// Kernel Panic
    fn panic() -> ! {
        unsafe { asm_halt() }
    }

    /// Read a byte from the specified port
    fn inb(port: u16) -> u8 {
        asm_inb(port)
    }

    /// Write a byte to the specified port
    fn outb(port: u16, val: u8) {
        asm_outb(port, val)
    }

    /// Initialize the bootstrap processor (BSP)
    ///
    ///  Initialize the application processors (APs)
    fn init_ap(&mut self) {
        //! This routine is run by each application processor to initialize itself prior to being handed off to the scheduler.
    }

    fn setup_isa_timer(&mut self, tps: u32, mode: HwTimerMode, _: u16) {
        let mut divisor = 1u8;
        let mut counter = 0u64;
        while divisor < 128 {
            counter = (self.bsp_apic.tps / divisor as u64) / (tps as u64 * 10);
            if counter < u32::MAX as u64 {
                break;
            }
            divisor <<= 1;
        }
        logln!(
            "Setting up ISA timer with divisor: {}, counter: {}",
            divisor,
            counter
        );
        self.bsp_apic
            .setup_timer(mode.into(), counter as u32, divisor.into());
    }

    fn start_isa_timers(&self) {
        self.bsp_apic.start_timer()
    }

    fn pause_isa_timers(&self) {
        todo!()
    }

    fn interrupts_enabled(&self) -> bool {
        asm_are_interrupts_enabled()
    }

    fn disable_interrupts(&mut self) {
        irq_disable();
    }

    fn restore_interrupts(&mut self) {
        irq_restore();
    }

    fn init_interrupts(&mut self) {
        self.bsp_apic.enable(BSP_IDT.lock().borrow_mut());
    }

    fn set_interrupt_handler(&mut self, h: fn(vector: u64), vector: u32) {
        if vector > u8::MAX as u32 {
            panic!("X86_64 can only have from iv 32 to iv 255 set");
        }
        register_iv_handler(h, vector as u8);
    }

    #[inline(always)]
    fn end_of_interrupt() {
        Apic::signal_eoi();
    }
}

impl Api {
    /// Get the number of significant physical address bits supported by the current CPU
    fn get_paddr_width() -> u8 {
        *PADDR_SIG_BITS
    }
    /// Get the number of significant virtual address bits supported by the current CPU
    fn get_vaddr_width() -> u8 {
        *VADDR_SIG_BITS
    }

    fn init_bsp() {
        //! This routine is run by the bootstrap processor to initialize itself prior to bringing up the kernel.
        logln!("Processor information:");
        BSP_GDT.load();
        logln!("Loaded GDT");
        Gdt::reload_segment_regs();
        logln!("Reloaded segment registers");
        Gdt::load_tss();
        logln!("Loaded TSS");

        logln!("Registering exception ISRs in the IDT");
        exceptions::load_exceptions(BSP_IDT.lock().borrow_mut());
        logln!("Exception ISRs registered");

        logln!("Attempting to load IDT");
        BSP_IDT.lock().borrow().load();
        logln!("Loaded IDT");

        let mut vendor_string = [0u8; 12];
        unsafe { asm_get_vendor_string(&mut vendor_string) }
        logln!("CPU Vendor ID: {}", str::from_utf8(&vendor_string).unwrap());
    }

    fn pmm_self_test() {
        logln!(
            "Number of Significant Physical Address Bits Supported: {}",
            Api::get_paddr_width()
        );
        logln!(
            "Number of Significant Virtual Address Bits Supported: {}",
            Api::get_vaddr_width()
        );

        logln!("Testing Physical Memory Manager");
        logln!("Performing single frame allocation and deallocation test.");
        let alloc = PHYSICAL_FRAME_ALLOCATOR.lock().allocate();
        let alloc2 = PHYSICAL_FRAME_ALLOCATOR.lock().allocate();
        match alloc {
            Ok(frame) => {
                logln!("Allocated frame with physical base address: {:?}", frame);
                let _ = PHYSICAL_FRAME_ALLOCATOR.lock().deallocate(frame);
                logln!("Deallocated frame with physical base address: {:?}", frame);
            }
            Err(e) => {
                logln!("Failed to allocate frame: {:?}", e);
            }
        }
        let alloc3 = PHYSICAL_FRAME_ALLOCATOR.lock().allocate();
        logln!("alloc2: {:?}, alloc3: {:?}", alloc2, alloc3);
        if let Err(e) = PHYSICAL_FRAME_ALLOCATOR.lock().deallocate(alloc2.unwrap()) {
            logln!("Failed to deallocate frame: {:?}", e);
        }
        if let Err(e) = PHYSICAL_FRAME_ALLOCATOR.lock().deallocate(alloc3.unwrap()) {
            logln!("Failed to deallocate frame: {:?}", e);
        }
        logln!("Single frame allocation and deallocation test complete.");
        logln!("Performing contiguous frame allocation and deallocation test.");
        let contiguous_alloc = PHYSICAL_FRAME_ALLOCATOR.lock().allocate_contiguous(256, 64);
        match contiguous_alloc {
            Ok(frame) => {
                logln!(
                    "Allocated physically contiguous region with physical base address: {:?}",
                    frame
                );
                let _ = PHYSICAL_FRAME_ALLOCATOR.lock().deallocate(frame);
                logln!(
                    "Deallocated physically contiguous region with physical base address: {:?}",
                    frame
                );
            }
            Err(e) => {
                logln!("Failed to allocate contiguous frames: {:?}", e);
            }
        }
        logln!("Contiguous frame allocation and deallocation test complete.");
        logln!("Physical Memory Manager test suite finished.");
    }

    fn vmm_self_test() {
        logln!("Beginning VMM Self Test...");
        let cr3 = unsafe { get_cr3() };
        let mut pm = match PageMap::from_cr3(cr3) {
            Ok(pm) => pm,
            Err(e) => panic!("Failed to create PageMap from CR3: {:?}", e),
        };
        logln!("PageMap created from current CR3 value.");

        logln!("Starting page mapping test...");
        let frame = match PHYSICAL_FRAME_ALLOCATOR.lock().allocate() {
            Ok(frame) => frame,
            Err(e) => panic!("Failed to allocate frame: {:?}", e),
        };

        //map to the beginning of the higher half of the virtual address space i.e. the beginning of kernelspace
        let vaddr = match VirtualAddress::try_from(0xFFFF800000000000usize) {
            Ok(vaddr) => vaddr,
            Err(e) => {
                panic!("Failed to create VirtualAddress: {:?}", e);
            }
        };
        pm.map_page(
            vaddr,
            frame,
            PteFlags::Write as u64 | PteFlags::Global as u64 | PteFlags::NoExecute as u64,
        );
        logln!(
            "Mapped page at virtual address: {:?} to physical frame: {:?}",
            vaddr,
            frame
        );
        unsafe {
            let ptr = <*mut u64>::from(vaddr);
            ptr.write(0xdeadbeef);
            logln!("Wrote 0xdeadbeef to virtual address: {:?}", vaddr);
            let val = ptr.read();
            logln!("Read value: 0x{:x} from virtual address: {:?}", val, vaddr);
        }
        let _ = pm.unmap_page(vaddr);
        logln!("Unmapped page at virtual address: {:?}", vaddr);
        let _ = PHYSICAL_FRAME_ALLOCATOR.lock().deallocate(frame);
        logln!("Deallocated frame: {:?}", frame);
        logln!("Page mapping test successful.");

        logln!("Starting large page mapping test...");
        let large_frame = match PHYSICAL_FRAME_ALLOCATOR
            .lock()
            .allocate_contiguous(512, 4096 * 512)
        {
            Ok(frame) => frame,
            Err(e) => panic!("Failed to allocate frame: {:?}", e),
        };
        pm.map_large_page(
            vaddr,
            large_frame,
            PteFlags::Write as u64 | PteFlags::Global as u64 | PteFlags::NoExecute as u64,
        );
        logln!(
            "Mapped large page at virtual address: {:?} to physical frame: {:?}",
            vaddr,
            large_frame
        );
        unsafe {
            let ptr = <*mut u64>::from(vaddr);
            ptr.write(0xcafebabe);
            logln!("Wrote 0xcafebabe to virtual address: {:?}", vaddr);
            let val = ptr.read();
            logln!("Read value: 0x{:x} from virtual address: {:?}", val, vaddr);
        }
        let _ = pm.unmap_large_page(vaddr);
        logln!("Unmapped large page at virtual address: {:?}", vaddr);
        let _ = PHYSICAL_FRAME_ALLOCATOR
            .lock()
            .deallocate_contiguous(large_frame, 512);
        logln!("Deallocated large frame: {:?}", large_frame);
        logln!("Large page mapping test successful.");

        logln!("Starting huge page mapping test...");
        let huge_frame = match PHYSICAL_FRAME_ALLOCATOR
            .lock()
            .allocate_contiguous(512 * 512, 4096 * 512 * 512)
        {
            Ok(frame) => frame,
            Err(e) => panic!("Failed to allocate frame: {:?}", e),
        };
        pm.map_huge_page(
            vaddr,
            huge_frame,
            PteFlags::Write as u64 | PteFlags::Global as u64 | PteFlags::NoExecute as u64,
        );
        logln!(
            "Mapped huge page at virtual address: {:?} to physical frame: {:?}",
            vaddr,
            huge_frame
        );
        unsafe {
            let ptr = <*mut u64>::from(vaddr);
            ptr.write(0xdeadbeef);
            logln!("Wrote 0xdeadbeef to virtual address: {:?}", vaddr);
            let val = ptr.read();
            logln!("Read value: 0x{:x} from virtual address: {:?}", val, vaddr);
        }
        let _ = pm.unmap_huge_page(vaddr);
        logln!("Unmapped huge page at virtual address: {:?}", vaddr);
        let _ = PHYSICAL_FRAME_ALLOCATOR
            .lock()
            .deallocate_contiguous(huge_frame, 512 * 512);
        logln!("Deallocated huge frame: {:?}", huge_frame);
        logln!("Huge page mapping test successful.");

        logln!("VMM Self Test Complete.");
    }
}
