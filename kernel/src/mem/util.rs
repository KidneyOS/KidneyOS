use core::mem::size_of;
use kidneyos_shared::mem::OFFSET as KMEM_OFFSET;
use kidneyos_shared::mem::PAGE_FRAME_SIZE;
use kidneyos_shared::println;

pub enum CStrError {
    Fault,
    BadUtf8,
}

fn can_access_range<T>(start: *const T, count: usize, write: bool) -> bool {
    let start = start as usize;
    let Some(bytes) = count.checked_mul(size_of::<T>()) else {
        return false;
    };
    if start >= KMEM_OFFSET {
        return false;
    }
    let Some(end) = start.checked_add(bytes) else {
        return false;
    };
    if end >= KMEM_OFFSET {
        return false;
    }
    crate::system::unwrap_system()
        .threads
        .running_thread
        .lock()
        .as_ref()
        .expect("A syscall was called without a running thread.")
        .page_manager
        .can_access_range(start, bytes, write)
}

fn is_range_readable<T>(start: *const T, count: usize) -> bool {
    can_access_range(start, count, false)
}

fn is_range_writeable<T>(start: *const T, count: usize) -> bool {
    can_access_range(start, count, true)
}

/// Construct null-terminated string from userspace pointer
///
/// # Safety
///
/// The returned reference is invalidated if the page(s) containing the string are mapped out of memory.
/// You must not hold any mutable references to any parts of the string
/// while it is in scope (as is required by Rust).
pub unsafe fn get_cstr_from_user_space(ptr: *const u8) -> Result<&'static str, CStrError> {
    let mut len = 0usize;
    if !is_range_readable(ptr, 1) {
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
        if end % PAGE_FRAME_SIZE == 0 && !is_range_readable(end as *const u8, 1) {
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
pub unsafe fn get_mut_slice_from_user_space<T>(
    ptr: *mut T,
    count: usize,
) -> Option<&'static mut [T]> {
    if !ptr.is_aligned() {
        return None;
    }
    if !is_range_writeable(ptr, count) {
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
pub unsafe fn get_slice_from_user_space<T>(ptr: *const T, count: usize) -> Option<&'static [T]> {
    if !ptr.is_aligned() {
        return None;
    }
    if !is_range_readable(ptr, count) {
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

/// Construct slice from a null-terminate userspace pointer.
/// The array pointed to by this pointer must end with a NULL.
/// If we do not find NULL after max_length, None will be returned.
///
/// # Safety
///
/// See [`get_slice_from_user_space`], except the pointer must be null-terminated.
/// No type-confusion constraints for safety, pointers must be checked again after being returned.
pub unsafe fn get_slice_from_null_terminated_user_space<T>(
    ptr: *const *const T,
    max_length: usize,
) -> Option<&'static [*const T]> {
    if !ptr.is_aligned() {
        return None;
    }

    let mut length = 0;

    while length < max_length && is_range_readable(ptr.add(length), size_of::<T>()) {
        if (*ptr.add(length)).is_null() {
            return Some(core::slice::from_raw_parts(ptr, length));
        }

        length += 1;
    }

    None
}
