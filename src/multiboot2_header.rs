use core::mem::size_of;

// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#OS-image-format

#[allow(unused)]
#[repr(align(8))]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    end_tag: Multiboot2HeaderTag,
}

#[allow(unused)]
#[repr(align(8))]
struct Multiboot2HeaderTag {
    r#type: u16,
    flags: u16,
    size: u32,
}

const MAGIC: u32 = 0xE85250D6;
const ARCHITECTURE: u32 = 0;
const HEADER_LENGTH: u32 = size_of::<Multiboot2Header>() as u32;

#[used]
#[link_section = ".multiboot2_header"]
static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MAGIC,
    architecture: ARCHITECTURE,
    header_length: HEADER_LENGTH,
    checksum: (MAGIC.wrapping_add(ARCHITECTURE).wrapping_add(HEADER_LENGTH)).wrapping_neg(),
    end_tag: Multiboot2HeaderTag {
        r#type: 0,
        flags: 0,
        size: size_of::<Multiboot2HeaderTag>() as u32,
    },
};
