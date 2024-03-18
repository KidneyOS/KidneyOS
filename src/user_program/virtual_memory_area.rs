use modular_bitfield::prelude::*;

pub trait VmOperations {
    fn open(&self, area: &VmAreaStruct); // This function is invoked when the given memory area is added to an address space. Such as when it is first set up or when it's inherited during a fork.
    fn close(&self, area: &VmAreaStruct); // This function is invoked when the given memory area is removed from an address space. Like during process termination or when unloading a shared library.
    fn nopage(&self, area: &VmAreaStruct, address: usize) -> Option<Page>; // This function is invoked by the page fault handler when a page that is not present in physical memory is accessed.
    fn populate(
        &self,
        area: &VmAreaStruct,
        address: usize,
        len: usize,
        prot: PprotT,
        pgoff: usize,
        nonblock: i32,
    ); // This function is invoked by the remap_pages() system call to prefault a new mapping. Typically to reduce the number of page faults during runtime.
}

pub struct VMAOperations; 

impl VmOperations for VMAOperations {
    fn open(&self, area: &VmAreaStruct) {
        // Implement open logic
    }

    fn close(&self, area: &VmAreaStruct) {
        // Implement close logic
    }

    fn nopage(&self, area: &VmAreaStruct, address: usize) -> Option<Page> {
        // Implement nopage logic, returning Some(Page) or None
        None
    }

    fn populate(&self, area: &VmAreaStruct, address: usize, len: usize, prot: PprotT, pgoff: usize, nonblock: i32) {
        // Implement populate logic
    }
}

struct VmAreaStruct {
    vm_start: usize, // VMA start, inclusive
    vm_end: usize,   // VMA end, exclusive
    flags: VmFlags,
    vm_ops: Box<dyn VmOperations>,
    // TODO: Add other necessary fields here
}

#[bitfield]
#[repr(u64)] // Use u64 as the underlying storage type for the bitfield
#[derive(Debug, PartialEq, Eq)]
pub struct VmFlags {
    read: bool,
    write: bool,
    execute: bool,
    shared: bool,
    private: bool,
    // TODO: add other necessary flags.
    #[skip] // Skip the remaining bits to fill up to 64 bits
    __unused: B59,
}

impl VmAreaStruct {
    fn new(vm_start: usize, vm_end: usize, flags: VmFlags) -> Self {
        Self {
            vm_start,
            vm_end,
            flags,
            // TODO: initialize other fields
        }
    }
}
