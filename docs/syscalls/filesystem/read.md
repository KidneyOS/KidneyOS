# System calls - `SYS_READ` (#3)

```rust
fn read(fd: i32, buf: *mut c_void, count: usize) -> isize;
```

Reads `count` bytes from the file descriptor `fd` into `buf`. Returns the number of bytes read.
A return value of 0 indicates that the end of the file was reached.
A positive value less than count may be returned even if there is more data available in the file.

### Errors

- `EIO` - an I/O error occurred while reading from the file
- `EBADF` - `fd` is invalid or cannot be read from
- `EFAULT` - `buf` is an invalid pointer
- `EISDIR` - `fd` is a directory
