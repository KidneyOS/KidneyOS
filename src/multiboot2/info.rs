// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

use core::{
    ffi::{c_char, CStr},
    mem::size_of,
    ptr::from_ref,
};

#[repr(C)]
pub struct Info {
    pub total_size: u32,
    pub reserved: u32,
}

const END_TYPE: u32 = 0;
const COMMANDLINE_TYPE: u32 = 1;
const BOOT_LOADER_NAME_TYPE: u32 = 2;
const BASIC_MEMORY_INFO_TYPE: u32 = 4;

#[allow(dead_code)]
#[repr(u32)]
#[repr(C)]
pub enum InfoTag {
    Commandline(CommandlineTag) = COMMANDLINE_TYPE,
    BootLoaderName(BootLoaderNameTag) = BOOT_LOADER_NAME_TYPE,
    BasicMemoryInfo(BasicMemoryInfoTag) = BASIC_MEMORY_INFO_TYPE,
}

// NOTE: We can't properly represent InfoTag's native structure as a Rust type
// because Rust doesn't support unsized enum variant fields, which prevents us
// from describing tags that end with variable-sized data such as strings. See:
// https://github.com/rust-lang/rfcs/issues/1151. This is why the impls below
// are necessary.

#[repr(C)]
pub struct CommandlineTag {
    _size: u32,
    commandline_start: c_char,
}

impl From<&CommandlineTag> for &CStr {
    fn from(val: &CommandlineTag) -> Self {
        // SAFETY: multiboot guarantees that commandline tags will contain a
        // valid C string.
        unsafe { CStr::from_ptr(from_ref(val).cast::<u32>().offset(1).cast::<c_char>()) }
    }
}

#[repr(C)]
pub struct BootLoaderNameTag {
    _size: u32,
    boot_loader_name_start: c_char,
}

impl From<&BootLoaderNameTag> for &CStr {
    fn from(val: &BootLoaderNameTag) -> Self {
        // SAFETY: multiboot guarantees that boot loader name tags will contain
        // a valid C string.
        unsafe { CStr::from_ptr(from_ref(val).cast::<u32>().offset(1).cast::<c_char>()) }
    }
}

#[repr(C)]
pub struct BasicMemoryInfoTag {
    _size: u32,
    pub mem_lower: u32,
    pub mem_upper: u32,
}

#[repr(C)]
struct Headers {
    r#type: u32,
    size: u32,
}

impl Info {
    pub const fn iter(&self) -> InfoIterator {
        InfoIterator {
            info: self,
            #[allow(clippy::cast_possible_truncation)]
            offset: size_of::<Self>() as u32,
        }
    }
}

pub struct InfoIterator<'a> {
    info: &'a Info,
    offset: u32,
}

impl<'a> InfoIterator<'a> {
    pub const unsafe fn curr_ptr(&self) -> *const u8 {
        from_ref(self.info).cast::<u8>().add(self.offset as usize)
    }

    const fn curr_headers(&self) -> &Headers {
        // SAFETY: The return value of curr_ptr depends on Info, which is
        // guaranteed by multiboot to have an alignment of 64, as well as on
        // offset, which is guaranteed by both multiboot and by our checks to be
        // a multiple of 8, meaning the result of curr_ptr is guaranteed to have
        // an alignment of at least 64, which is greater than what is required
        // by Headers.
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            &*self.curr_ptr().cast::<Headers>()
        }
    }
}

impl<'a> Iterator for InfoIterator<'a> {
    type Item = &'a InfoTag;

    fn next(&mut self) -> Option<Self::Item> {
        let curr_headers = self.curr_headers();
        let curr = match curr_headers.r#type {
            END_TYPE => return None,
            COMMANDLINE_TYPE | BOOT_LOADER_NAME_TYPE | BASIC_MEMORY_INFO_TYPE => {
                // SAFETY: Same as curr_headers.
                unsafe {
                    #[allow(clippy::cast_ptr_alignment)]
                    &*self.curr_ptr().cast::<InfoTag>()
                }
            }
            _ => {
                // It is UB to cast this to a variant since its discriminant is
                // not in the type definition for InfoTag, so we skip it.
                self.offset += curr_headers.size;
                self.offset = self.offset.next_multiple_of(8);
                return self.next();
            }
        };
        self.offset += curr_headers.size;
        self.offset = self.offset.next_multiple_of(8);
        Some(curr)
    }
}
