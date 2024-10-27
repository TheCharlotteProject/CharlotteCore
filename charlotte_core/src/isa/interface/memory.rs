//! # Memory ISA Interface
//! This module provides an interface to ISA specific functionality required for memory management.

pub enum MemType {
    KernelReadWrite,
    KernelReadOnly,
    KernelReadExecute,
}

pub trait MemoryMap {
    type Error;
    type Flags;

    fn get_flags(mem_type: MemType) -> Self::Flags;

    /// Loads the page map into the logical processor.
    unsafe fn load(&self) -> Result<(), Self::Error>;

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
    ) -> Result<(), Self::Error>;

    /// Unmaps a page from the given page map at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to unmap.
    /// # Returns
    /// Returns an error of type `Self::Error` if unmapping fails or the physical address that was
    /// previously mapped to the given virtual address if successful.
    fn unmap_page(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, Self::Error>;

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
    ) -> Result<(), Self::Error>;

    /// Unmaps a large page from the given page map at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to unmap.
    /// # Returns
    /// Returns an error of type `Self::Error` if unmapping fails or the physical address that was
    /// previously mapped to the given virtual address if successful.
    fn unmap_large_page(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, Self::Error>;

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
    ) -> Result<(), Self::Error>;

    /// Unmaps a huge page from the given page map at the given virtual address.
    /// # Arguments
    /// * `vaddr` - The virtual address to unmap.
    /// # Returns
    /// Returns an error of type `Self::Error` if unmapping fails or the physical address that was
    /// previously mapped to the given virtual address if successful.
    fn unmap_huge_page(&mut self, vaddr: VirtualAddress) -> Result<PhysicalAddress, Self::Error>;

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
    ) -> Result<VirtualAddress, Self::Error>;
}