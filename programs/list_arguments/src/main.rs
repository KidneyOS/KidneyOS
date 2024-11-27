#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use core::cmp::min;
use core::ffi::c_char;
use kidneyos_syscalls::arguments::{argument_slice_from_raw, RawArguments};
use kidneyos_syscalls::STDOUT_FILENO;

struct BufferWriter<const N: usize> {
    cursor: usize,
    buffer: [u8; N]
}

static mut WRITER: BufferWriter<8000> = BufferWriter::new();

impl<const N: usize> BufferWriter<N> {
    const fn new() -> Self {
        Self {
            cursor: 0,
            buffer: [0; N]
        }
    }
    
    fn write(&mut self, value: &[u8]) {
        let remaining = self.buffer.len() - self.cursor;
        
        if remaining == 0 {
            return
        }
        
        let write_bytes = min(value.len(), remaining);
        
        for i in 0 .. write_bytes {
            self.buffer[self.cursor + i] = value[i];
        }

        // Don't use this! Rust will generate a memcpy and we will get a link error.
        // self.buffer[self.cursor .. self.cursor + write_bytes]
        //     .copy_from_slice(&value[..write_bytes]);
        
        self.cursor += write_bytes
    }
    
    fn get(&self) -> &[u8] {
        &self.buffer[..self.cursor]
    }
}

// Why not use CStr::count_bytes?
//  1. It doesn't seem to be stable yet.
//  2. It seems to invoke strlen, which is obviously not available in our no_str environment.
fn count_bytes(str: *const c_char) -> usize {
    if str.is_null() {
        return 0
    }

    let mut length = 0;

    unsafe {
        while *str.add(length) != b'\0' as c_char {
            length += 1;
        }
    }

    length
}

#[no_mangle]
unsafe extern "C" fn _start(raw: RawArguments) {
    let arguments = unsafe { argument_slice_from_raw(raw) };
    
    WRITER.write(b"Argument Count: ");
    
    let len_char = b'0' + (arguments.len() as u8 % 10);
    WRITER.write(&[len_char, b'\n']);
    
    for (i, arg) in arguments.iter().enumerate() {
        WRITER.write(b"Argument ");
        
        // Not going to bother to format numbers.
        let i_char = b'0' + (i as u8 % 10);
        WRITER.write(&[i_char]);
        WRITER.write(b": ");
        
        WRITER.write(core::slice::from_raw_parts((*arg).cast(), count_bytes(*arg)));
    
        WRITER.write(b"\n");
    }

    let writer_out = WRITER.get();
    kidneyos_syscalls::write(STDOUT_FILENO, writer_out.as_ptr(), writer_out.len());
    
    kidneyos_syscalls::exit(0);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
