# System calls - `SYS_GETCWD` (#183)

```rust
fn getcwd(buf: *mut u8, size: usize) -> isize;
```

Get the current working directory for the running process. The CWD is written
into `buf` with a terminating null byte. At most `size` bytes are written, including
the null terminator. Returns 0 on success.

### Errors

- `EFAULT` - `buf` is an invalid pointer
- `ERANGE` - `size` is not large enough — in this case `buf` is unmodified
