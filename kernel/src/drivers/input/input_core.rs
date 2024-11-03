use core::fmt::Display;

const BUFFER_SIZE: usize = 256;

/// A circular buffer for storing input from the PS/2 controller.
pub struct InputBuffer {
    // TODO: CV for not full and not empty?
    /// The buffer itself.
    buf: [u8; BUFFER_SIZE],
    /// The index of the head of the buffer.
    head: usize,
    /// The index of the tail of the buffer.
    tail: usize,
}

#[allow(unused)]
impl InputBuffer {
    /// Create a new, empty input buffer.
    pub const fn new() -> InputBuffer {
        InputBuffer {
            buf: [0; BUFFER_SIZE],
            head: 0,
            tail: 0,
        }
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    /// Add a byte to the buffer.
    pub fn putc(&mut self, c: u8) {
        // TODO: wait while buffer is full

        self.buf[self.head] = c;
        self.head = (self.head + 1) % BUFFER_SIZE;
    }

    /// Get a byte from the buffer.
    pub fn getc(&mut self) -> Option<u8> {
        if self.head == self.tail {
            None
        } else {
            let c = self.buf[self.tail];
            self.tail = (self.tail + 1) % BUFFER_SIZE;
            Some(c)
        }
    }
}

impl Display for InputBuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for i in self.tail..self.head {
            write!(f, "{}", self.buf[i % BUFFER_SIZE] as char)?;
        }
        Ok(())
    }
}
