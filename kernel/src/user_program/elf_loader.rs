/* This file is part of a system that loads and validates ELF (Executable and Linkable Format) binaries,
commonly used for executables and shared libraries in Unix-like operating systems. See ELF Spec: https://refspecs.linuxfoundation.org/elf/elf.pdf */

use super::virtual_memory_area::{VmAreaStruct, VmFlags};
use alloc::vec::Vec;
use kidneyos_shared::mem::{OFFSET, PAGE_FRAME_SIZE};

/* Executable header.  See [ELF1] 1-4 to 1-8.
This appears at the very beginning of an ELF binary. */
#[repr(C)]
#[derive(Debug)]
struct Elf32Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u32,
    e_phoff: u32,
    e_shoff: u32,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

/* Program header.  See [ELF1] 2-2 to 2-4.*/
#[repr(C)]
#[derive(Debug)]
struct Elf32Phdr {
    p_type: u32,
    p_offset: u32,
    p_vaddr: u32,
    p_paddr: u32,
    p_filesz: u32,
    p_memsz: u32,
    p_flags: u32,
    p_align: u32,
}

/* Values for p_type.  See [ELF1] 2-3. */
#[repr(u32)]
#[derive(Debug, PartialEq)]
#[allow(unused)]
pub enum SegmentType {
    Null = 0,           // Ignore.
    Load = 1,           // Loadable segment.
    Dynamic = 2,        // Dynamic linking info.
    Interp = 3,         // Name of dynamic loader.
    Note = 4,           // Auxiliary info.
    Shlib = 5,          // Reserved.
    Phdr = 6,           // Program header table.
    Stack = 0x6474e551, // Stack segment.
}

/* Various fields to ensure the ELF file is valid and compatible with the system's expectations. */
#[derive(Debug)]
pub enum ElfError {
    InvalidMagicNumber,
    UnsupportedClass,
    UnsupportedEndianess,
    UnsupportedVersion,
    UnsupportedType,
    UnsupportedMachine,
    SegmentError(ElfSegmentError),
    // Additional error types as needed
}

// Error types that will arise when we try to validate segment
#[derive(Debug)]
pub enum ElfSegmentError {
    DifferentAlignment,
    ContentsOutOfRange,
    MemSizeLesserThanFileSize,
    VMRegionOutOfRange,
    VmRegionWrapAround,
    PageZeroMapping,
    // Additional error types as needed
}

// Flags for p_flags
const PF_X: u32 = 1; // Executable.
const PF_W: u32 = 2; // Writable.
const PF_R: u32 = 4; // Readable.

const ELF_MAGIC_NUMBER: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/* The value of PHYS_BASE determines the boundary between user and kernel space.
With a PHYS_BASE of 4096, any address from 0 to 4095 would be considered user space,
and addresses from 4096 and above would be considered kernel space. */
const PHYS_BASE: *const () = OFFSET as *const ();

/* For p_align, this member gives the value to which the
segments are aligned in memory and in the file. Values 0 and 1 mean that no
alignment is required. */
const MIN_ALIGNMENT: u32 = 1;

// Function to verify ELF header
fn verify_elf_header(header: &Elf32Ehdr) -> Result<(), ElfError> {
    // Check magic number
    if header.e_ident[..4] != ELF_MAGIC_NUMBER {
        return Err(ElfError::InvalidMagicNumber);
    }

    // Check ELF class (e_ident[4]), 1 for 32-bit
    if header.e_ident[4] != 1 {
        return Err(ElfError::UnsupportedClass);
    }

    // Check data encoding (e_ident[5]), assuming 1 for little endian, 2 for big endian
    if header.e_ident[5] != 1 {
        // Adjust according to your target architecture
        return Err(ElfError::UnsupportedEndianess);
    }

    // Check ELF version (e_ident[6]), must be 1 for original ELF version
    if header.e_ident[6] != 1 {
        return Err(ElfError::UnsupportedVersion);
    }

    // Check ELF type (e_type), assuming 2 for executable
    if header.e_type != 2 {
        return Err(ElfError::UnsupportedType);
    }

    // Check machine type (e_machine)
    // 3 for x86
    if header.e_machine != 3 {
        return Err(ElfError::UnsupportedMachine);
    }

    Ok(())
}

/* This function takes a pointer to a constant void type, and returns a boolean indicating
whether or not the given address is within the kernel's address space. */
#[allow(unused)]
fn is_kernel_vaddr(vaddr: *const ()) -> bool {
    unsafe { vaddr >= PHYS_BASE }
}

/* This function takes a pointer to a constant void type, and returns a boolean indicating
whether or not the given address is within the user's address space. */
#[allow(unused)]
fn is_user_vaddr(vaddr: *const ()) -> bool {
    unsafe { vaddr < PHYS_BASE }
}

/* Checks whether phdr describes a valid, loadable segment in
FILE and returns ok if so, ElfSegmentError otherwise. */
fn validate_segment(phdr: &Elf32Phdr, file_data: &[u8]) -> Result<(), ElfSegmentError> {
    // Check alignment constraints.
    // If p_align is NO_ALIGNMENT or MIN_ALIGNMENT, no alignment check is needed.
    // Otherwise, p_offset and p_vaddr should be congruent modulo p_align. Check section 2-2 of ELF spec.
    if (phdr.p_align > MIN_ALIGNMENT)
        && (phdr.p_offset % phdr.p_align != phdr.p_vaddr % phdr.p_align)
    {
        return Err(ElfSegmentError::DifferentAlignment);
    }

    // The range must point within FILE.
    if (phdr
        .p_offset
        .checked_add(phdr.p_filesz)
        .ok_or(ElfSegmentError::ContentsOutOfRange)? as usize)
        > file_data.len()
    {
        return Err(ElfSegmentError::ContentsOutOfRange);
    }

    // p_memsz must be at least as big as p_filesz.
    if phdr.p_memsz < phdr.p_filesz {
        return Err(ElfSegmentError::MemSizeLesserThanFileSize);
    }

    // The virtual memory region must both start and end within the
    // user address space range.
    if !is_user_vaddr(phdr.p_vaddr as *const ()) {
        return Err(ElfSegmentError::VMRegionOutOfRange);
    }

    // The region cannot "wrap around" across the kernel virtual
    // address space.
    if !is_user_vaddr(
        phdr.p_vaddr
            .checked_add(phdr.p_memsz)
            .ok_or(ElfSegmentError::VmRegionWrapAround)?
            .saturating_sub(1) as *const (),
    ) {
        return Err(ElfSegmentError::VMRegionOutOfRange);
    }

    // Disallow mapping page 0.
    if (phdr.p_vaddr as usize) < PAGE_FRAME_SIZE {
        return Err(ElfSegmentError::PageZeroMapping);
    }

    // It's okay.
    Ok(())
}

// Main function to load the ELF binary
pub fn parse_elf(elf_data: &[u8]) -> Result<(u32, Vec<VmAreaStruct>), ElfError> {
    let header = unsafe { &*(elf_data.as_ptr() as *const Elf32Ehdr) };
    let mut vm_areas = Vec::new();

    // Verify ELF header
    verify_elf_header(header)?;

    // Iterate over program headers
    let ph_offset = header.e_phoff as usize;
    let ph_size = header.e_phentsize as usize;
    for i in 0..header.e_phnum as usize {
        let ph = unsafe {
            &*elf_data
                .as_ptr()
                .add(ph_offset + i * ph_size)
                .cast::<Elf32Phdr>()
        };

        if ph.p_type == SegmentType::Load as u32 {
            validate_segment(ph, elf_data).map_err(ElfError::SegmentError)?;
            // The segment must not be empty, if yes, skip this segment.
            if ph.p_memsz == 0 {
                continue;
            }
            let vm_start = ph.p_vaddr as usize;
            let vm_end = vm_start + ph.p_memsz as usize;
            let offset = ph.p_offset as usize;

            let flags: VmFlags = Default::default();
            let mut vma = VmAreaStruct::new(vm_start, vm_end, offset, flags);
            // Set flags based on program header flags
            vma.flags.read = ph.p_flags & PF_R != 0;
            vma.flags.write = ph.p_flags & PF_W != 0;
            vma.flags.execute = ph.p_flags & PF_X != 0;
            vm_areas.push(vma);
        }
    }

    Ok((header.e_entry, vm_areas))
}

#[allow(unused)]
// How we would load elf
fn open_file_and_load() {
    // let elf_data: &'static [u8] = include_bytes!("path/to/your/elf/file");;
    // load_elf(elf_data);
}
