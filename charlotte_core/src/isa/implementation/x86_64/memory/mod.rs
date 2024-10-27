pub mod page_map;

use core::arch::x86_64::__cpuid_count;

use crate::memory::address::{PhysicalAddress, VirtualAddress};
use crate::memory::pmm::Error as PmmError;
use spin::lazy::Lazy;

/// The number of significant binary digits in a physical address
pub static PADDR_SIGBITS: Lazy<u8> = Lazy::new(|| {
    let cpuid_result = unsafe { __cpuid_count(0x80000008, 0) };
    (cpuid_result.eax & 0xFF) as u8
});

/// This is the number of significant binary digits in a virtual (linear) address
/// This will not be used for now however it is here to support an implementation of
/// 5-level paging in the future
pub static VADDR_SIGBITS: Lazy<u8> = Lazy::new(|| {
    let cpuid_result = unsafe { __cpuid_count(0x80000001, 0) };
    if cpuid_result.ecx & (1 << 16) != 0 {
        57u8
    } else {
        48u8
    }
});

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedOperation,
    InvalidArgument,
    InvalidAddress,
    InvalidPAddrAlignment,
    InvalidVAddrAlignment,
    OutOfMemory,
    VAddrRangeUnavailable,
    EntryNotPresent,
    EntryNotTable,
    NoSizeBit,
    OpNotSupportedAtThisLevel,
    AlredyHasPcid,
    InvalidPcid,
    SubPageSizeNotAllowed,
    PmmError(PmmError),
}

impl From<PmmError> for Error {
    fn from(error: PmmError) -> Self {
        Error::PmmError(error)
    }
}

extern "C" {
    fn asm_load_page_map(paddr: PhysicalAddress);
    fn asm_invalidate_tlb_entry(vaddr: VirtualAddress);
    pub fn asm_get_cr4() -> u64;
}
