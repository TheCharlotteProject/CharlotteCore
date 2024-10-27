use page_table_entry::*;

use crate::arch::x86_64::memory::*;
use crate::arch::ISA_PARAMS;
use crate::memory::address::*;
use crate::memory::pmm::PHYSICAL_FRAME_ALLOCATOR;

pub mod page_table_entry;

#[derive(Debug, PartialEq, Eq)]
pub enum PageSize {
    Standard = 0,
    Large = 1,
    Huge = 2,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PageTableLevel {
    PML4 = 4,
    PDPT = 3,
    PD = 2,
    PT = 1,
}

const N_PT_ENTRIES: usize = 512;
const LARGE_PAGE_NFRAMES: usize = 512;
const HUGE_PAGE_NFRAMES: usize = 512 * 512;

#[repr(C, align(4096))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PageTable {
    table: [PageTableEntry; N_PT_ENTRIES],
}

impl PageTable {
    pub fn new() -> Self {
        Self {
            table: [PageTableEntry::new(); N_PT_ENTRIES],
        }
    }

    pub fn map_table(&mut self, index: usize, flags: u64) -> Result<PhysicalAddress, Error> {
        let table_paddr = PHYSICAL_FRAME_ALLOCATOR.lock().allocate()?;
        self.table[index].map_table(table_paddr, flags)?;
        Ok(PHYSICAL_FRAME_ALLOCATOR.lock().allocate().unwrap())
    }

    pub unsafe fn unmap_table(&mut self, index: usize) -> Result<(), Error> {
        if self.table[index].is_present() {
            if !self.table[index].is_size_bit_set() {
                let table_paddr = self.table[index].unmap()?;
                PHYSICAL_FRAME_ALLOCATOR.lock().deallocate(table_paddr)?;
                Ok(())
            } else {
                Err(Error::EntryNotTable)
            }
        } else {
            Err(Error::EntryNotPresent)
        }
    }

    pub fn map_page(
        &mut self,
        size: PageSize,
        index: usize,
        paddr: PhysicalAddress,
        flags: u64,
    ) -> Result<(), Error> {
        match size {
            PageSize::Standard => {
                if !paddr.is_page_aligned() {
                    return Err(Error::InvalidPAddrAlignment);
                }
            }
            PageSize::Large => {
                if !paddr.is_aligned_to(ISA_PARAMS.paging.page_size * LARGE_PAGE_NFRAMES) {
                    return Err(Error::InvalidPAddrAlignment);
                }
            }
            PageSize::Huge => {
                if !paddr.is_aligned_to(ISA_PARAMS.paging.page_size * HUGE_PAGE_NFRAMES) {
                    return Err(Error::InvalidPAddrAlignment);
                }
            }
        }
        self.table[index].map_page(paddr, flags, size)?;
        Ok(())
    }

    pub unsafe fn unmap_page(
        &mut self,
        size: PageSize,
        index: usize,
    ) -> Result<PhysicalAddress, Error> {
        let page_paddr = self.table[index].unmap()?;
        match size {
            PageSize::Standard => PHYSICAL_FRAME_ALLOCATOR.lock().deallocate(page_paddr)?,
            PageSize::Large => PHYSICAL_FRAME_ALLOCATOR
                .lock()
                .deallocate_contiguous(page_paddr, LARGE_PAGE_NFRAMES)?,
            PageSize::Huge => PHYSICAL_FRAME_ALLOCATOR
                .lock()
                .deallocate_contiguous(page_paddr, HUGE_PAGE_NFRAMES)?,
        }
        Ok(page_paddr)
    }

    pub fn get_or_map_table(
        &mut self,
        vaddr: VirtualAddress,
        level: PageTableLevel,
        flags: u64,
    ) -> Result<*mut PageTable, Error> {
        let index = match level {
            PageTableLevel::PML4 => vaddr.pml4_index(),
            PageTableLevel::PDPT => vaddr.pdpt_index(),
            PageTableLevel::PD => vaddr.pd_index(),
            PageTableLevel::PT => vaddr.pt_index(),
        };
        if self.table[index].is_present() {
            match level {
                PageTableLevel::PDPT | PageTableLevel::PD => {
                    if self.table[index].is_size_bit_set() {
                        return Err(Error::VAddrRangeUnavailable);
                    }
                }
                _ => {}
            }
            Ok(<*mut PageTable>::from(self.table[index].addr().unwrap()))
        } else {
            Ok(<*mut PageTable>::from(self.map_table(index, flags)?))
        }
    }
}
