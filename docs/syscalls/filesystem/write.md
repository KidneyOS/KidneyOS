# System calls - `SYS_WRITE` (#4)

```rust
fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
```

Write `count` bytes from `buf` into the file descriptor `fd`. Returns the number of bytes written.
This may be less than `count`, even if no error occurred.
If `count` is not zero, `write` will never return zero.

### Errors

- `EIO` - an I/O error occurred while reading from the file
- `EBADF` - `fd` is invalid or cannot be written to
- `EFAULT` - `buf` is an invalid pointer
- `EISDIR` - `fd` is a directory
- `ENOSPC` - not enough space on device to write data
- `EROFS` - `fd` is in a read-only file system

