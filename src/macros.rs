#![allow(unused_macros)]

// NOTE: VIDEO_MEMORY_WRITER should be written to before SERIAL_WRITER since the
// second may fail if the serial chip misbehaves there, but VIDEO_MEMORY_WRITER
// will always succeed.

// TODO: Modify uses of writers once interrupts are enabled, since we have to
// make sure they don't get left in an inconsistent state if we get interrupted
// in the middle of a print.

macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        // SAFETY: Single core, no interrupts.
        unsafe {
            write!($crate::video_memory::VIDEO_MEMORY_WRITER, "{}", format_args!($($arg)*)).unwrap();
            write!($crate::serial::SERIAL_WRITER, "{}", format_args!($($arg)*)).unwrap();
        }
    }};
}

macro_rules! println {
    () => {{
        use core::fmt::Write;
        // SAFETY: Single core, no interrupts.
        unsafe {
            write!($crate::video_memory::VIDEO_MEMORY_WRITER, "\n").unwrap();
            write!($crate::serial::SERIAL_WRITER, "\n").unwrap();
        }
    }};
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        use $crate::{serial::SERIAL_WRITER, video_memory::VIDEO_MEMORY_WRITER};
        // SAFETY: Single core, no interrupts.
        unsafe {
            write!(VIDEO_MEMORY_WRITER, "{}", format_args!($($arg)*)).unwrap();
            write!(VIDEO_MEMORY_WRITER, "\n").unwrap();
            write!(SERIAL_WRITER, "{}", format_args!($($arg)*)).unwrap();
            write!(SERIAL_WRITER, "\n").unwrap();
        }
    }};
}

macro_rules! eprint {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        use $crate::video_memory::*;
        // SAFETY: Single core, no interrupts.
        unsafe {
            let prev_attribute = VIDEO_MEMORY_WRITER.attribute;
            VIDEO_MEMORY_WRITER.attribute = Attribute::new(Colour::Red, Colour::Black);
            write!(VIDEO_MEMORY_WRITER, "{}", format_args!($($arg)*)).unwrap();
            VIDEO_MEMORY_WRITER.attribute = prev_attribute;
            write!($crate::serial::SERIAL_WRITER, "{}", format_args!($($arg)*)).unwrap();
        }
    }};
}

macro_rules! eprintln {
    () => {{
        use core::fmt::Write;
        // SAFETY: Single core, no interrupts.
        unsafe {
            write!($crate::video_memory::VIDEO_MEMORY_WRITER, "\n").unwrap();
            write!($crate::serial::SERIAL_WRITER, "\n").unwrap();
        }
    }};
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        use $crate::{serial::SERIAL_WRITER, video_memory::{Attribute, Colour, VIDEO_MEMORY_WRITER}};
        // SAFETY: Single core, no interrupts.
        unsafe {
            let prev_attribute = VIDEO_MEMORY_WRITER.attribute;
            VIDEO_MEMORY_WRITER.attribute = Attribute::new(Colour::Red, Colour::Black);
            write!(VIDEO_MEMORY_WRITER, "{}", format_args!($($arg)*)).unwrap();
            write!(VIDEO_MEMORY_WRITER, "\n").unwrap();
            VIDEO_MEMORY_WRITER.attribute = prev_attribute;
            write!(SERIAL_WRITER, "{}", format_args!($($arg)*)).unwrap();
            write!(SERIAL_WRITER, "\n").unwrap();
        }
    }};
}

macro_rules! bochs_break {
    () => {
        // SAFETY: This is safe to use anywhere since it's a noop. The Bochs
        // emulator will break upon encountering it when magic_break: enabled=1
        // is enabled.
        #[cfg(debug_assertions)]
        unsafe {
            core::arch::asm!("xchg bx, bx")
        }
    };
}
