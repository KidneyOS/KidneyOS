//! Implementation of some common frame placement policies.

use super::CoreMapEntry;
use core::alloc::AllocError;
use core::ops::Range;

/// A placement algorithm for allocating frames.
pub trait PlacementAlgorithm: Default {
    /// Returns [`Ok`] containing a range indicating the frame numbers to be allocated.
    ///
    /// # Errors
    ///
    /// If a sufficiently large range of free frames cannot be found, or if the frames cannot
    /// be placed for any other reason, the function returns an error.
    fn place(
        &mut self,
        core_map: &[CoreMapEntry],
        frames_requested: usize,
    ) -> Result<Range<usize>, AllocError>;
}

#[derive(Default)]
pub struct NextFit {
    /// The next frame number to start searching for free frames.
    position: usize,
}

// There is no internal data for these two algorithms. Declare them as zero-sized types.
#[derive(Default)]
pub struct FirstFit;
#[derive(Default)]
pub struct BestFit;

impl PlacementAlgorithm for NextFit {
    fn place(
        &mut self,
        core_map: &[CoreMapEntry],
        frames_requested: usize,
    ) -> Result<Range<usize>, AllocError> {
        let total_frames = core_map.len();

        let mut block_start_ind = self.position;
        let mut wrapped_around = false;

        while !(wrapped_around && block_start_ind >= self.position) {
            if block_start_ind + frames_requested > total_frames {
                block_start_ind = 0;
                // If we've already wrapped around once, don't do it again.
                // Without this check, we will run into an infinite loop if we request a large
                // number of frames, since `block_start_ind` may never be allowed to reach
                // `self.position` again.
                if wrapped_around {
                    break;
                }
                wrapped_around = true;
                continue;
            }

            // Count the number of free blocks starting from block_start_ind, up to the number
            // requested
            let mut block_size = 0;
            while block_size < frames_requested
                && !core_map[block_start_ind + block_size].allocated()
            {
                block_size += 1;
            }

            // Have we found a block large enough?
            if block_size == frames_requested {
                self.position = (block_start_ind + block_size) % total_frames;
                return Ok(block_start_ind..(block_start_ind + block_size));
            }
            // Previous block too small, keep searching starting from one past the allocated frame
            block_start_ind += block_size + 1;
        }

        Err(AllocError)
    }
}

impl PlacementAlgorithm for FirstFit {
    fn place(
        &mut self,
        core_map: &[CoreMapEntry],
        frames_requested: usize,
    ) -> Result<Range<usize>, AllocError> {
        let total_frames = core_map.len();

        for i in 0..=total_frames - frames_requested {
            let mut free_frames_found = 0;

            if !core_map[i].allocated() {
                free_frames_found += 1;

                for j in 1..frames_requested {
                    if !core_map[i + j].allocated() {
                        free_frames_found += 1;
                    }
                }
            }

            if free_frames_found == frames_requested {
                return Ok(i..i + frames_requested);
            }
        }

        Err(AllocError)
    }
}

impl PlacementAlgorithm for BestFit {
    fn place(
        &mut self,
        core_map: &[CoreMapEntry],
        frames_requested: usize,
    ) -> Result<Range<usize>, AllocError> {
        let total_frames = core_map.len();

        let mut best_start_index_so_far = total_frames;
        let mut best_chunk_size_so_far = total_frames + 1;
        let mut i = 0;

        while i < total_frames {
            if !core_map[i].allocated() {
                let start_index = i;
                let mut chunk_size = 0;

                while i < total_frames {
                    if core_map[i].allocated() {
                        break;
                    }

                    chunk_size += 1;
                    i += 1;
                }
                if chunk_size >= frames_requested && chunk_size < best_chunk_size_so_far {
                    best_chunk_size_so_far = chunk_size;
                    best_start_index_so_far = start_index;
                }
            } else {
                i += 1;
            }
        }

        if best_start_index_so_far == total_frames {
            return Err(AllocError);
        }

        Ok(best_start_index_so_far..best_start_index_so_far + frames_requested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem::frame_allocator::CoreMapEntry;

    /// Fills the coremap entries in `range` to indicate they are allocated.
    fn fill_coremap_range(core_map: &mut [CoreMapEntry], range: &Range<usize>) {
        for i in range.clone() {
            assert!(!core_map[i].allocated());
            core_map[i] = core_map[i].with_next(true).with_allocated(true);
        }
    }

    #[test]
    fn test_next_fit() {
        let mut core_map = [CoreMapEntry::default(); 16];
        fill_coremap_range(&mut core_map, &(1..4));
        fill_coremap_range(&mut core_map, &(8..12));
        fill_coremap_range(&mut core_map, &(14..16));

        // Frames left are 0, 4-7, 12-13 (inclusive)

        let mut algorithm: NextFit = Default::default();
        assert_eq!(algorithm.place(&core_map, 4), Ok(4..8));
        fill_coremap_range(&mut core_map, &(4..8));

        // Next allocation should start from position 8
        assert_eq!(algorithm.place(&core_map, 1), Ok(12..13));
        fill_coremap_range(&mut core_map, &(12..13));

        assert_eq!(algorithm.place(&core_map, 2), Err(AllocError));
    }

    #[test]
    fn test_next_fit_wrap_around() {
        let mut core_map = [CoreMapEntry::default(); 16];
        let mut algorithm = NextFit { position: 8 };
        fill_coremap_range(&mut core_map, &(0..1));
        assert_eq!(algorithm.place(&core_map, 16), Err(AllocError));
        assert_eq!(algorithm.place(&core_map, 15), Ok(1..16));
    }

    #[test]
    fn test_first_fit() {
        let mut core_map = [CoreMapEntry::default(); 16];
        fill_coremap_range(&mut core_map, &(2..4));
        fill_coremap_range(&mut core_map, &(8..13));
        fill_coremap_range(&mut core_map, &(15..16));

        // Frames left are 0-2, 4-7, 13-14 (inclusive)

        let mut algorithm = FirstFit;
        assert_eq!(algorithm.place(&core_map, 4), Ok(4..8));
        fill_coremap_range(&mut core_map, &(4..8));

        // If we want 2 frames, the algo should pick first fit, i.e. 0-1
        assert_eq!(algorithm.place(&core_map, 2), Ok(0..2));
        fill_coremap_range(&mut core_map, &(0..2));

        assert_eq!(algorithm.place(&core_map, 3), Err(AllocError));
    }

    #[test]
    fn test_best_fit() {
        let mut core_map = [CoreMapEntry::default(); 16];
        fill_coremap_range(&mut core_map, &(3..4));
        fill_coremap_range(&mut core_map, &(8..13));
        fill_coremap_range(&mut core_map, &(15..16));

        // Frames left are 0-2, 4-7, 13-14 (inclusive)

        let mut algorithm = BestFit;
        assert_eq!(algorithm.place(&core_map, 4), Ok(4..8));
        fill_coremap_range(&mut core_map, &(4..8));

        // If we want 2 frames, the algo should pick 13-14
        assert_eq!(algorithm.place(&core_map, 2), Ok(13..15));
        fill_coremap_range(&mut core_map, &(13..15));

        assert_eq!(algorithm.place(&core_map, 4), Err(AllocError));
    }
}
