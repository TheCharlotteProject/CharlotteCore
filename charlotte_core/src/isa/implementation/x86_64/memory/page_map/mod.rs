pub mod page_table;

use page_table::page_table_entry::PteFlags;
use page_table::PageTable;

use super::Error;

use core::arch::asm;
use core::fmt::Write;
use core::num::NonZeroUsize;
use core::ptr::addr_of_mut;

use crate::arch::x86_64::cpu::ARE_HUGE_PAGES_SUPPORTED;
use crate::arch::{Api, ArchApi, MemoryMap};
use crate::logln;
use crate::memory::address::{MemoryAddress, VirtualAddress};
use crate::memory::{address::PhysicalAddress, pmm::PHYSICAL_FRAME_ALLOCATOR};

static N_FRAMES_PDPT: usize = 512 * 512;
static N_FRAMES_PD: usize = 512;

struct Walker<'a> {
    page_map: &'a PageMap,
    pml4: Option<&'a mut PageTable>,
    pdpt: Option<&'a mut PageTable>,
    pd: Option<&'a mut PageTable>,
    pt: Option<&'a mut PageTable>,
}

impl<'a> Walker<'a> {
    fn new(page_map: &'a PageMap) -> Self {
        Self {
            page_map: page_map,
            pml4: None,
            pdpt: None,
            pd: None,
            pt: None,
        }
    }
    fn walk_cr3(&mut self) -> Result<(), Error> {
        unsafe {
            self.pml4 = Some(&mut *(<*mut PageTable>::from(self.page_map.get_pml4_paddr())));
        }
        Ok(())
    }
    fn walk_pml4(&mut self, vaddr: VirtualAddress, flags: u64) -> Result<(), Error> {
        match &mut self.pml4 {
            Some(pml4) => {
                unsafe {
                    let pml4_ptr = addr_of_mut!(*pml4);
                    self.pdpt = Some(
                        &mut *((*pml4_ptr).get_or_map_table(
                            vaddr,
                            page_table::PageTableLevel::PML4,
                            flags,
                        )?),
                    );
                }
                Ok(())
            }
            None => {
                self.walk_cr3()?;
                self.walk_pml4(vaddr, flags)
            }
        }
    }

    fn walk_pdpt(&mut self, vaddr: VirtualAddress, flags: u64) -> Result<(), Error> {
        match &mut self.pdpt {
            Some(pdpt) => {
                unsafe {
                    let pdpt_ptr = addr_of_mut!(*pdpt);
                    self.pd = Some(
                        &mut *((*pdpt_ptr).get_or_map_table(
                            vaddr,
                            page_table::PageTableLevel::PDPT,
                            flags,
                        )?),
                    );
                }
                Ok(())
            }
            None => {
                logln!("Walking PML4");
                self.walk_pml4(vaddr, flags)?;
                self.walk_pdpt(vaddr, flags)
            }
        }
    }

    fn walk_pd(&mut self, vaddr: VirtualAddress, flags: u64) -> Result<(), Error> {
        match &mut self.pd {
            Some(pd) => {
                unsafe {
                    let pd_ptr = addr_of_mut!(*pd);
                    logln!("Obtained PD pointer: {:p}", pd_ptr);
                    self.pd = Some(
                        &mut *((*pd_ptr).get_or_map_table(
                            vaddr,
                            page_table::PageTableLevel::PD,
                            flags,
                        )?),
                    );
                    logln!("Obtained or Mapped PD table.");
                }
                Ok(())
            }
            None => {
                logln!("Walking PDPT");
                self.walk_pdpt(vaddr, flags)?;
                self.walk_pd(vaddr, flags)
            }
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct PageMap {
    cr3: u64,
}

impl PageMap {
    pub fn try_new() -> Result<Self, Error> {
        Ok(PageMap {
            cr3: PHYSICAL_FRAME_ALLOCATOR.lock().allocate()?.bits() as u64,
        })
    }
    pub fn from_cr3(cr3: u64) -> Result<Self, Error> {
        // clear the PCID bits
        //cr3 &= !0xFFF;

        if ArchApi::validate_paddr(cr3 as usize) == false {
            Err(Error::InvalidAddress)
        } else {
            Ok(PageMap { cr3: cr3 })
        }
    }
    pub fn get_pml4_paddr(&self) -> PhysicalAddress {
        PhysicalAddress::from((self.cr3 & !0xFFF) as usize)
    }
    pub fn get_pcid(&self) -> u16 {
        (self.cr3 & 0xFFF) as u16
    }
    pub fn set_pcid(&mut self, pcid: u16) -> Result<(), Error> {
        if self.get_pcid() != 0 {
            Err(Error::AlredyHasPcid)
        } else {
            self.cr3 = (self.cr3 & !0xFFF) | pcid as u64;
            Ok(())
        }
    }
    fn invalidate_pcid(&self) {
        let mut pcid = [0u64; 2];
        pcid[0] = self.get_pcid() as u64;
        unsafe {
            asm! {
                "invpcid 1, [{pcid}]",
                pcid = in(reg) pcid.as_ptr(),
            }
        }
    }
    fn is_range_available(&self, start: VirtualAddress, size: NonZeroUsize) -> bool {
        let mut walker = Walker::new(self);
        let mut vaddr = start;
        let n_huge_pages = size.get() / N_FRAMES_PDPT;
        let mut rem_size = size.get() % N_FRAMES_PDPT;
        let n_large_pages = rem_size / N_FRAMES_PD;
        rem_size = rem_size % N_FRAMES_PD;
        let n_pages = rem_size / 4096 + 1;

        for _ in 0..n_huge_pages {
            if walker.walk_pml4(vaddr, 0).is_err() {
                return false;
            }
            if walker.walk_pdpt(vaddr, 0).is_err() {
                return false;
            }
            if walker.walk_pd(vaddr, 0).is_err() {
                return false;
            }
            if walker.walk_pd(vaddr, 0).is_err() {
                return false;
            }
            vaddr += N_FRAMES_PDPT * 4096;
        }
        for _ in 0..n_large_pages {
            if walker.walk_pml4(vaddr, 0).is_err() {
                return false;
            }
            if walker.walk_pdpt(vaddr, 0).is_err() {
                return false;
            }
            if walker.walk_pd(vaddr, 0).is_err() {
                return false;
            }
            vaddr += N_FRAMES_PD * 4096;
        }
        for _ in 0..n_pages {
            if walker.walk_pml4(vaddr, 0).is_err() {
                return false;
            }
            if walker.walk_pdpt(vaddr, 0).is_err() {
                return false;
            }
            if walker.walk_pd(vaddr, 0).is_err() {
                return false;
            }
            vaddr += 4096;
        }
        true
    }
}

impl MemoryMap for PageMap {
    type Error = Error;
    type Flags = u64;

    fn get_flags(mem_type: crate::arch::MemType) -> Self::Flags {
        match mem_type {
            crate::arch::MemType::KernelReadWrite => {
                PteFlags::Present as u64
                | PteFlags::Write as u64
                | PteFlags::NoExecute as u64
                | PteFlags::Global as u64
                | PteFlags::WriteThrough as u64
            },
            crate::arch::MemType::KernelReadOnly => {
                PteFlags::Present as u64
                | PteFlags::NoExecute as u64
                | PteFlags::Global as u64
                | PteFlags::WriteThrough as u64
            },
            crate::arch::MemType::KernelReadExecute => {
                PteFlags::Present as u64
                | PteFlags::Global as u64
                | PteFlags::WriteThrough as u64
            },
        }
    }

    /// Loads the page map into the logical processor.
    unsafe fn load(&self) -> Result<(), Self::Error> {
        if self.get_pcid() != 0 {
            unsafe {
                asm! {
                    "mov cr3, {0}",
                    in(reg) self.cr3,
                }
            }
            Ok(())
        } else {
            Err(Error::InvalidPcid)
        }
    }

    /// Maps a page at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to map the page to
    /// * `paddr` - The physical base address of the page frame to be mapped
    /// * `flags` - The flags to apply to the page table entry
    fn map_page(
        &mut self,
        vaddr: VirtualAddress,
        paddr: PhysicalAddress,
        flags: Self::Flags,
    ) -> Result<(), Self::Error> {
        if vaddr.is_aligned_to(crate::arch::ISA_PARAMS.paging.page_size) == false {
            Err(Error::InvalidVAddrAlignment)
        } else if vaddr.is_null() {
            Err(Error::InvalidAddress)
        } else {
            let mut walker = Walker::new(self);
            logln!("Walker created.");
            walker.walk_pd(vaddr, flags)?;
            logln!("Walker walked to PD.");
            walker.pt.unwrap().map_page(
                page_table::PageSize::Standard,
                vaddr.pt_index(),
                paddr,
                flags,
            )?;

            Ok(())
        }
    }

    /// Unmaps a page from the given page map at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to unmap.
    /// # Returns
    /// Returns an error of type `Self::Error` if unmapping fails or the physical address that was
    /// previously mapped to the given virtual address if successful.
    fn unmap_page(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, Self::Error> {
        let mut walker = Walker::new(self);
        walker.walk_pd(vaddr, 0)?;
        unsafe {
            walker
                .pt
                .unwrap()
                .unmap_page(page_table::PageSize::Standard, vaddr.pt_index())
        }
    }

    /// Maps a large page (2 MiB) at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to map.
    /// * `paddr` - The physical address to map.
    /// * `flags` - The flags to apply to the page table entry.
    /// # Returns
    /// Returns an error of type `Self::Error` if mapping fails.
    fn map_large_page(
        &mut self,
        vaddr: VirtualAddress,
        paddr: PhysicalAddress,
        flags: Self::Flags,
    ) -> Result<(), Self::Error> {
        let mut walker = Walker::new(self);
        walker.walk_pdpt(vaddr, flags)?;
        walker
            .pd
            .unwrap()
            .map_page(page_table::PageSize::Large, vaddr.pd_index(), paddr, flags)
    }

    /// Unmaps a large page from the given page map at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to unmap.
    /// # Returns
    /// Returns an error of type `Self::Error` if unmapping fails or the physical address that was
    /// previously mapped to the given virtual address if successful.
    fn unmap_large_page(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, Self::Error> {
        let mut walker = Walker::new(self);
        walker.walk_pdpt(vaddr, 0)?;
        unsafe {
            walker
                .pd
                .unwrap()
                .unmap_page(page_table::PageSize::Large, vaddr.pd_index())
        }
    }

    /// Maps a huge page (1 GiB) at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to map.
    /// * `paddr` - The physical address to map.
    /// * `flags` - The flags to apply to the page table entry.
    /// # Returns
    /// Returns an error of type `Self::Error` if mapping fails.
    fn map_huge_page(
        &mut self,
        vaddr: VirtualAddress,
        paddr: PhysicalAddress,
        flags: Self::Flags,
    ) -> Result<(), Self::Error> {
        if *ARE_HUGE_PAGES_SUPPORTED == false {
            Err(Error::UnsupportedOperation)
        } else {
            let mut walker = Walker::new(self);
            walker.walk_pml4(vaddr, flags)?;
            walker.pdpt.unwrap().map_page(
                page_table::PageSize::Huge,
                vaddr.pdpt_index(),
                paddr,
                flags,
            )
        }
    }

    /// Unmaps a huge page from the given page map at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to unmap.
    /// # Returns
    /// Returns an error of type `Self::Error` if unmapping fails or the physical address that was
    /// previously mapped to the given virtual address if successful.
    fn unmap_huge_page(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, Self::Error> {
        if *ARE_HUGE_PAGES_SUPPORTED == false {
            Err(Error::UnsupportedOperation)
        } else {
            let mut walker = Walker::new(self);
            walker.walk_pml4(vaddr, 0)?;
            unsafe {
                walker
                    .pdpt
                    .unwrap()
                    .unmap_page(page_table::PageSize::Huge, vaddr.pdpt_index())
            }
        }
    }

    /// Finds an available region of memory within the given range that is large enough to hold the
    /// requested size.
    /// # Arguments
    /// * `size` - The size of the region to find.
    /// * `alignment` - The alignment of the region to find.
    /// * `start` - The start of the range to search.
    /// * `end` - The end of the range to search.
    /// # Returns
    /// Returns the base address of the region if one is found, or an error of type `Self::Error` if
    /// no region is found or if an error occurs during the search.
    fn find_available_region(
        &self,
        size: NonZeroUsize,
        alignment: usize,
        start: VirtualAddress,
        end: VirtualAddress,
    ) -> Result<VirtualAddress, Self::Error> {
        /*Input Validation*/
        if size.get() < 4096 {
            return Err(Error::SubPageSizeNotAllowed);
        }
        if alignment < 4096 {
            return Err(Error::InvalidArgument);
        }
        if start.is_aligned_to(alignment) == false {
            return Err(Error::InvalidVAddrAlignment);
        }
        /*Search all possible regions of the specified range that could work and return the first one that does*/
        for addr in (start.aligned_after(alignment)..end).step_by(alignment as usize) {
            if self.is_range_available(addr, size) {
                return Ok(addr);
            }
        }
        // If no region is found, return an error
        Err(Error::VAddrRangeUnavailable)
    }
}

#[inline]
pub fn get_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        asm! {
            "mov {0}, cr3",
            out(reg) cr3,
        }
    }
    cr3
}
#[inline]
fn get_cr4() -> u64 {
    let cr4: u64;
    unsafe {
        asm! {
            "mov {0}, cr4",
            out(reg) cr4,
        }
    }
    cr4
}
#[inline]
fn load_page_map(cr3: u64) {
    unsafe {
        asm! {
            "mov cr3, {0}",
            in(reg) cr3,
        }
    }
}
#[inline]
fn invalidate_tlb_entry(vaddr: VirtualAddress) {
    unsafe {
        asm! {
            "invlpg [{0}]",
            in(reg) vaddr.bits(),
        }
    }
}
