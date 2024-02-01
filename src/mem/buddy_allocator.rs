// https://en.wikipedia.org/wiki/Buddy_memory_allocation
//
// We store each region's State in its first byte. This isn't a very smart way
// to do things with respect to alignment, so this could definitely be improved.

use crate::eprintln;
use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::size_of,
    ptr::NonNull,
};

/// BuddyAllocator is a simple implementation of a buddy allocator.
#[derive(Clone, Copy)]
pub struct BuddyAllocator {
    region: NonNull<[u8]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum State {
    // This region is unused, and can be allocated or split as necessary.
    Free,
    // This region has been handed out to a caller.
    Allocated,
    // This region has been split in half.
    Split,
}

impl BuddyAllocator {
    /// The minimum extra space within its region required by a buddy allocator
    /// for internal bookkeeping.
    pub const OVERHEAD: usize = size_of::<State>();

    /// Creates a new buddy allocator using the backing region.
    pub unsafe fn new(region: NonNull<[u8]>) -> Self {
        assert!(region.len() >= size_of::<State>());
        *region.as_ptr().cast::<State>() = State::Free;
        Self { region }
    }

    /// Returns whether all allocations within the buddy allocator have been
    /// freed.
    pub fn is_empty(&self) -> bool {
        // SAFETY: self.region must be at least big enough for the initial State
        // as per the assertion in new.
        unsafe { *self.region.cast::<State>().as_ptr() == State::Free }
    }

    // Prints log messages for leaks and returns whether there were any leaks.
    pub fn detect_leaks(&self) -> bool {
        unsafe fn detect_leaks(region: NonNull<[u8]>) -> bool {
            match *region.as_ptr().cast::<State>() {
                State::Free => false,
                State::Allocated => {
                    eprintln!(
                        "address within buddy allocator region {:?} leaked!",
                        region.as_ptr()
                    );
                    true
                }
                State::Split => {
                    let (left, right) = split_region(region);
                    let left_leaked = detect_leaks(left);
                    let right_leaked = detect_leaks(right);
                    left_leaked || right_leaked
                }
            }
        }

        // SAFETY: self.region belongs to this allocator so it should still be
        // valid to traverse.
        unsafe { detect_leaks(self.region) }
    }
}

// The length of the usable space within region (the space not already used for
// bookkeeping).
const fn usable_len(region: NonNull<[u8]>) -> usize {
    region.len() - size_of::<State>()
}

// Divides the given region in half.
unsafe fn split_region(region: NonNull<[u8]>) -> (NonNull<[u8]>, NonNull<[u8]>) {
    let size = usable_len(region) / 2;

    let left_start = region.as_ptr().cast::<u8>().add(size_of::<State>());
    let left = NonNull::slice_from_raw_parts(NonNull::new_unchecked(left_start), size);

    let right = NonNull::slice_from_raw_parts(NonNull::new_unchecked(left_start.add(size)), size);

    (left, right)
}

// Searches for a free subregion within the passed region whose size is as close
// as possible to but no smaller than target_size. If one is found, it will be
// marked as allocated before being returned.
unsafe fn find_free(region: NonNull<[u8]>, target_size: usize) -> Option<NonNull<[u8]>> {
    if usable_len(region) < target_size {
        return None;
    }

    match *region.as_ptr().cast::<State>() {
        State::Free => Some(split_free(region, target_size)),
        State::Allocated => None,
        State::Split => {
            let (left, right) = split_region(region);
            find_free(left, target_size).or_else(|| find_free(right, target_size))
        }
    }
}

// Splits the passed region repeatedly, until any futher splits would make the
// result smaller than target_size, then marks the leftmost subregion as
// allocated and returns it.
unsafe fn split_free(region: NonNull<[u8]>, target_size: usize) -> NonNull<[u8]> {
    let (left, right) = split_region(region);
    if usable_len(left) < target_size {
        // If dividing would make the region too small, return it.
        *region.as_ptr().cast::<State>() = State::Allocated;
        return NonNull::slice_from_raw_parts(
            NonNull::new_unchecked(region.as_ptr().cast::<u8>().add(1)),
            usable_len(region),
        );
    }

    *region.as_ptr().cast::<State>() = State::Split;
    *right.as_ptr().cast::<State>() = State::Free;
    split_free(left, target_size)
}

// Recurses within region until the subregion containing ptr is found. Returns
// whether region itself was marked as free (either because it directly
// contained ptr, or because enough buddies were also free that coalescing
// allowed it to be freed).
unsafe fn deallocate(region: NonNull<[u8]>, ptr: NonNull<u8>) -> bool {
    let state_ptr = region.as_ptr().cast::<State>();
    match *state_ptr {
        // We should never be searching for the region of a ptr in a region
        // that's already marked as free.
        State::Free => panic!("internal inconsistency detected in buddy allocator"),

        // We've found the region, mark it as free and return to indicate
        // that we freed something.
        State::Allocated => {
            *state_ptr = State::Free;
            true
        }

        // The ptr belongs to one of the two sides of the split.
        State::Split => {
            let (left, right) = split_region(region);

            // Determine target to recurse within.
            let (target, buddy) = if ptr.as_ptr() < right.as_ptr().cast::<u8>() {
                (left, right)
            } else {
                (right, left)
            };

            // If the target was marked as free and our buddy is also
            // already free, collapse the outer region and mark it as freed
            // too.
            if deallocate(target, ptr) && *buddy.as_ptr().cast::<State>() == State::Free {
                *state_ptr = State::Free;
                return true;
            }

            false
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
        match unsafe { find_free(self.region, target_size) } {
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
        deallocate(self.region, ptr);
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
