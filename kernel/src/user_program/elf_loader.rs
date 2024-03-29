use super::virtual_memory_area::{VmAreaStruct, VmFlags};
use alloc::vec::Vec;

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

#[derive(Debug)]
pub enum ElfError {
    InvalidMagicNumber,
    UnsupportedClass,
    UnsupportedEndianess,
    UnsupportedVersion,
    UnsupportedType,
    UnsupportedMachine,
    // Additional error types as needed
}

// Flags for p_flags
const PF_X: u32 = 1; // Executable.
const PF_W: u32 = 2; // Writable.
const PF_R: u32 = 4; // Readable.

const ELF_MAGIC_NUMBER: [u8; 4] = [0x7F, b'E', b'L', b'F'];

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

// Main function to load the ELF binary
#[allow(unused)]
fn load_elf(elf_data: &[u8]) -> Result<(), ElfError> {
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
            let vm_start = ph.p_vaddr as usize;
            let vm_end = vm_start + ph.p_memsz as usize;

            let flags: VmFlags = Default::default();
            let mut vma = VmAreaStruct::new(vm_start, vm_end, flags);
            // Set flags based on program header flags
            vma.flags.read = ph.p_flags & PF_R != 0;
            vma.flags.write = ph.p_flags & PF_W != 0;
            vma.flags.execute = ph.p_flags & PF_X != 0;
            vm_areas.push(vma);
            // Here we would load the segment into memory, copy from `elf_data[ph.p_offset as usize..]` to `ph.p_vaddr` address in memory
        }
    }

    Ok(())
}

#[allow(unused)]
// How we would load elf
fn open_file_and_load() {
    // let elf_data: &'static [u8] = include_bytes!("path/to/your/elf/file");;
    // load_elf(elf_data);
}
