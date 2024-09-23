use nom::bytes::complete::{tag, take};
use nom::combinator::{map_opt, verify};
use nom::error::Error;
use nom::number::complete::{u16, u32, u8};
use nom::number::Endianness;
use nom::IResult;

use alloc::vec::Vec;

// Endianness of the fields within the Elf file.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ElfEndianness {
    Little,
    Big,
}

impl ElfEndianness {
    fn to_nom(self) -> Endianness {
        match self {
            ElfEndianness::Little => Endianness::Little,
            ElfEndianness::Big => Endianness::Big,
        }
    }
}

// Intended purpose for the ELF file.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ElfUsage {
    // Static Library
    Relocatable,
    // Executable
    Executable,
    // Dynamically Linked Library
    Shared,
    // Core Dump (from a crash)
    Core,
}

// Common Architectures from https://wiki.osdev.org/ELF
// Our host architecture is x86.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ElfArchitecture {
    // No architecture specified.
    Generic,

    Sparc,
    Mips,
    PowerPC,
    RiscV,
    Ia64,
    X86,
    X8664,
    Arm,
    Arm64,
}

// Strictly 32-bit ELFs.
#[derive(Copy, Clone, Debug)]
pub struct ElfHeader {
    pub endianness: ElfEndianness,
    pub header_version: u8,
    pub abi: u8,
    pub usage: ElfUsage,
    pub architecture: ElfArchitecture,
    pub elf_version: u32,
    pub program_entry: u32,
    pub program_headers_offset: u32,
    pub section_headers_offset: u32,
    pub flags: u32,
    pub elf_header_size: u16,
    pub program_header_entry_size: u16,
    pub program_header_count: u16,
    pub section_header_entry_size: u16,
    pub section_header_count: u16,
    pub section_header_index: u16,
}

impl ElfHeader {
    pub fn parse(bytes: &[u8]) -> IResult<&[u8], ElfHeader> {
        let (bytes, _) = tag([0x7F, b'E', b'L', b'F'])(bytes)?;

        // Elf Bit Width, we don't parse 64-bits ELF binaries.
        let (bytes, _) = tag([1])(bytes)?;

        // Endianness seems to reflect the endianness of the fields, not only the architecture
        let (bytes, endianness) = map_opt(u8, |value| match value {
            1 => Some(ElfEndianness::Little),
            2 => Some(ElfEndianness::Big),
            _ => None,
        })(bytes)?;

        let endian = endianness.to_nom();

        let (bytes, header_version) = u8(bytes)?;
        let (bytes, abi) = u8(bytes)?;

        let (bytes, _) = take(8usize)(bytes)?;

        let (bytes, usage) = map_opt(u16(endian), |value| match value {
            1 => Some(ElfUsage::Relocatable),
            2 => Some(ElfUsage::Executable),
            3 => Some(ElfUsage::Shared),
            4 => Some(ElfUsage::Core),
            _ => None,
        })(bytes)?;

        let (bytes, architecture) = map_opt(u16(endian), |value| match value {
            0x00 => Some(ElfArchitecture::Generic),
            0x02 => Some(ElfArchitecture::Sparc),
            0x03 => Some(ElfArchitecture::X86),
            0x08 => Some(ElfArchitecture::Mips),
            0x14 => Some(ElfArchitecture::PowerPC),
            0x28 => Some(ElfArchitecture::Arm),
            0x32 => Some(ElfArchitecture::Ia64),
            0x3E => Some(ElfArchitecture::X8664),
            0xB7 => Some(ElfArchitecture::Arm64),
            0xF3 => Some(ElfArchitecture::RiscV),
            _ => None, // Could be Some(ElfArchitecture::Generic)
        })(bytes)?;

        let (bytes, elf_version) = u32(endian)(bytes)?;
        let (bytes, program_entry) = u32(endian)(bytes)?;
        let (bytes, program_headers_offset) = u32(endian)(bytes)?;
        let (bytes, section_headers_offset) = u32(endian)(bytes)?;

        let (bytes, flags) = u32(endian)(bytes)?;

        let (bytes, elf_header_size) = u16(endian)(bytes)?;
        let (bytes, program_header_entry_size) = u16(endian)(bytes)?;
        let (bytes, program_header_count) = u16(endian)(bytes)?;
        let (bytes, section_header_entry_size) = u16(endian)(bytes)?;
        let (bytes, section_header_count) = u16(endian)(bytes)?;
        let (bytes, section_header_index) = u16(endian)(bytes)?;

        Ok((
            bytes,
            ElfHeader {
                endianness,
                header_version,
                abi,
                usage,
                architecture,
                elf_version,
                program_entry,
                program_headers_offset,
                section_headers_offset,
                flags,
                elf_header_size,
                program_header_entry_size,
                program_header_count,
                section_header_entry_size,
                section_header_count,
                section_header_index,
            },
        ))
    }
}

// Intended purpose for this program segment type.
// We only really care about the "Load" type, but the other types can carry useful information.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ElfProgramType {
    Ignore,
    Load,
    Dynamic,
    Interpret,
    Note,
    OsSpecific(u32),
}

#[derive(Copy, Clone, Debug)]
pub struct ElfProgramHeader<'a> {
    pub program_type: ElfProgramType,

    // Offset of the program data within the file.
    pub file_offset: u32,

    // Mount address, virtual address.
    pub virtual_address: u32,

    // Typically ignored, unless our operating system cares about physical addresses.
    // I find this is typically equal to the virtual_address field.
    pub physical_address: u32,

    // Content of the program section.
    pub data: &'a [u8], // avoid copying large sections of the ELF

    // Size that needs to be allocated, should be strictly greater than data.
    pub memory_size: u32,

    // Memory access flags.
    pub executable: bool,
    pub writable: bool,
    pub readable: bool,

    pub alignment: u32,
}

impl<'a> ElfProgramHeader<'a> {
    // Parses a Program Header for a segment.
    //  `bytes` is a slice pointing to the start of the ProgramHeader.
    //  `endian` refers to the Endianness of the fields within the header (from the elf header).
    //  `full_file` is a slice that contains the whole file. Used to grab a slice to segment data.
    pub fn parse(
        bytes: &'a [u8],
        endian: Endianness,
        full_file: &'a [u8],
    ) -> IResult<&'a [u8], Self> {
        let (bytes, program_type) = map_opt(u32(endian), |value| match value {
            0 => Some(ElfProgramType::Ignore),
            1 => Some(ElfProgramType::Load),
            2 => Some(ElfProgramType::Dynamic),
            3 => Some(ElfProgramType::Interpret),
            4 => Some(ElfProgramType::Note),
            0x60000000.. => Some(ElfProgramType::OsSpecific(value)), // OS Specific Headers
            _ => None,
        })(bytes)?;

        let (bytes, file_offset) = u32(endian)(bytes)?;
        let (bytes, virtual_address) = u32(endian)(bytes)?;
        let (bytes, physical_address) = u32(endian)(bytes)?;
        let (bytes, file_size) = u32(endian)(bytes)?;
        // Protection, to make sure we map enough memory to contain the file data.
        let (bytes, memory_size) = verify(u32(endian), |size| file_size <= *size)(bytes)?;
        let (bytes, flags) = u32(endian)(bytes)?;
        let (bytes, alignment) = u32(endian)(bytes)?;

        let executable = flags & 1 != 0;
        let writable = flags & 2 != 0;
        let readable = flags & 4 != 0;

        let (data_start, _) = take(file_offset)(full_file)?;
        let (_, data) = take(file_size)(data_start)?;

        Ok((
            bytes,
            ElfProgramHeader {
                program_type,
                file_offset,
                virtual_address,
                physical_address,
                data,
                memory_size,
                executable,
                writable,
                readable,
                alignment,
            },
        ))
    }
}

#[derive(Clone, Debug)]
pub struct Elf<'a> {
    // Contains elf metadata.
    pub header: ElfHeader,

    // And program segments.
    pub program_headers: Vec<ElfProgramHeader<'a>>,
    // Sections (used for linking) are ignored in this parse method.
}

impl<'a> Elf<'a> {
    pub fn parse(full_bytes: &'a [u8]) -> IResult<&'a [u8], Elf<'a>> {
        let (bytes, header) = ElfHeader::parse(full_bytes)?;

        let (mut program_header_bytes, _) = take(header.program_headers_offset)(full_bytes)?;

        let mut program_headers = Vec::with_capacity(header.program_header_count as usize);

        for _ in 0..header.program_header_count {
            let (_, program_header) = ElfProgramHeader::parse(
                program_header_bytes,
                header.endianness.to_nom(),
                full_bytes,
            )?;

            program_headers.push(program_header);

            (program_header_bytes, _) =
                take(header.program_header_entry_size)(program_header_bytes)?;
        }

        Ok((
            bytes,
            Elf {
                header,
                program_headers,
            },
        ))
    }

    pub fn parse_bytes(bytes: &'a [u8]) -> Result<Elf<'a>, nom::Err<Error<&'a [u8]>>> {
        Ok(Self::parse(bytes)?.1)
    }
}
