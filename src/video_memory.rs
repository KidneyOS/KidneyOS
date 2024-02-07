use core::{fmt, slice};

pub const VIDEO_MEMORY_BASE: usize = 0xb8000;
const VIDEO_MEMORY_COLS: usize = 80;
const VIDEO_MEMORY_LINES: usize = 25;
pub const VIDEO_MEMORY_SIZE: usize = VIDEO_MEMORY_COLS * VIDEO_MEMORY_LINES;

pub struct VideoMemoryWriter {
    // TODO: Actually move cursor visually.
    cursor: usize,
    pub attribute: Attribute,
}

#[allow(dead_code)]
pub enum Colour {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Purple = 5,
    Brown = 6,
    Gray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightPurple = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct Attribute {
    #[allow(dead_code)]
    inner: u8,
}

impl Attribute {
    pub const fn new(fg: Colour, bg: Colour) -> Self {
        const MASK_3: u8 = (1 << 3) - 1;
        Self {
            inner: (((bg as u8) & MASK_3) << 4) | (fg as u8),
        }
    }
}

impl fmt::Write for VideoMemoryWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        #[allow(dead_code)]
        #[repr(packed)]
        #[derive(Clone, Copy)]
        struct Character {
            ascii: u8,
            attribute: Attribute,
        }

        // SAFETY: Assumes that there is only one core => multiple threads
        // cannot be inside this function at once holding video_memory.
        let video_memory = unsafe {
            slice::from_raw_parts_mut(VIDEO_MEMORY_BASE as *mut Character, VIDEO_MEMORY_SIZE)
        };

        for b in s.as_bytes() {
            if self.cursor >= video_memory.len() {
                video_memory.copy_within(VIDEO_MEMORY_COLS..VIDEO_MEMORY_SIZE, 0);
                self.cursor = VIDEO_MEMORY_SIZE - VIDEO_MEMORY_COLS;
            }

            if *b == b'\n' {
                self.cursor = self.cursor.next_multiple_of(VIDEO_MEMORY_COLS);
                continue;
            }

            video_memory[self.cursor] = Character {
                ascii: *b,
                attribute: self.attribute,
            };
            self.cursor += 1;
        }

        Ok(())
    }
}

pub static mut VIDEO_MEMORY_WRITER: VideoMemoryWriter = VideoMemoryWriter {
    cursor: 0,
    attribute: Attribute::new(Colour::White, Colour::Black),
};
