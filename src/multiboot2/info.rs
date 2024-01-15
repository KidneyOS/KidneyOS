// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format

use core::{ffi::c_char, mem::size_of};

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

// TODO: Use &CStr instead of *const c_char below once linking issues are fixed.

#[repr(C)]
pub struct CommandlineTag {
    _size: u32,
    commandline_start: c_char,
}

impl From<&CommandlineTag> for *const c_char {
    fn from(val: &CommandlineTag) -> Self {
        unsafe { (val as *const _ as *const u32).offset(1) as *const _ as *const c_char }
    }
}

#[repr(C)]
pub struct BootLoaderNameTag {
    _size: u32,
    boot_loader_name_start: c_char,
}

impl From<&BootLoaderNameTag> for *const c_char {
    fn from(val: &BootLoaderNameTag) -> Self {
        unsafe { (val as *const _ as *const u32).offset(1) as *const _ as *const c_char }
    }
}

#[repr(C)]
pub struct BasicMemoryInfoTag {
    _size: u32,
    pub mem_lower: u32,
    pub mem_upper: u32,
}

#[repr(C)]
pub struct Headers {
    pub r#type: u32,
    size: u32,
}

impl Info {
    pub fn iter(&self) -> InfoIterator {
        InfoIterator {
            info: self,
            offset: size_of::<Info>() as u32,
        }
    }
}

#[repr(C)]
pub struct InfoIterator<'a> {
    info: &'a Info,
    pub offset: u32,
}

impl<'a> InfoIterator<'a> {
    pub unsafe fn curr_ptr(&self) -> *const u8 {
        (self.info as *const _ as *const u8).offset(self.offset as isize)
    }

    fn curr_headers(&self) -> &Headers {
        unsafe { &*(self.curr_ptr() as *const Headers) }
    }
}

impl<'a> Iterator for InfoIterator<'a> {
    type Item = &'a InfoTag;

    fn next(&mut self) -> Option<Self::Item> {
        let curr_headers = self.curr_headers();
        let curr = match curr_headers.r#type {
            END_TYPE => return None,
            COMMANDLINE_TYPE | BOOT_LOADER_NAME_TYPE | BASIC_MEMORY_INFO_TYPE => unsafe {
                &*(self.curr_ptr() as *const _ as *const InfoTag)
            },
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
