// https://en.wikipedia.org/wiki/Buddy_memory_allocation
//
// We store each region's State in its first byte. This isn't a very smart way
// to do things with respect to alignment, so this could definitely be improved.

use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::size_of,
    ptr::NonNull,
};

#[derive(Clone, Copy)]
pub struct BuddyAllocator {
    region: NonNull<[u8]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum State {
    Free,
    Allocated,
    Split,
}

impl BuddyAllocator {
    pub const OVERHEAD: usize = size_of::<State>();

    pub unsafe fn new(region: NonNull<[u8]>) -> Self {
        assert!(region.len() >= size_of::<State>());
        *region.as_ptr().cast::<State>() = State::Free;
        Self { region }
    }

    pub fn is_empty(&self) -> bool {
        // SAFETY: self.region must be at least big enough for the initial State
        // as per the assertion in new.
        unsafe { *self.region.cast::<State>().as_ptr() == State::Free }
    }
}

impl BuddyAllocator {
    unsafe fn split_region(region: NonNull<[u8]>) -> (NonNull<[u8]>, NonNull<[u8]>) {
        let size = (region.len() - size_of::<State>()) / 2;

        let left_start = region.as_ptr().cast::<u8>().add(size_of::<State>());
        let left = NonNull::slice_from_raw_parts(NonNull::new_unchecked(left_start), size);

        let right =
            NonNull::slice_from_raw_parts(NonNull::new_unchecked(left_start.add(size)), size);

        (left, right)
    }

    unsafe fn find_free(region: NonNull<[u8]>, target_size: usize) -> Option<NonNull<[u8]>> {
        if region.len() - size_of::<State>() < target_size {
            return None;
        }

        match *region.as_ptr().cast::<State>() {
            State::Free => Some(Self::divide_free(region, target_size)),
            State::Allocated => None,
            State::Split => {
                let (left, right) = Self::split_region(region);
                Self::find_free(left, target_size).or_else(|| Self::find_free(right, target_size))
            }
        }
    }

    unsafe fn divide_free(region: NonNull<[u8]>, target_size: usize) -> NonNull<[u8]> {
        let (left, right) = Self::split_region(region);
        if left.len() - size_of::<State>() < target_size {
            // If dividing would make the region too small, return this.
            *region.as_ptr().cast::<State>() = State::Allocated;
            return NonNull::slice_from_raw_parts(
                NonNull::new_unchecked(region.as_ptr().cast::<u8>().add(1)),
                region.len() - 1,
            );
        }

        *region.as_ptr().cast::<State>() = State::Split;
        *right.as_ptr().cast::<State>() = State::Free;
        Self::divide_free(left, target_size)
    }

    unsafe fn deallocate_rec(region: NonNull<[u8]>, ptr: NonNull<u8>) -> bool {
        match *region.as_ptr().cast::<State>() {
            // We should never be searching for the region of a ptr in a region
            // that's already marked as free.
            State::Free => panic!("internal inconsistency detected in buddy allocator"),

            // We've found the region, mark it as free and return to indicate
            // that we freed something.
            State::Allocated => {
                *region.as_ptr().cast::<State>() = State::Free;
                true
            }

            // The ptr belongs to one of the two sides of the split.
            State::Split => {
                let (left, right) = Self::split_region(region);

                // Determine target to recurse within.
                let (target, buddy) = if ptr.as_ptr() < right.as_ptr().cast::<u8>() {
                    (left, right)
                } else {
                    (right, left)
                };

                // If the target was marked as free and our buddy is also
                // already free, collapse the outer region and mark it as freed
                // too.
                if Self::deallocate_rec(target, ptr)
                    && *buddy.as_ptr().cast::<State>() == State::Free
                {
                    *region.as_ptr().cast::<State>() = State::Free;
                    return true;
                }

                false
            }
        }
    }
}

// SAFETY:
//
// - Memory blocks retain their validity as required so long as the buddy
//   allocator's region as passed to new does.
// - The Buddy Allocator struct itself contains no mutable state so copying and
//   cloning will not cause issues.
// - It only implements deallocate, and will handle this correctly for any
//   allocated pointer.
unsafe impl Allocator for BuddyAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Instead of doing complicated things with alignment, we just allocate
        // a region large enough that we're guaranteed to be able to find a
        // large enough sub-section of that region with the correct alignment.
        let target_size = layout.size() + layout.align() - 1;

        // SAFETY: self.region belongs to this allocator, so find_free will
        // behave correctly.
        match unsafe { Self::find_free(self.region, target_size) } {
            Some(region) => Ok(NonNull::slice_from_raw_parts(
                // SAFETY: We just got got the region as a NonNull from
                // find_free so we know it's valid, and we requested that its
                // size be large enough that all the math below will remain in
                // bounds.
                unsafe {
                    NonNull::new_unchecked(
                        (region.as_ptr().cast::<u8>() as usize).next_multiple_of(layout.align())
                            as *mut u8,
                    )
                },
                layout.size(),
            )),
            None => Err(AllocError),
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        Self::deallocate_rec(self.region, ptr);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;

    use crate::constants::KB;
    use std::{alloc::Global, collections::BTreeMap, error::Error};

    #[test]
    fn buddy_allocator_simple() -> Result<(), Box<dyn Error>> {
        let region = Global.allocate(Layout::from_size_align(2 * KB, 2)?)?;
        let alloc = unsafe { BuddyAllocator::new(region) };

        assert!(alloc.is_empty());

        let mut v = Vec::new_in(&alloc);
        v.push(35);

        assert!(!alloc.is_empty());

        v.push(351);
        v.push(881);

        let mut m = BTreeMap::new_in(&alloc);
        m.insert(35, 15);
        m.insert(816, 3122);
        assert_eq!(m.get(&35), Some(&15));
        assert_eq!(m.remove(&1313), None);

        assert_eq!(v.pop(), Some(881));
        assert_eq!(v.pop(), Some(351));
        assert_eq!(v.pop(), Some(35));
        assert!(v.is_empty());

        assert_eq!(m.remove(&816), Some(3122));

        v.reserve(200);
        drop(v);

        assert_eq!(m.remove(&35), Some(15));

        assert!(!alloc.is_empty());

        drop(m);

        assert!(alloc.is_empty());

        Ok(())
    }
}
