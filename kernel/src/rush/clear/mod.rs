use kidneyos_shared::video_memory::VIDEO_MEMORY_WRITER;

/// Clear the screen.
pub fn clear() {
    unsafe { VIDEO_MEMORY_WRITER.clear_screen() };
}
