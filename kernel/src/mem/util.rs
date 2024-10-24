use crate::paging::{is_userspace_readable, is_userspace_writeable};

pub enum CStrError {
    Fault,
    BadUtf8,
}

/// Minimum possible page size. It's okay for this to be smaller than the actual page size.
const MIN_PAGE_SIZE: usize = 4096;

#[must_use]
fn can_access_address_range<T>(start: *const T, bytes: usize, write: bool) -> bool {
    let check = if write {
        is_userspace_writeable
    } else {
        is_userspace_readable
    };
    if !check(start) {
        return false;
    }
    let Some(end) = (start as usize).checked_add(bytes) else {
        // addition overflows so this definitely isn't valid
        return false;
    };
    // round up to nearest page
    let mut p = (start as usize).div_ceil(MIN_PAGE_SIZE) * MIN_PAGE_SIZE;
    while p + MIN_PAGE_SIZE < end {
        if !check(p as *const T) {
            return false;
        }
        p += MIN_PAGE_SIZE;
    }
    true
}

/// Construct null-terminated string from userspace pointer
///
/// # Safety
///
/// The returned reference is invalidated if the page(s) containing the string are mapped out of memory.
/// You must not hold any mutable references to any parts of the string
/// while it is in scope (as is required by Rust).
/// TODO: this doesn't actually check that ptr is mapped yet.
pub unsafe fn get_cstr_from_user_space(ptr: *const u8) -> Result<&'static str, CStrError> {
    let mut len = 0usize;
    if !is_userspace_readable(ptr) {
        return Err(CStrError::Fault);
    }
    loop {
        if *ptr.add(len) == 0 {
            break;
        }
        len += 1;
        let Some(end) = (ptr as usize).checked_add(len) else {
            return Err(CStrError::Fault);
        };
        if end % MIN_PAGE_SIZE == 0 && !is_userspace_readable(end as *const u8) {
            return Err(CStrError::Fault);
        }
    }
    let slice: &'static [u8] = core::slice::from_raw_parts(ptr, len);
    core::str::from_utf8(slice).map_err(|_| CStrError::BadUtf8)
}

/// Construct mutable slice from userspace pointer
///
/// Returns `None` if the pointer is not writeable for the given count of items of type `T`, or if it's not aligned to type `T`.
///
/// # Safety
///
/// The returned reference is invalidated if the page(s) containing the slice are mapped out of memory.
/// You must not hold any other mutable references to any parts of the slice
/// while it is in scope (as is required by Rust).
/// Additionally, `ptr[..count]` must only contain valid values for `T` (e.g. it can't contain the byte `2` if `T` is `bool`).
///
/// TODO: this doesn't actually check that ptr is mapped yet.
pub unsafe fn get_mut_slice_from_user_space<T>(
    ptr: *mut T,
    count: usize,
) -> Option<&'static mut [T]> {
    if !ptr.is_aligned() {
        return None;
    }
    let bytes = count.checked_mul(core::mem::size_of::<T>())?;
    if !can_access_address_range(ptr, bytes, true) {
        return None;
    }
    Some(core::slice::from_raw_parts_mut(ptr.cast(), count))
}

/// Construct slice from userspace pointer
///
/// Returns `None` if the pointer is not readable for the given count of items of type `T`, or if it's not aligned to type `T`.
///
/// # Safety
///
/// The returned reference is invalidated if the page(s) containing the slice are mapped out of memory.
/// You must not hold any mutable references to any parts of the slice
/// while it is in scope (as is required by Rust).
/// Additionally, `ptr[..count]` must only contain valid values for `T` (e.g. it can't contain the byte `2` if `T` is `bool`).
///
/// TODO: this doesn't actually check that ptr is mapped yet.
pub unsafe fn get_slice_from_user_space<T>(ptr: *const T, count: usize) -> Option<&'static [T]> {
    if !ptr.is_aligned() {
        return None;
    }
    let bytes = count.checked_mul(core::mem::size_of::<T>())?;
    if !can_access_address_range(ptr, bytes, false) {
        return None;
    }
    let ptr: *const T = ptr.cast();
    Some(core::slice::from_raw_parts(ptr, count))
}

/// Construct mutable reference from userspace pointer.
///
/// Returns `None` if the pointer is not writeable, or if it's not aligned to type `T`.
///
/// # Safety
///
/// See [`get_mut_slice_from_user_space`].
pub unsafe fn get_mut_from_user_space<T>(ptr: *mut T) -> Option<&'static mut T> {
    Some(&mut get_mut_slice_from_user_space(ptr, 1)?[0])
}

/// Construct reference from userspace pointer.
///
/// Returns `None` if the pointer is not readable, or if it's not aligned to type `T`.
///
/// # Safety
///
/// See [`get_slice_from_user_space`].
pub unsafe fn get_ref_from_user_space<T>(ptr: *const T) -> Option<&'static T> {
    Some(&get_slice_from_user_space(ptr, 1)?[0])
}
