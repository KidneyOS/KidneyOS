use alloc::{vec, vec::Vec};

/// Keeps track of a set of slots, some of which can be free and some of which can be allocated.
///
/// This was designed for keeping track of free FAT clusters, but it could be used for something else.
///
/// Fast operations are:
///   - Find a free slot, and mark it as allocated
///   - Mark a (previously-allocated) slot as free.
///
/// Uses roughly 1.1 bits per available slot.
#[derive(Debug, Clone)]
pub struct FreeSet {
    bitmap: Vec<u64>,
    queue: Vec<u32>,
}

impl FreeSet {
    /// Create a new FreeSet with all slots allocated.
    pub fn new_all_allocated(count: u32) -> Self {
        let group_count = count.div_ceil(64) as usize;
        Self {
            bitmap: vec![0; group_count],
            queue: Vec::with_capacity(group_count),
        }
    }

    /// Allocate a slot.
    ///
    /// Returns `None` if no slots are available.
    ///
    /// This takes *O(1)* time.
    pub fn allocate(&mut self) -> Option<u32> {
        let group_index = self.queue.pop()?;
        let group = &mut self.bitmap[group_index as usize];
        debug_assert_ne!(*group, 0, "FreeSet consistency error");
        let index_in_group = group.trailing_zeros();
        // clear bit
        *group &= !(1 << index_in_group);
        if *group != 0 {
            // add back to queue
            self.queue.push(group_index);
        }
        Some(group_index * 64 + index_in_group)
    }

    /// Free a slot.
    ///
    /// In debug mode, this panics if the slot was already free.
    /// Otherwise, nothing happens.
    ///
    /// This takes *O(1)* time.
    pub fn free(&mut self, index: u32) {
        let group_index = index / 64;
        let index_in_group = index % 64;
        let group = &mut self.bitmap[group_index as usize];
        let add = *group == 0;
        debug_assert!(
            (*group & (1 << index_in_group)) == 0,
            "FreeSet::free called on already free slot"
        );
        // set bit
        *group |= 1 << index_in_group;
        if add {
            self.queue.push(group_index);
        }
    }
}
