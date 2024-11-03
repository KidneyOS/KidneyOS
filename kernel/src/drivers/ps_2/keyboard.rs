// https://wiki.osdev.org/%228042%22_PS/2_Controller#PS/2_Controller_IO_Ports

use kidneyos_shared::serial::inb;

/// Data port           Read/Write
///
/// The Data Port (IO Port 0x60) is used for reading data that was received from a PS/2 device or from the PS/2 controller itself and writing data to a PS/2 device or to the PS/2 controller itself.
static DATA_PORT: u16 = 0x60;
/// Status register     Read
static _STATUS_REGISTER: u16 = 0x64;
/// Command register    Write
static _COMMAND_REGISTER: u16 = 0x64;

// Modifier Keys
static mut L_SHIFT: bool = false;
static mut R_SHIFT: bool = false;
static mut L_CTRL: bool = false;
static mut R_CTRL: bool = false;
static mut L_ALT: bool = false;
static mut R_ALT: bool = false;
static mut CAPS_LOCK: bool = false;

struct Keymap {
    first_scancode: u16,
    chars: &'static str,
}

// Scancode to key mappings

/// Keys that produce the same characters regardless of Shift keys. Case of
/// letters is handled separately.
static INVARIANT_KEYMAP: &[Keymap] = &[
    Keymap {
        first_scancode: 0x01,
        chars: "\x1B",
    }, // Escape
    Keymap {
        first_scancode: 0x0e,
        chars: "\x08",
    }, // Backspace
    Keymap {
        first_scancode: 0x0f,
        chars: "\tQWERTYUIOP",
    },
    Keymap {
        first_scancode: 0x1c,
        chars: "\r",
    }, // Enter
    Keymap {
        first_scancode: 0x1e,
        chars: "ASDFGHJKL",
    },
    Keymap {
        first_scancode: 0x2c,
        chars: "ZXCVBNM",
    },
    Keymap {
        first_scancode: 0x37,
        chars: "*",
    },
    Keymap {
        first_scancode: 0x39,
        chars: " ",
    }, // Space
    Keymap {
        first_scancode: 0x53,
        chars: "\x7F",
    }, // Delete
    Keymap {
        first_scancode: 0,
        chars: "",
    },
];

/// Characters for keys pressed without Shift, for those keys where it matters.
static UNSHIFTED_KEYMAP: &[Keymap] = &[
    Keymap {
        first_scancode: 0x02,
        chars: "1234567890-=",
    },
    Keymap {
        first_scancode: 0x1a,
        chars: "[]",
    },
    Keymap {
        first_scancode: 0x27,
        chars: ";'`",
    },
    Keymap {
        first_scancode: 0x2b,
        chars: "\\",
    },
    Keymap {
        first_scancode: 0x33,
        chars: ",./",
    },
    Keymap {
        first_scancode: 0,
        chars: "",
    },
];

/// Characters for keys pressed with Shift, for those keys where it matters.
static SHIFTED_KEYMAP: &[Keymap] = &[
    Keymap {
        first_scancode: 0x02,
        chars: "!@#$%^&*()_+",
    },
    Keymap {
        first_scancode: 0x1a,
        chars: "{}",
    },
    Keymap {
        first_scancode: 0x27,
        chars: ":\"~",
    },
    Keymap {
        first_scancode: 0x2b,
        chars: "|",
    },
    Keymap {
        first_scancode: 0x33,
        chars: "<>?",
    },
    Keymap {
        first_scancode: 0,
        chars: "",
    },
];

pub fn on_keyboard_interrupt() {
    // Modifier keys
    let shift: bool = unsafe { L_SHIFT || R_SHIFT };
    // TODO: Handle ctrl and alt?
    let _ctrl: bool = unsafe { L_CTRL || R_CTRL };
    let _alt: bool = unsafe { L_ALT || R_ALT };

    // Read the scancode
    let mut code = unsafe { inb(DATA_PORT) } as u16;
    if code == 0xe0 {
        // Extended scancode
        code = code << 8 | (unsafe { inb(DATA_PORT) } as u16);
    }

    // > 0x80 means key release
    let release: bool = code & 0x80 != 0;
    code &= 0x7F;

    // Caps Lock
    if code == 0x3A {
        if !release {
            unsafe { CAPS_LOCK = !CAPS_LOCK };
        }
        return;
    }

    // Handle the key
    let c = map_key(INVARIANT_KEYMAP, code)
        .or_else(|| {
            if !shift {
                map_key(UNSHIFTED_KEYMAP, code)
            } else {
                None
            }
        })
        .or_else(|| {
            if shift {
                map_key(SHIFTED_KEYMAP, code)
            } else {
                None
            }
        });

    if let Some(mut c) = c {
        if release {
            // No need to handle key release
            return;
        }

        // Ordinary character
        if shift == unsafe { CAPS_LOCK } {
            c = c.to_ascii_lowercase();
        }
        // TODO: Add to buffer
        kidneyos_shared::eprint!("{}", c as char);
    } else {
        // Modifier keys

        match code {
            0x2A => unsafe {
                L_SHIFT = !release;
            },
            0x36 => unsafe {
                R_SHIFT = !release;
            },
            0x38 => unsafe {
                L_ALT = !release;
            },
            0xE038 => unsafe {
                R_ALT = !release;
            },
            0x1D => unsafe {
                L_CTRL = !release;
            },
            0xE01D => unsafe {
                R_CTRL = !release;
            },
            _ => (),
        }
    }
}

/// Scans the array of keymaps `k` for `scancode`.
fn map_key(k: &[Keymap], scancode: u16) -> Option<u8> {
    for keymap in k {
        if keymap.first_scancode != 0
            && scancode >= keymap.first_scancode
            && scancode < keymap.first_scancode + keymap.chars.len() as u16
        {
            let character = keymap.chars.as_bytes()[(scancode - keymap.first_scancode) as usize];
            return Some(character);
        }
    }
    None
}
