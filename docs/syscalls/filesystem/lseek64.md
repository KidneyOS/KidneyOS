# System calls - `SYS_LSEEK64` (#140)

```rust
const SEEK_SET: i32 = 0;
const SEEK_CUR: i32 = 1;
const SEEK_END: i32 = 2;

fn lseek64(fd: i32, offset: *mut i64, whence: i32) -> isize;
```

Seeks the file descriptor `fd` to the offset indicated by `*offset`, from the reference point `whence`.
Returns 0 on success, and sets `*offset` to the new file offset, relative to the start of the file.
`whence` must be one of `SEEK_SET` (`*offset` is relative to the start of the file),
`SEEK_CUR` (`*offset` is relative to the current file position), or `SEEK_END` (`*offset` is relative
to the end of the file). If `fd` is a directory, `whence` must be `SEEK_SET`, and `*offset` must be an
offset previously returned by `lseek64` or `getdents`.

### Errors

- `EIO` - an I/O error occurred while seeking in the file
- `EBADF` - `fd` is invalid
- `EFAULT` - `offset` is an invalid pointer
- `EINVAL` - seek offset is invalid (e.g. positive offset from end of file)
- `ESPIPE` - `fd` is not a seekable file
- `EISDIR` - invalid seek for a directory
