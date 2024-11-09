use crate::mem::frame_allocator::CoreMapEntry;
use core::alloc::AllocError;
use core::ops::Range;

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

        if core_map[i].allocated() {
            free_frames_found += 1;

            for j in 1..frames_requested {
                if core_map[i + j].allocated() {
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

        if core_map[i].allocated() {
            free_frames_found += 1;

            for j in 1..frames_requested {
                if core_map[i + j].allocated() {
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
        if core_map[i].allocated() {
            let start_index = i;
            let mut chunk_size = 0;

            while i < total_frames {
                if core_map[i].allocated() {
                    break;
                }

                chunk_size += 1;
                i += 1;
            }

            if chunk_size >= frames_requested
                && chunk_size - frames_requested < best_chunk_size_so_far
            {
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
