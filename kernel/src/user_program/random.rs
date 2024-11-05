use core::arch::asm;

/// Generates a random int using the CPU's RDRAND instruction.
fn generate_random_i32() -> Option<i32> {
    let mut random_int: i32;
    let success: u8;

    unsafe {
        asm!(
            "rdrand {0}",
            "setc {1}",
            out(reg) random_int,
            out(reg_byte) success,
            options(nostack, nomem),
        );
    }
    
    (success == 1).then_some(random_int)
}

/// Fills a buffer with random bytes from the CPU's RDRAND instruction.
/// Returns the number of bytes written, or -1 if an error occurs.
/// Currently no flags are implemented, if there is no random data available,
/// the random data generated so far is returned.
pub fn getrandom(buffer: &mut [u8], length: usize, _flags: usize) -> isize {
    let mut bytes_written: usize = 0;
    let chunks = length / 4;
    let remainder = length % 4;

    for i in 0..chunks {
        match generate_random_i32() {
            Some(random_int) => {
                let random_bytes = random_int.to_le_bytes();
                buffer[i * 4..i * 4 + 4].copy_from_slice(&random_bytes);
            }
            None => return bytes_written.try_into().unwrap(),
        }
    }

    // Handle any remaining bytes if length is not a multiple of 4
    if remainder > 0 {
        if let Some(random_int) = generate_random_i32() {
            let random_bytes = random_int.to_le_bytes();
            for (i, &byte) in random_bytes.iter().enumerate().take(remainder) {
                buffer[chunks * 4 + i] = byte;
            }
            bytes_written += remainder;
        }
    }

    bytes_written.try_into().unwrap()
}
