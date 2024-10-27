//! # Allocator
//!
//! The kernel allocator provides a single unified interface for allocating and deallocating memory
//! in the kernel's address space. The general allocation strategy is to allocate memory starting from
//! the back of the kernel heap and moving towards the front. This is done to prevent fragmentation and to
//! allow the allocator's allocation tracking vector to grow upwards starting from the kernel heap's base address.
//! 
//! Allocation tracking works as follows:
//! The dynamic memory segment is the the section of the kernel heap that is currently allocated inlcuding both the
//! parts that are currently in use and the parts that are free. The dynamic memory segment itself is tracked using a
//! single value called `DYN_SEG_FRONT`. This value is a pointer to the front of the dynamic memory segment. The back of
//! the dynamic memory segment is the `KERNEL_HEAP_END` address. Entries in the allocation tracking vector track which parts
//! of the dynamic memory segment are in use and which parts are free. Each entry in the allocation tracking vector is a
//! enum designed to efficiently represent the state of a memory block. The enum has three variants: 
//! - `Available` - Represents a free memory section
//! - `Subpage` - Represents a memory section that is in use and is smaller than a page

use spin::lazy::Lazy;

use crate::bootinfo::KERNEL_ADDRESS_REQUEST;
use crate::memory::address::*;

static KERNEL_HEAP_START: Lazy<VirtualAddress> =
    Lazy::new(|| VirtualAddress::try_from(0x8000_0000_0000usize).unwrap());

static KERNEL_HEAP_END: Lazy<VirtualAddress> = Lazy::new(|| {
    let kaddr_response = KERNEL_ADDRESS_REQUEST
        .get_response()
        .expect("Failed to obtain kernel address from Limine");
    VirtualAddress::try_from(kaddr_response.virtual_base())
        .expect("Could not convert the kernel base address provided by Limine to a VirtualAddress")
});

enum Error {
    OutOfMemory,
    AlignmentUnavailable,
}

/// A manually managed vector for use in the kernel allocator
#[derive(Debug, Copy, Clone)]
struct ManualVec<T> {
    base: *mut T,
    capacity: usize,
    length: usize,
}
#[derive(Debug, Copy, Clone)]
struct Buffer {
    base: VirtualAddress,
    size: usize,
}
#[derive(Debug, Copy, Clone)]
enum AllocationState {
    Available(Buffer),
    Subpage(Buffer),
    Page(Buffer),
}

trait Alloc {
    extern "C" fn alloc(&mut self, size: usize, alignment: usize) -> Result<VirtualAddress, Error>;
    extern "C" fn dealloc(&mut self, addr: VirtualAddress);
}
/// The kernel allocator
pub struct Allocator {
    tracking_vec: ManualVec<AllocationState>,
    dyn_mem_front: VirtualAddress,
}




