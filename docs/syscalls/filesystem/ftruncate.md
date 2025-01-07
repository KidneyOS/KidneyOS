# System calls - `SYS_FTRUNCATE` (#93)

```rust
fn ftruncate(fd: i32, size_lo: usize, size_hi: usize) -> isize;
```

Set the size of the file `fd` to `size_lo | size_hi << 32`. If this is shorter
than the previous file size, the data at the end of the file is deleted.
If this is longer than the previous file size, the extra space is filled with zero bytes.
Returns 0 on success.

### Errors

- `EIO` - an I/O error occurred while changing the file size
- `EBADF` - `fd` is invalid or cannot be truncated
- `EISDIR` - `fd` is a directory
- `ENOSPC` - not enough space on device to extend file
- `EROFS` - `fd` is in a read-only file system

