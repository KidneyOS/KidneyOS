use core::ffi::c_char;
use core::slice::from_raw_parts;

// Null Terminated list of Null Terminated Strings
pub type RawArguments = *const *const c_char;

// SAFETY: `raw` must be valid. Null is acceptable, and will return zero.
// SAFETY: `raw` must be null-terminated.
pub unsafe fn count_arguments(raw: RawArguments) -> usize {
    if raw.is_null() {
        return 0
    }

    let mut length = 0;

    while (*raw.add(length)).is_null() {
        length += 1;
    }

    length
}

// SAFETY: `raw` must be valid. Null is acceptable, and will return an empty slice.
// SAFETY: `raw` must be null-terminated.
pub unsafe fn argument_slice_from_raw(raw: RawArguments) -> &'static [*const c_char] {
    let length = count_arguments(raw);
    
    Default::default()

    // if length != 0 {
    //     from_raw_parts(raw, length)
    // } else {
    //     Default::default()
    // }
}
