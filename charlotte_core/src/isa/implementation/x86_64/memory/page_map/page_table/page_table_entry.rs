use super::PageSize;

use crate::arch::x86_64::memory::*;
use crate::memory::address::*;

static ADDR_MASK: Lazy<u64> = Lazy::new(|| {
    // Create a mask that will clear all bits in a PTE that are not address bits
    let mut mask = (1 << *PADDR_SIGBITS) - 1;
    mask &= !0xFFF; // Clear the lower 12 bits
    mask
});

/// Page Table Entry Flags
/// Any items prefixed with `Cc` are specific to this kernel and are not part of the x86_64 ISA
pub enum PteFlags {
    Present = 1,
    Write = 1 << 1,
    User = 1 << 2,
    WriteThrough = 1 << 3,
    CacheDisable = 1 << 4,
    Accessed = 1 << 5,
    Dirty = 1 << 6,         // Only for entries that point to pages
    PageSizeOrPat = 1 << 7, // PageSize for entires in the PDPT, and PD for 1GiB and 2MiB pages and PAT for 4KiB pages
    Global = 1 << 8,
    HugeAndLargePat = 1 << 12, // Only for entries in the PDPT, and PD for 1GiB and 2MiB pages
    CcCopyOnWrite = 1 << 52, // Only for entries that point to pages. This bit indicates that the page should be copied on write
    CcShared = 1 << 53, // Only for entries that point to pages. This bit indicates that the page is shared between multiple address spaces
    NoExecute = 1 << 63,
}

static FLAG_MASK: u64 = PteFlags::Present as u64
    | PteFlags::Write as u64
    | PteFlags::User as u64
    | PteFlags::WriteThrough as u64
    | PteFlags::CacheDisable as u64
    | PteFlags::Accessed as u64
    | PteFlags::Dirty as u64
    | PteFlags::PageSizeOrPat as u64
    | PteFlags::Global as u64
    | PteFlags::CcCopyOnWrite as u64
    | PteFlags::CcShared as u64
    | PteFlags::NoExecute as u64;

static HUGE_AND_LARGE_PAGE_FLAG_MASK: u64 = FLAG_MASK | PteFlags::HugeAndLargePat as u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    #[inline]
    pub fn new() -> Self {
        Self { entry: 0 }
    }

    #[inline]
    pub fn addr(&self) -> Result<PhysicalAddress, Error> {
        if self.is_present() == false {
            Err(Error::EntryNotPresent)
        } else {
            Ok(PhysicalAddress::from((self.entry & *ADDR_MASK) as usize))
        }
    }

    pub fn map_table(&mut self, paddr: PhysicalAddress, flags: u64) -> Result<(), Error> {
        if self.is_present() {
            Err(Error::VAddrRangeUnavailable)
        } else if !paddr.is_page_aligned() {
            Err(Error::InvalidPAddrAlignment)
        } else {
            self.entry = (paddr.bits() as u64 & *ADDR_MASK) | (flags & FLAG_MASK);
            Ok(())
        }
    }

    pub fn map_page(
        &mut self,
        paddr: PhysicalAddress,
        flags: u64,
        size: PageSize,
    ) -> Result<(), Error> {
        if self.is_present() {
            Err(Error::VAddrRangeUnavailable)
        } else if !paddr.is_page_aligned() {
            Err(Error::InvalidPAddrAlignment)
        } else {
            let flag_mask = if size == PageSize::Standard {
                FLAG_MASK
            } else {
                HUGE_AND_LARGE_PAGE_FLAG_MASK
            };
            self.entry = (paddr.bits() as u64 & *ADDR_MASK) | (flags & flag_mask);
            Ok(())
        }
    }

    pub fn unmap(&mut self) -> Result<PhysicalAddress, Error> {
        let paddr = self.addr()?;
        self.entry = 0;
        Ok(paddr)
    }

    #[inline]
    pub fn is_present(&self) -> bool {
        self.entry & PteFlags::Present as u64 != 0
    }

    #[inline]
    pub fn is_size_bit_set(&self) -> bool {
        self.entry & PteFlags::PageSizeOrPat as u64 != 0
    }
}
