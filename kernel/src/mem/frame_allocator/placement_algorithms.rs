use crate::mem::frame_allocator::CoreMapEntry;
use core::alloc::AllocError;
use core::ops::Range;

/// This file contains the implementation for some common frame placement policies
///
/// Any implementation needs to follow the following function signature (as defined
/// in frame_allocator.rs)
///
/// fn(core_map: &[CoreMapEntry], frames_requested: usize, _position: usize) -> Result<Range<usize>, AllocError>,
///
/// where
///
/// core_map: reference to a slice of all CoreMap Entries
/// frames_requested: the number of frames requested to be allocated
/// _position: the current position relative to the last allocation. This parameter may not be
/// needed for certain placement policies
///
/// Any implementation should return a range of indices indicating the frames to be allocated
/// on success or Err(AllocError) if there is insufficient space.

pub fn next_fit(
    core_map: &[CoreMapEntry],
    frames_requested: usize,
    _position: usize,
) -> Result<Range<usize>, AllocError> {
    let total_frames = core_map.len();

    for index in _position.._position + total_frames {
        let i = index % total_frames;

        if i + frames_requested > total_frames {
            continue;
        }

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

#[allow(dead_code)]
pub fn first_fit(
    core_map: &[CoreMapEntry],
    frames_requested: usize,
    _position: usize,
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

#[allow(dead_code)]
pub fn best_fit(
    core_map: &[CoreMapEntry],
    frames_requested: usize,
    _position: usize,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem::frame_allocator::CoreMapEntry;

    fn fill_in_coremap(core_map: &mut [CoreMapEntry], indices: Range<usize>) {
        for i in indices {
            assert!(!core_map[i].allocated());
            core_map[i] = core_map[i].with_next(true).with_allocated(true);
        }
    }

    #[test]
    fn test_placement_algorithms() {
        let mut core_map = [CoreMapEntry::default(); 30];

        let next_fit_range = next_fit(&core_map, 14, 11).unwrap();
        assert_eq!(next_fit_range.start, 11);
        assert_eq!(next_fit_range.end, 25);

        fill_in_coremap(&mut core_map, next_fit_range.clone());

        let first_fit_range = first_fit(&core_map, 3, 13).unwrap();
        assert_eq!(first_fit_range.start, 0);
        assert_eq!(first_fit_range.end, 3);

        fill_in_coremap(&mut core_map, first_fit_range.clone());

        let best_fit_range = best_fit(&core_map, 3, 29).unwrap();
        assert_eq!(best_fit_range.start, 25);
        assert_eq!(best_fit_range.end, 28);

        fill_in_coremap(&mut core_map, best_fit_range.clone());

        let no_room = first_fit(&core_map, 10, 30);
        assert!(no_room.is_err());
    }
}
