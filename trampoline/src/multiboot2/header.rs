// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#OS-image-format

use core::mem::size_of;

#[allow(unused)]
#[repr(align(64))]
struct Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    end_tag: HeaderTag,
}

#[allow(unused)]
#[repr(align(64))]
struct HeaderTag {
    r#type: u16,
    flags: u16,
    size: u32,
}

const MAGIC: u32 = 0xE85250D6;
const ARCHITECTURE: u32 = 0;
const HEADER_LENGTH: u32 = size_of::<Header>() as u32;

#[used]
#[link_section = ".multiboot2_header"]
static HEADER: Header = Header {
    magic: MAGIC,
    architecture: ARCHITECTURE,
    header_length: HEADER_LENGTH,
    checksum: (MAGIC.wrapping_add(ARCHITECTURE).wrapping_add(HEADER_LENGTH)).wrapping_neg(),
    end_tag: HeaderTag {
        r#type: 0,
        flags: 0,
        size: size_of::<HeaderTag>() as u32,
    },
};
