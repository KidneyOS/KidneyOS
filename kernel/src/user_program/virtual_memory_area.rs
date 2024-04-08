/* The function of the VMA is to define a contiguous area of virtual memory (contiguous virtual addresses) with the correct permissions such as read, write, execute,
and whether the area is private to the process or shared with others, and the file (if any) that the region is mapped to. Operating systems with virtual memory use VMAs
to manage memory protection, to share memory between processes, and to lazily allocate memory, among other things. When a binary file, like an ELF executable, is loaded,
the operating system creates VMAs for its different sections, such as code, data, and BSS segments, according to the program headers in the ELF file.  */

// VmOperations to perform operations related to the VMA (might implement in future).
#[allow(unused)]
pub trait VmOperations {
    fn open(&self, area: &VmAreaStruct); // This function is invoked when the given memory area is added to an address space. Such as when it is first set up or when it's inherited during a fork.
    fn close(&self, area: &VmAreaStruct); // This function is invoked when the given memory area is removed from an address space. Like during process termination or when unloading a shared library.
                                          // fn nopage(&self, area: &VmAreaStruct, address: usize) -> Option<Page>; // This function is invoked by the page fault handler when a page that is not present in physical memory is accessed.// This function is invoked by the remap_pages() system call to prefault a new mapping. Typically to reduce the number of page faults during runtime.
}

pub struct VMAOperations;

// Implementations of VmOperations.
impl VmOperations for VMAOperations {
    #[allow(unused)]
    fn open(&self, area: &VmAreaStruct) {
        // Implement open logic
    }

    #[allow(unused)]
    fn close(&self, area: &VmAreaStruct) {
        // Implement close logic
    }
}

/* Represents a Virtual Memory Area in an operating system's memory management system.
It contains fields for the start and end addresses of the VMA, an instance of VmFlags which stores various flags as a bitfield.*/
pub struct VmAreaStruct {
    pub vm_start: usize, // VMA start, inclusive
    pub vm_end: usize,   // VMA end, exclusive
    pub flags: VmFlags,
    // TODO: Add other necessary fields here
}

/* Stores boolean flags. The flags include read, write, execute, shared, and private, each represented as a single bit.
This allows the operating system to track the properties and permissions of different memory areas allocated to a process.*/
#[derive(Debug, PartialEq, Eq, Default)]
pub struct VmFlags {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub shared: bool,
    pub private: bool,
    // TODO: add other necessary flags.
}

impl VmAreaStruct {
    pub fn new(vm_start: usize, vm_end: usize, flags: VmFlags) -> Self {
        Self {
            vm_start,
            vm_end,
            flags,
            // TODO: initialize other fields
        }
    }
}
